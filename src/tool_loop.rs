//! @efficiency-role: domain-logic
//! Tool Loop — continuous execution loop using native tool calling.

use crate::auto_compact::{
    apply_compact, apply_compact_with_summarizer, CompactTracker, DEFAULT_COMPACT_BUFFER_TOKENS,
    DEFAULT_CONTEXT_WINDOW_TOKENS,
};
use crate::tool_calling::build_tool_definitions;
use crate::tool_result_storage::{apply_tool_result_budget, DEFAULT_MAX_RESULT_SIZE_CHARS};
use crate::ui_state::{
    get_total_intel_failures, increment_intel_failure_count, reset_intel_failure_counts,
};
use crate::*;
use futures::stream::StreamExt;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::future::Future;
use std::time::{Duration, Instant};

// Legacy constants absorbed into StopPolicy (StageBudget::default).
// Kept briefly for reference; remove after validation.

async fn await_with_busy_input<T, F>(
    tui: &mut crate::ui_terminal::TerminalUI,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
           result = &mut future => return result,
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                let _ = tui.pump_ui();
                if let Ok(Some(queued)) = tui.poll_busy_submission() {
                    tui.enqueue_submission(queued);
                }
            }
        }
    }
}

fn append_streaming_tool_call_delta(
    parts: &mut BTreeMap<usize, StreamingToolCallPart>,
    delta: &serde_json::Value,
) {
    let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) else {
        return;
    };
    for (fallback_index, call) in calls.iter().enumerate() {
        let index = call
            .get("index")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(fallback_index);
        let part = parts.entry(index).or_default();
        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
            part.id = Some(id.to_string());
        }
        if let Some(call_type) = call.get("type").and_then(|v| v.as_str()) {
            part.call_type = Some(call_type.to_string());
        }
        if let Some(function) = call.get("function") {
            if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                part.name = Some(name.to_string());
            }
            if let Some(arguments) = function.get("arguments").and_then(|v| v.as_str()) {
                part.arguments.push_str(arguments);
            }
        }
    }
}

fn finish_streaming_tool_calls(parts: BTreeMap<usize, StreamingToolCallPart>) -> Vec<ToolCall> {
    parts
        .into_iter()
        .filter_map(|(index, part)| {
            let name = part.name?;
            Some(ToolCall {
                id: part.id.unwrap_or_else(|| format!("call_{index}")),
                call_type: part.call_type.unwrap_or_else(|| "function".to_string()),
                function: ToolFunctionCall {
                    name,
                    arguments: part.arguments,
                },
            })
        })
        .collect()
}

async fn request_tool_loop_model_turn_streaming(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    mut req: ChatCompletionRequest,
    timeout_s: u64,
    session: &SessionPaths,
) -> Result<ToolLoopModelTurn> {
    req.stream = true;
    req.reasoning_format = Some("auto".to_string());

    let response = client
        .post(chat_url.clone())
        .json(&req)
        .timeout(Duration::from_secs(timeout_s))
        .send()
        .await
        .context("Tool loop streaming request failed")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API error {}: {}", status, body);
    }

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut content = String::new();
    let mut tool_call_parts: BTreeMap<usize, StreamingToolCallPart> = BTreeMap::new();
    let mut thinking_started = false;
    let mut content_started = false;
    let mut in_think_block = false;
    let mut pending_think_tag = String::new();
    let mut thinking_accumulated = String::new();

    loop {
        let chunk_result_opt = tokio::select! {
            chunk = byte_stream.next() => chunk,
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                let _ = tui.pump_ui();
                if let Ok(Some(queued)) = tui.poll_busy_submission() {
                    tui.enqueue_submission(queued);
                }
                continue;
            }
        };

        let Some(chunk_result) = chunk_result_opt else {
            break;
        };
        let chunk_bytes = match chunk_result {
            Ok(bytes) => bytes,
            Err(error) => {
                append_trace_log_line(&format!("[TOOL_LOOP_STREAM_ERROR] {}", error));
                break;
            }
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk_bytes));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer.drain(..pos + 1).collect::<String>();
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data.is_empty() || data == "[DONE]" {
                continue;
            }

            let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) else {
                continue;
            };
            let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) else {
                continue;
            };
            for choice in choices {
                let Some(delta) = choice.get("delta") else {
                    continue;
                };

                let reasoning = delta
                    .get("reasoning_content")
                    .or_else(|| delta.get("reasoning"))
                    .or_else(|| delta.get("thought"))
                    .and_then(|v| v.as_str())
                    .map(crate::claude_ui::strip_thinking_tags_preserve_spacing)
                    .unwrap_or_default();
                if !reasoning.is_empty() {
                    if !thinking_started {
                        thinking_started = true;
                        tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingStarted);
                        let _ = tui.pump_ui();
                    }
                    tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingDelta(
                        reasoning.clone(),
                    ));
                    thinking_accumulated.push_str(&reasoning);
                    let _ = tui.pump_ui();
                }

                if let Some(raw_content) = delta.get("content").and_then(|v| v.as_str()) {
                    let (assistant_delta, thinking_delta) =
                        crate::orchestration_helpers::process_stream_content_chunk(
                            raw_content,
                            &mut in_think_block,
                            &mut pending_think_tag,
                        );
                    let thinking_delta =
                        crate::claude_ui::strip_thinking_tags_preserve_spacing(&thinking_delta);
                    if !thinking_delta.is_empty() {
                        if !thinking_started {
                            thinking_started = true;
                            tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingStarted);
                            let _ = tui.pump_ui();
                        }
                        tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingDelta(
                            thinking_delta.clone(),
                        ));
                        thinking_accumulated.push_str(&thinking_delta);
                        let _ = tui.pump_ui();
                    }

                    if !assistant_delta.is_empty() {
                        if thinking_started && !in_think_block {
                            thinking_started = false;
                            tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingFinished);
                            let _ = save_thinking_display(session, &thinking_accumulated);
                            thinking_accumulated.clear();
                            let _ = tui.pump_ui();
                        }
                        content.push_str(&assistant_delta);
                        if !content_started {
                            content_started = true;
                        }
                        tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantContentDelta(
                            assistant_delta,
                        ));
                        let _ = tui.pump_ui();
                    }
                }

                append_streaming_tool_call_delta(&mut tool_call_parts, delta);
            }
        }
    }

    if thinking_started {
        tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingFinished);
        let _ = save_thinking_display(session, &thinking_accumulated);
        thinking_accumulated.clear();
        let _ = tui.pump_ui();
    }
    if content_started {
        tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantFinished);
        let _ = tui.pump_ui();
    }

    Ok(ToolLoopModelTurn {
        content: content.trim().to_string(),
        tool_calls: finish_streaming_tool_calls(tool_call_parts),
    })
}

pub(crate) struct ToolLoopResult {
    pub(crate) final_answer: String,
    pub(crate) iterations: usize,
    pub(crate) tool_calls_made: usize,
    pub(crate) stopped_by_max: bool,
    pub(crate) stop_outcome: Option<StopOutcome>,
    pub(crate) total_elapsed_s: f64,
    pub(crate) timeout_reason: Option<String>,
}

struct ToolLoopModelTurn {
    content: String,
    tool_calls: Vec<ToolCall>,
}

#[derive(Default)]
struct StreamingToolCallPart {
    id: Option<String>,
    call_type: Option<String>,
    name: Option<String>,
    arguments: String,
}

fn is_tool_call_markup(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    lower.contains("<tool_call>")
        || lower.contains("</tool_call>")
        || (lower.contains("\"name\"")
            && lower.contains("\"arguments\"")
            && (lower.contains("\"name\":\"shell\"")
                || lower.contains("\"name\":\"read\"")
                || lower.contains("\"name\":\"search\"")
                || lower.contains("\"name\":\"respond\"")
                || lower.contains("\"name\":\"update_todo_list\"")
                || lower.contains("\"name\": \"shell\"")
                || lower.contains("\"name\": \"read\"")
                || lower.contains("\"name\": \"search\"")
                || lower.contains("\"name\": \"respond\"")
                || lower.contains("\"name\": \"update_todo_list\"")))
}

fn normalize_final_answer_candidate(text: &str) -> String {
    crate::text_utils::strip_thinking_blocks(text)
        .trim()
        .to_string()
}

fn final_answer_needs_retry(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.is_empty() || is_tool_call_markup(trimmed) || is_intent_only_response(trimmed)
}

fn build_fallback_from_recent_tool_evidence(messages: &[ChatMessage]) -> String {
    let mut facts = Vec::new();
    for msg in messages.iter().rev() {
        if msg.role != "tool" {
            continue;
        }
        let line = msg
            .content
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string());
        if let Some(first_line) = line {
            facts.push(first_line);
            if facts.len() >= 10 {
                break;
            }
        }
    }
    facts.reverse();
    if facts.is_empty() {
        "I couldn't produce a reliable final summary from the tool loop; please retry with a more specific prompt.".to_string()
    } else if facts.len() == 1 {
        format!(
            "Based on the evidence gathered:\n{}\n\n(This is the best answer I could extract. Consider rephrasing your request.)",
            facts[0]
        )
    } else {
        format!(
            "Based on the evidence gathered:\n- {}",
            facts.join("\n- ")
        )
    }
}

/// Build a clean finalization context that discards tool-call history and
/// presents only the user's request + compact evidence summary. Small models
/// get stuck in tool-calling mode when conversation history is saturated with
/// tool calls; a fresh context breaks the loop.
async fn request_final_answer_from_evidence(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    original_user_request: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
) -> Result<String> {
    // Collect all tool results as compact evidence
    let mut evidence_lines: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for msg in messages.iter().rev() {
        if msg.role != "tool" {
            continue;
        }
        let content = msg.content.trim();
        if content.is_empty() || !seen.insert(content.to_string()) {
            continue;
        }
        evidence_lines.push(content.to_string());
    }
    evidence_lines.reverse();

    let evidence_block = if evidence_lines.is_empty() {
        "(no tool results)".to_string()
    } else if evidence_lines.len() <= 3 {
        evidence_lines.join("\n")
    } else {
        // Truncate to avoid blowing context; first N entries are usually most
        // relevant since they're the initial findings.
        let display_count = evidence_lines.len().min(20);
        let truncated: Vec<_> = evidence_lines.iter().take(display_count).cloned().collect();
        truncated.join("\n")
    };

    let clean_messages = vec![
        ChatMessage::simple(
            "user",
            &format!(
                "{}\n\n--- Evidence gathered so far ---\n{}\n--- End evidence ---\n\nAnswer concisely using only the evidence above. Do not call tools.",
                original_user_request,
                evidence_block
            ),
        ),
    ];

    let req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: clean_messages,
        temperature: 0.2,
        top_p: 1.0,
        stream: false,
        max_tokens: max_tokens.min(1024),
        n_probs: None,
        repeat_penalty: None,
        reasoning_format: Some("none".to_string()),
        grammar: None,
        tools: None,
    };
    let resp = await_with_busy_input(
        tui,
        crate::ui_chat::chat_once_with_timeout(client, chat_url, &req, 60),
    )
    .await?;
    Ok(normalize_final_answer_candidate(
        &resp
            .choices
            .first()
            .map(|c| c.message.content.clone().unwrap_or_default())
            .unwrap_or_default(),
    ))
}

async fn request_final_answer_without_tools(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
    force_plain_text: bool,
) -> Result<String> {
    let mut req_messages = messages.to_vec();
    if force_plain_text {
        req_messages.push(ChatMessage::simple(
            "user",
            "Return plain terminal text only. Do not emit XML/JSON tool calls or function-call markup.",
        ));
    }
    let req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: req_messages,
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: max_tokens.min(1024),
        n_probs: None,
        repeat_penalty: None,
        reasoning_format: Some("none".to_string()),
        grammar: None,
        tools: None,
    };
    let resp = await_with_busy_input(
        tui,
        crate::ui_chat::chat_once_with_timeout(client, chat_url, &req, 60),
    )
    .await?;
    Ok(normalize_final_answer_candidate(
        &resp
            .choices
            .first()
            .map(|c| c.message.content.clone().unwrap_or_default())
            .unwrap_or_default(),
    ))
}

fn tool_signal(tc: &ToolCall) -> String {
    let fn_name = tc.function.name.as_str();
    let parsed: serde_json::Value =
        serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);
    let key = match fn_name {
        "shell" => parsed
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string(),
        "read" => parsed
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string(),
        "search" => {
            let pat = parsed
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let path = parsed
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            format!("{pat}|{path}")
        }
        "tool_search" => {
            let query = parsed
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            format!("query:{}", query)
        }
        "respond" => "respond".to_string(),
        other => format!("{other}:{}", tc.function.arguments),
    };
    if fn_name == "shell" {
        format!("{fn_name}:{}", normalize_shell_signal(&key))
    } else {
        format!("{fn_name}:{key}")
    }
}

fn normalize_shell_signal(cmd: &str) -> String {
    // Collapse highly variable identifiers (timestamps, session ids) so repeated
    // directory-probing loops are detected as the same strategy.
    let mut out = String::with_capacity(cmd.len());
    let mut prev_was_digit = false;
    for ch in cmd.chars() {
        if ch.is_ascii_digit() {
            if !prev_was_digit {
                out.push('#');
                prev_was_digit = true;
            }
            continue;
        }
        prev_was_digit = false;
        out.push(ch);
    }
    out.replace("s_#_#", "s_SESSION")
}

pub(crate) async fn run_tool_loop(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    system_prompt: &str,
    user_message: &str,
    workdir: &PathBuf,
    sess: &SessionPaths,
    temperature: f64,
    max_tokens: u32,
    tui: &mut crate::ui_terminal::TerminalUI,
    summarizer_cfg: Option<&Profile>,
) -> Result<ToolLoopResult> {
    let budget = StageBudget::default();
    let total_timeout = Duration::from_secs(30 * 60); // 30 minutes
    let loop_start = Instant::now();
    let original_user_request = user_message.to_string();
    trace(
        args,
        &format!(
            "tool_loop: starting max_iterations={} stagnation_threshold={} timeout={}m",
            budget.max_iterations, budget.max_stagnation_cycles, 30
        ),
    );

    // Task 287: Initialize evidence ledger for this session
    let session_id = sess
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    crate::evidence_ledger::init_session_ledger(&session_id, &sess.root);

    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage::simple("system", system_prompt),
        ChatMessage::simple("user", user_message),
    ];
    let mut tracker = CompactTracker::new();
    let mut stop_policy = StopPolicy::new(budget);

    let mut update_context_estimate =
        |msgs: &[ChatMessage], tui: &mut crate::ui_terminal::TerminalUI| {
            let mut total = 0u64;
            for m in msgs {
                total += crate::ui_terminal::TerminalUI::estimate_tokens(&m.content);
            }
            tui.update_context_tokens(total);
        };

    update_context_estimate(&messages, tui);

    loop {
        // Check 30-minute timeout
        let elapsed = loop_start.elapsed();
        if elapsed > total_timeout {
            let elapsed_mins = elapsed.as_secs() as f64 / 60.0;
            let timeout_reason = format!(
                "30-minute timeout exceeded after {:.1} minutes",
                elapsed_mins
            );
            trace(args, &format!("tool_loop: TIMEOUT {}", timeout_reason));
            return Ok(ToolLoopResult {
                final_answer: format!(
                    "⏱️ **Timeout After {:.1} Minutes**\n\n\
                     The task was cancelled due to exceeding the 30-minute time limit.\n\n\
                     **Time spent:** {:.1} minutes\n\
                     **Iterations completed:** {}\n\
                     **Tool calls made:** {}\n\n\
                     **Cause:** Slow model response time (local model)\n\n\
                     Try simplifying the request or breaking it into smaller steps.",
                    elapsed_mins,
                    elapsed_mins,
                    stop_policy.iteration(),
                    stop_policy.total_tool_calls()
                ),
                iterations: stop_policy.iteration(),
                tool_calls_made: stop_policy.total_tool_calls(),
                stopped_by_max: false,
                stop_outcome: None,
                total_elapsed_s: elapsed.as_secs() as f64,
                timeout_reason: Some(timeout_reason),
            });
        }

        // Check stop policy before starting this iteration
        if let Some(outcome) = stop_policy.start_iteration() {
            trace(
                args,
                &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
            );
            messages.push(ChatMessage::simple(
                "user",
                "You've reached the maximum number of tool calls. Please provide your final answer.",
            ));
            let mut final_content = request_final_answer_without_tools(
                tui, client, chat_url, model_id, &messages, max_tokens, false,
            )
            .await?;
            if final_answer_needs_retry(&final_content) {
                final_content = request_final_answer_without_tools(
                    tui, client, chat_url, model_id, &messages, max_tokens, true,
                )
                .await?;
            }
            let final_trimmed = normalize_final_answer_candidate(&final_content);
            return Ok(ToolLoopResult {
                final_answer: if final_answer_needs_retry(&final_trimmed) {
                    build_fallback_from_recent_tool_evidence(&messages)
                } else {
                    final_trimmed
                },
                iterations: stop_policy.iteration(),
                tool_calls_made: stop_policy.total_tool_calls(),
                stopped_by_max: true,
                stop_outcome: Some(outcome),
                total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                timeout_reason: None,
            });
        }

        // Task 121: Reset per-turn shell call counter
        crate::command_budget::get_budget().start_turn();

        // Check if we need to compact before this iteration
        tracker.recalculate(&messages);
        let (should_compact, ctx, buf) = tracker.should_compact(None, None);
        if should_compact {
            trace(
                args,
                &format!(
                    "auto_compact: firing (tokens={}, turns={}, ctx={}, buf={})",
                    tracker.total_tokens, tracker.turn_count, ctx, buf
                ),
            );
            let (new_messages, result) = if let Some(cfg) = summarizer_cfg {
                apply_compact_with_summarizer(&messages, 3, client, chat_url, cfg).await
            } else {
                apply_compact(&messages, 3)
            };
            if result.ok {
                let before_count = messages.len();
                messages = new_messages;
                tracker.record_success();
                update_context_estimate(&messages, tui);
                tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactBoundary);
                tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactSummary {
                    message_count: before_count,
                    context_preview: Some("auto compact".to_string()),
                });
                trace(
                    args,
                    &format!(
                        "auto_compact: succeeded (freed {} tokens)",
                        result.tokens_freed
                    ),
                );
            } else {
                tracker.record_failure();
                trace(args, "auto_compact: failed (no messages to compact)");
            }
        }
        trace(
            args,
            &format!(
                "tool_loop: iteration {}/{}",
                stop_policy.iteration(),
                stop_policy.max_iterations()
            ),
        );
        // Telemetry: warn when approaching budget limits
        let iter = stop_policy.iteration();
        let max_iter = stop_policy.max_iterations();
        if iter == max_iter - 2 {
            tui.push_budget_notice(&format!(
                "Approaching iteration limit ({}/{})",
                iter, max_iter
            ));
        }
        let total_calls = stop_policy.total_tool_calls();
        if total_calls >= 25 && total_calls % 5 == 0 {
            tui.push_budget_notice(&format!("Tool calls used: {}/30", total_calls));
        }
        let req = ChatCompletionRequest {
            model: model_id.to_string(),
            messages: messages.clone(),
            temperature,
            top_p: 1.0,
            stream: true,
            max_tokens,
            n_probs: None,
            repeat_penalty: None,
            reasoning_format: Some("auto".to_string()),
            grammar: None,
            tools: Some(crate::tool_calling::build_tool_definitions(&PathBuf::new())),
        };
        let turn = match request_tool_loop_model_turn_streaming(
            tui,
            client,
            chat_url,
            req.clone(),
            120,
            sess,
        )
        .await
        {
            Ok(turn) => turn,
            Err(error) => {
                append_trace_log_line(&format!("[TOOL_LOOP_STREAM_FALLBACK] {}", error));
                let mut fallback_req = req;
                fallback_req.stream = false;
                let resp = await_with_busy_input(
                    tui,
                    crate::ui_chat::chat_once_with_timeout(client, chat_url, &fallback_req, 120),
                )
                .await?;
                let choice = resp.choices.get(0).context("No choices in response")?;
                ToolLoopModelTurn {
                    content: choice.message.content.clone().unwrap_or_default(),
                    tool_calls: choice.message.tool_calls.clone().unwrap_or_default(),
                }
            }
        };
        let content = turn.content;
        if !turn.tool_calls.is_empty() {
            // Track tool calls through stop policy
            if let Some(outcome) = stop_policy.record_tool_calls(&turn.tool_calls) {
                trace(
                    args,
                    &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
                );
                let mut final_content = request_final_answer_from_evidence(
                    tui, client, chat_url, model_id,
                    &original_user_request, &messages, max_tokens,
                )
                .await?;
                if final_answer_needs_retry(&final_content) {
                    final_content = request_final_answer_without_tools(
                        tui, client, chat_url, model_id, &messages, max_tokens, true,
                    )
                    .await?;
                }
                let trimmed = normalize_final_answer_candidate(&final_content);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            }

            let mut new_signal_seen = false;
            for tc in &turn.tool_calls {
                // Use normalized signal for stagnation detection so slight
                // parameter variations (page counts, head/tail sizes) do not
                // reset the stagnation counter.
                let sig = if tc.function.name == "shell" {
                    let parsed: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null);
                    let cmd = parsed
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    crate::stop_policy::normalize_shell_signal(&cmd)
                } else {
                    tool_signal(tc)
                };
                if stop_policy.register_signal(sig) {
                    new_signal_seen = true;
                }
            }
            if new_signal_seen {
                stop_policy.record_new_signals();
            } else if let Some(outcome) = stop_policy.record_stagnation() {
                trace(
                    args,
                    "tool_loop: stagnation threshold reached; forcing finalization",
                );
                let mut final_content = request_final_answer_from_evidence(
                    tui, client, chat_url, model_id,
                    &original_user_request, &messages, max_tokens,
                )
                .await?;
                if final_answer_needs_retry(&final_content) {
                    final_content = request_final_answer_without_tools(
                        tui, client, chat_url, model_id, &messages, max_tokens, true,
                    )
                    .await?;
                }
                let trimmed = normalize_final_answer_candidate(&final_content);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: false,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            } else {
                trace(
                    args,
                    &format!(
                        "tool_loop: stagnation run {} (no new tool signal)",
                        stop_policy.stagnation_runs()
                    ),
                );
            }

            trace(
                args,
                &format!("tool_loop: {} tool call(s)", turn.tool_calls.len()),
            );
            for tc in &turn.tool_calls {
                // Task T209: Shell budget forecasting
                if tc.function.name == "shell" {
                    let (is_risky, reason) =
                        CompactTracker::forecast_shell_output_risk(&tc.function.arguments);
                    if is_risky {
                        tui.push_budget_notice(&format!(
                            "High-risk command detected: {}. Forecast: high volume.",
                            reason
                        ));

                        // If we are already over 70% capacity, compact now to make room for the risky result
                        let mut ctx_limit = tui.get_context_max() as usize;
                        if ctx_limit == 0 {
                            ctx_limit = DEFAULT_CONTEXT_WINDOW_TOKENS;
                        }
                        if tracker.total_tokens > (ctx_limit * 70 / 100) {
                            trace(
                                args,
                                "auto_compact: proactive compaction for high-risk command",
                            );
                            let (new_messages, result) = if let Some(cfg) = summarizer_cfg {
                                apply_compact_with_summarizer(&messages, 3, client, chat_url, cfg)
                                    .await
                            } else {
                                apply_compact(&messages, 3)
                            };
                            if result.ok {
                                messages = new_messages;
                                tracker.record_success();
                                tracker.recalculate(&messages);
                                update_context_estimate(&messages, tui);
                                tui.add_claude_message(
                                    crate::claude_ui::ClaudeMessage::CompactBoundary,
                                );
                                tui.push_compaction_notice(
                                    "Proactive compaction triggered to accommodate high-volume shell output.",
                                );
                            }
                        }
                    }
                }

                let result = tool_calling::execute_tool_call(
                    args,
                    tc,
                    workdir,
                    sess,
                    client,
                    chat_url,
                    user_message,
                    Some(&mut *tui),
                )
                .await;

                // Task 283: Flush tool result to session transcript and artifacts
                crate::session_flush::flush_tool_result(
                    &sess.root,
                    &tc.id,
                    &tc.function.name,
                    &result.content,
                    result.ok,
                );

                // Task 287: Add evidence ledger entry for tool result
                if tc.function.name != "read_evidence"
                    && tc.function.name != "respond"
                    && tc.function.name != "update_todo_list"
                    && tc.function.name != "tool_search"
                {
                    let source = match tc.function.name.as_str() {
                        "shell" => {
                            let cmd =
                                serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                                    .ok()
                                    .and_then(|v| v["command"].as_str().map(String::from))
                                    .unwrap_or_default();
                            crate::evidence_ledger::EvidenceSource::Shell {
                                command: cmd,
                                exit_code: if result.ok { 0 } else { 1 },
                            }
                        }
                        "read" => {
                            let path =
                                serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                                    .ok()
                                    .and_then(|v| v["path"].as_str().map(String::from))
                                    .unwrap_or_default();
                            crate::evidence_ledger::EvidenceSource::Read { path }
                        }
                        "search" => {
                            let args_val =
                                serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                                    .ok();
                            let pattern = args_val
                                .as_ref()
                                .and_then(|v| v["pattern"].as_str().map(String::from))
                                .unwrap_or_default();
                            let path = args_val
                                .as_ref()
                                .and_then(|v| v["path"].as_str().map(String::from))
                                .unwrap_or_default();
                            crate::evidence_ledger::EvidenceSource::Search { path, pattern }
                        }
                        _ => crate::evidence_ledger::EvidenceSource::Tool {
                            name: tc.function.name.clone(),
                            input: tc.function.arguments.chars().take(100).collect(),
                        },
                    };
                    crate::evidence_ledger::with_session_ledger(|ledger| {
                        ledger.add_entry(source, &result.content);
                    });
                }

                stop_policy.record_tool_result(tc, &result);

                // respond tool = final answer, exit the loop immediately (only if non-empty)
                if tc.function.name == "respond" {
                    let trimmed = normalize_final_answer_candidate(&result.content);
                    if trimmed.is_empty() {
                        trace(
                            args,
                            "tool_loop: respond returned empty answer; continuing loop",
                        );
                    } else {
                        return Ok(ToolLoopResult {
                            final_answer: trimmed,
                            iterations: stop_policy.iteration(),
                            tool_calls_made: stop_policy.total_tool_calls(),
                            stopped_by_max: true,
                            stop_outcome: None,
                            total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                            timeout_reason: None,
                        });
                    }
                }

                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: "".to_string(),
                    name: None,
                    tool_calls: Some(vec![tc.clone()]),
                    tool_call_id: None,
                    summarized: false,
                });
                // Apply result budget — persist large outputs to disk, keep inline for small ones
                let budgeted = apply_tool_result_budget(
                    sess,
                    &tc.id,
                    &tc.function.name,
                    &result.content,
                    DEFAULT_MAX_RESULT_SIZE_CHARS,
                );
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: budgeted.content_for_model,
                    name: Some(tc.function.name.clone()),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                    summarized: false,
                });
            }

            update_context_estimate(&messages, tui);

            // Check for repeated tool failures after executing all calls
            if let Some(outcome) = stop_policy.check_should_stop() {
                trace(
                    args,
                    &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
                );
                let mut final_content = request_final_answer_from_evidence(
                    tui, client, chat_url, model_id,
                    &original_user_request, &messages, max_tokens,
                )
                .await?;
                if final_answer_needs_retry(&final_content) {
                    final_content = request_final_answer_without_tools(
                        tui, client, chat_url, model_id, &messages, max_tokens, true,
                    )
                    .await?;
                }
                let trimmed = normalize_final_answer_candidate(&final_content);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            }

            continue;
        }
        if !content.trim().is_empty() {
            // Check if this looks like an intent-only response without actual evidence
            let trimmed = content.trim();
            if is_intent_only_response(&trimmed) && !has_recent_tool_evidence(&messages) {
                // Force continuation to gather actual evidence instead of accepting intent-only answer
                trace(args, "tool_loop: detected intent-only response without evidence, continuing to gather proof");
                // Push a user nudge to force action
                messages.push(ChatMessage::simple("user", "You haven't executed any tools yet. Please execute the necessary tools to answer my request accurately."));
                continue;
            } else {
                return Ok(ToolLoopResult {
                    final_answer: normalize_final_answer_candidate(&content),
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: false,
                    stop_outcome: None,
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            }
        }
    }
}

/// Check if the response looks like an intent-only statement without actual evidence gathering
fn is_intent_only_response(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    // Patterns that indicate intent without action
    let intent_patterns = [
        "the user is",
        "let me show",
        "let me demonstrate",
        "i will show",
        "i will demonstrate",
        "allow me to show",
        "i can show",
        "i could show",
        "let me explain how",
        "i determined by",
        "i came to this conclusion",
        "my conclusion was based on",
        "i figured this out by",
        "here's how i",
        "this is how i",
    ];

    intent_patterns
        .iter()
        .any(|&pattern| lower.contains(pattern))
}

/// Check if recent messages contain actual tool evidence (not just intent statements)
fn has_recent_tool_evidence(messages: &[ChatMessage]) -> bool {
    // Look at last few tool messages for actual execution evidence
    for msg in messages.iter().rev().take(5) {
        if msg.role == "tool" {
            // Tool messages with actual content indicate evidence gathering
            let content = msg.content.trim();
            if !content.is_empty() && !content.contains("<tool_call>") && !content.contains("```") {
                // Contains actual tool output, not just markup
                return true;
            }
        }
    }
    false
}

/// Extract a short preview of a tool argument.
pub(crate) fn extract_tool_arg_preview(args_json: &str, field: &str, max_len: usize) -> String {
    match serde_json::from_str::<serde_json::Value>(args_json) {
        Ok(val) => val
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| {
                if s.len() > max_len {
                    format!("{}...", s.chars().take(max_len).collect::<String>())
                } else {
                    s.to_string()
                }
            })
            .unwrap_or_else(|| args_json.chars().take(max_len).collect()),
        Err(_) => args_json.chars().take(max_len).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tool_call_markup() {
        assert!(is_tool_call_markup(
            "<tool_call>{\"name\":\"shell\"}</tool_call>"
        ));
        assert!(is_tool_call_markup(
            "{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}"
        ));
        assert!(!is_tool_call_markup(
            "The latest prompts are in sessions/history.txt."
        ));
    }

    #[test]
    fn tool_signal_uses_semantic_fields() {
        let tc = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "read".to_string(),
                arguments: r#"{"path":"sessions/history.txt"}"#.to_string(),
            },
        };
        assert_eq!(tool_signal(&tc), "read:sessions/history.txt");
    }

    #[test]
    fn normalizes_shell_signal_session_ids() {
        let a = normalize_shell_signal("ls sessions/s_1776868918_801751000/shell/");
        let b = normalize_shell_signal("ls sessions/s_1775151941_439997000/shell/");
        assert_eq!(a, b);
        assert!(a.contains("s_SESSION"));
    }

    #[test]
    fn fallback_uses_recent_tool_content() {
        let msgs = vec![
            ChatMessage::simple("user", "hello"),
            ChatMessage {
                role: "tool".to_string(),
                content: "line one\nline two".to_string(),
                name: Some("shell".to_string()),
                tool_calls: None,
                tool_call_id: Some("t1".to_string()),
                summarized: false,
            },
        ];
        let out = build_fallback_from_recent_tool_evidence(&msgs);
        assert!(out.contains("line one"));
    }

    #[test]
    fn normalize_final_answer_strips_think_and_tool_call_blocks() {
        let raw = "<think>hidden</think>\nAnswer\n<tool_call>{\"name\":\"respond\"}</tool_call>";
        assert_eq!(normalize_final_answer_candidate(raw), "Answer");
    }
}
