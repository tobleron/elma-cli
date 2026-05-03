//! @efficiency-role: domain-logic
//! Tool Loop — continuous execution loop using native tool calling.

use crate::auto_compact::{
    apply_compact, apply_compact_with_summarizer, CompactTracker, DEFAULT_COMPACT_BUFFER_TOKENS,
    DEFAULT_CONTEXT_WINDOW_TOKENS,
};
use crate::event_log;
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
    let mut reasoning_content_full = String::new();

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
                    reasoning_content_full.push_str(&reasoning);
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
        reasoning_content: if reasoning_content_full.is_empty() {
            None
        } else {
            Some(reasoning_content_full)
        },
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
    reasoning_content: Option<String>,
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
        format!("Based on the evidence gathered:\n- {}", facts.join("\n- "))
    }
}

const FINAL_EVIDENCE_MAX_ITEMS: usize = 12;
const FINAL_EVIDENCE_ITEM_MAX_CHARS: usize = 3_000;
const FINAL_EVIDENCE_TOTAL_MAX_CHARS: usize = 24_000;

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    let omitted = chars.count();
    if omitted == 0 {
        input.to_string()
    } else {
        format!("{truncated}\n[... {omitted} chars omitted from finalization evidence ...]")
    }
}

fn build_bounded_final_evidence(messages: &[ChatMessage]) -> String {
    let mut chunks_rev = Vec::new();
    let mut seen = HashSet::new();
    let mut total_chars = 0usize;

    for msg in messages.iter().rev() {
        if msg.role != "tool" {
            continue;
        }
        let content = msg.content.trim();
        if content.is_empty() {
            continue;
        }

        let dedupe_key = format!(
            "{}:{}",
            msg.name.as_deref().unwrap_or("tool"),
            content.chars().take(512).collect::<String>()
        );
        if !seen.insert(dedupe_key) {
            continue;
        }

        let tool_name = msg.name.as_deref().unwrap_or("tool");
        let body = truncate_chars(content, FINAL_EVIDENCE_ITEM_MAX_CHARS);
        let chunk = format!("Tool result ({tool_name}):\n{body}");
        let chunk_chars = chunk.chars().count();
        let remaining = FINAL_EVIDENCE_TOTAL_MAX_CHARS.saturating_sub(total_chars);
        if remaining == 0 {
            break;
        }

        if chunk_chars > remaining {
            if remaining > 200 {
                chunks_rev.push(truncate_chars(&chunk, remaining));
            }
            break;
        }

        total_chars += chunk_chars;
        chunks_rev.push(chunk);

        if chunks_rev.len() >= FINAL_EVIDENCE_MAX_ITEMS {
            break;
        }
    }

    chunks_rev.reverse();
    if chunks_rev.is_empty() {
        "(no tool results)".to_string()
    } else {
        chunks_rev.join("\n\n")
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
    let evidence_block = build_bounded_final_evidence(messages);

    let clean_messages = vec![
        ChatMessage::simple(
            "user",
            &format!(
                "{}\n\n--- Evidence gathered so far ---\n{}\n--- End evidence ---\n\nAnswer concisely using only the evidence above. Use plain text only — no markdown formatting, no headings, no tables, no code blocks, no bullet lists. Do not call tools.",
                original_user_request,
                evidence_block
            ),
        ),
    ];

    let profile = ad_hoc_profile(model_id, "tool_loop_evidence_finalizer");
    let req = chat_request_from_profile(
        &profile,
        clean_messages,
        ChatRequestOptions {
            temperature: Some(0.2),
            max_tokens: Some(max_tokens.min(runtime_llm_config().max_response_tokens_cap)),
            repeat_penalty: Some(None),
            ..ChatRequestOptions::deterministic(max_tokens)
        },
    );
    request_tool_loop_final_answer_streaming(tui, client, chat_url, req, runtime_llm_config().final_answer_timeout_s).await
}

/// Stream a final answer from the LLM, pushing content to the TUI incrementally.
async fn request_tool_loop_final_answer_streaming(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    mut req: ChatCompletionRequest,
    timeout_s: u64,
) -> Result<String> {
    req.stream = true;

    let response = client
        .post(chat_url.clone())
        .json(&req)
        .timeout(Duration::from_secs(timeout_s))
        .send()
        .await
        .context("final answer stream request failed")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API error {}: {}", status, body);
    }

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut content = String::new();

    loop {
        let chunk_opt = tokio::select! {
            chunk = byte_stream.next() => chunk,
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                let _ = tui.pump_ui();
                if let Ok(Some(queued)) = tui.poll_busy_submission() {
                    tui.enqueue_submission(queued);
                }
                continue;
            }
        };
        let Some(chunk_result) = chunk_opt else {
            break;
        };
        let chunk_bytes = chunk_result?;
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
                if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                    content.push_str(text);
                    tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantContentDelta(
                        text.to_string(),
                    ));
                    let _ = tui.pump_ui();
                }
            }
        }
    }

    content.push('\n');
    tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantFinished);
    let _ = tui.pump_ui();

    let cleaned = crate::text_utils::strip_thinking_blocks(&content);
    Ok(cleaned.trim().to_string())
}

async fn finalize_from_evidence_or_fallback(
    args: &Args,
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    original_user_request: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
) -> String {
    let mut final_content = match request_final_answer_from_evidence(
        tui,
        client,
        chat_url,
        model_id,
        original_user_request,
        messages,
        max_tokens,
    )
    .await
    {
        Ok(content) => content,
        Err(e) => {
            trace(
                args,
                &format!("finalization_failed_nonfatal stage=evidence error={}", e),
            );
            build_fallback_from_recent_tool_evidence(messages)
        }
    };

    if final_answer_needs_retry(&final_content) {
        final_content = match request_final_answer_without_tools(
            tui, client, chat_url, model_id, messages, max_tokens, true,
        )
        .await
        {
            Ok(content) => content,
            Err(e) => {
                trace(
                    args,
                    &format!("finalization_failed_nonfatal stage=plain_retry error={}", e),
                );
                build_fallback_from_recent_tool_evidence(messages)
            }
        };
    }

    final_content
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
            "Return plain terminal text only. No markdown formatting, no headings, no tables, no code blocks, no bullet lists. Do not emit XML/JSON tool calls or function-call markup.",
        ));
    }
    let profile = ad_hoc_profile(model_id, "tool_loop_plain_finalizer");
    let req = chat_request_from_profile(
        &profile,
        req_messages,
        ChatRequestOptions {
            max_tokens: Some(max_tokens.min(runtime_llm_config().max_response_tokens_cap)),
            repeat_penalty: Some(None),
            ..ChatRequestOptions::deterministic(max_tokens)
        },
    );
    let resp = await_with_busy_input(
        tui,
        crate::ui_chat::chat_once_with_timeout(
            client,
            chat_url,
            &req,
            runtime_llm_config().final_answer_timeout_s,
        ),
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
        "respond" => {
            let answer = parsed
                .get("answer")
                .or_else(|| parsed.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let snippet: String = answer.chars().take(40).collect();
            format!("respond:{}", snippet)
        }
        "summary" => String::new(), // Don't count summary toward stagnation - it stops the loop
        other => format!("{other}:{}", tc.function.arguments),
    };
    if fn_name == "respond" {
        return key;
    }
    if fn_name == "shell" {
        format!("{fn_name}:{}", crate::text_utils::normalize_shell_signal(&key))
    } else {
        format!("{fn_name}:{key}")
    }
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
    context_hint: &str,
    evidence_required: bool,
    ctx_max: Option<u64>,
    goal_state: &GoalState,
    complexity: &str,
) -> Result<ToolLoopResult> {
    let budget = StageBudget::from_complexity(complexity);
    let total_timeout = Duration::from_secs(45 * 60); // 45 minutes
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
    crate::event_log::init_session_event_log(&session_id);

    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage::simple("system", system_prompt),
        ChatMessage::simple("user", user_message),
    ];
    let mut tracker = CompactTracker::new();
    let mut stop_policy = StopPolicy::new(budget);
    // Track tool outcomes keyed by normalized signal.
    // Used to skip duplicate tool calls and keep their results from previous execution.
    // Maps signal -> (success, preview_content)
    let mut tool_outcomes: std::collections::HashMap<String, (bool, String)> = std::collections::HashMap::new();

    let mut update_context_estimate =
        |msgs: &[ChatMessage], tui: &mut crate::ui_terminal::TerminalUI| {
            let mut total = 0u64;
            for m in msgs {
                total += crate::ui_terminal::TerminalUI::estimate_tokens(&m.content);
            }
            tui.update_context_tokens(total);
        };

    update_context_estimate(&messages, tui);

    let mut turn_counter: usize = 0;

    loop {
        turn_counter += 1;
        let turn_id = format!("turn_{}", turn_counter);
        // Task 470: Mark current turn and record turn start
        crate::event_log::set_current_turn(&turn_id);
        crate::event_log::record_lifecycle(
            crate::event_log::LifecycleEventType::TurnStarted,
            Some(&turn_id),
        );

        // Check 45-minute timeout
        let elapsed = loop_start.elapsed();
        if elapsed > total_timeout {
            let elapsed_mins = elapsed.as_secs() as f64 / 60.0;
            let timeout_reason = format!(
                "45-minute timeout exceeded after {:.1} minutes",
                elapsed_mins
            );
            trace(args, &format!("tool_loop: TIMEOUT {}", timeout_reason));
            // Record finalization and finish turn
            crate::event_log::record_finalization(
                crate::event_log::FinalizationEventType::FinalAnswerPrepared,
                &turn_id,
                "timeout",
            );
            crate::event_log::record_lifecycle(
                crate::event_log::LifecycleEventType::TurnFinished,
                Some(&turn_id),
            );
            crate::event_log::clear_current_turn();
            let _ = crate::event_log::persist(&sess.root);
            tui.push_stop_notice(&format!("Timeout: {}", timeout_reason));
            return Ok(ToolLoopResult {
                final_answer: format!(
                    "⏱️ **Timeout After {:.1} Minutes**\n\n\
                     The task was cancelled due to exceeding the 45-minute time limit.\n\n\
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
            let final_content = finalize_from_evidence_or_fallback(
                args,
                tui,
                client,
                chat_url,
                model_id,
                &original_user_request,
                &messages,
                max_tokens,
            )
            .await;
            let final_trimmed = normalize_final_answer_candidate(&final_content);
            // Record finalization events (stop policy and final answer)
            crate::event_log::record_finalization(
                crate::event_log::FinalizationEventType::FinalAnswerPrepared,
                &turn_id,
                outcome.reason.as_str(),
            );
            crate::event_log::record_finalization(
                crate::event_log::FinalizationEventType::StopPolicyTriggered,
                &turn_id,
                outcome.reason.as_str(),
            );
            // Finalize turn lifecycle and persist
            crate::event_log::record_lifecycle(
                crate::event_log::LifecycleEventType::TurnFinished,
                Some(&turn_id),
            );
            crate::event_log::clear_current_turn();
            let _ = crate::event_log::persist(&sess.root);
            tui.push_stop_notice(&format!("Budget limit: {}", outcome.reason.as_str()));
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
        let (should_compact, ctx, buf) = tracker.should_compact(ctx_max.map(|v| v as usize), None);
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
        let max_iter = stop_policy.max_iterations();
        if max_iter > 0 {
            trace(
                args,
                &format!(
                    "tool_loop: iteration {}/{}",
                    stop_policy.iteration(),
                    max_iter
                ),
            );
        }
        // Telemetry: warn when approaching budget limits (only when a limit is set)
        let iter = stop_policy.iteration();
        if max_iter > 0 && iter >= max_iter.saturating_sub(2) {
            tui.push_budget_notice(&format!(
                "Approaching iteration limit ({}/{})",
                iter, max_iter
            ));
        }
        let total_calls = stop_policy.total_tool_calls();
        let profile = ad_hoc_profile(model_id, "tool_loop");
        let req = chat_request_from_profile(
            &profile,
            messages.clone(),
            ChatRequestOptions {
                temperature: Some(temperature),
                top_p: Some(1.0),
                stream: Some(true),
                max_tokens: Some(max_tokens.min(runtime_llm_config().tool_loop_max_tokens_cap)),
                repeat_penalty: Some(None),
                reasoning_format: Some(Some("auto".to_string())),
                tools: Some(crate::tool_calling::build_tool_definitions(&PathBuf::new())),
                ..ChatRequestOptions::default()
            },
        );
        // Task 470: Record ModelRequestStarted event
        crate::event_log::record_model_event(
            crate::event_log::ModelEventType::ModelRequestStarted,
            &turn_id,
            None,
            None,
        );
        let turn = match request_tool_loop_model_turn_streaming(
            tui,
            client,
            chat_url,
            req.clone(),
            runtime_llm_config().tool_loop_timeout_s,
            sess,
        )
        .await
        {
            Ok(turn) => {
                // Task 470: Record ModelResponseReceived event
                crate::event_log::record_model_event(
                    crate::event_log::ModelEventType::ModelResponseReceived,
                    &turn_id,
                    None,
                    None,
                );
                // Record ModelToolCallProposed for each tool call
                for tc in &turn.tool_calls {
                    crate::event_log::record_model_event(
                        crate::event_log::ModelEventType::ModelToolCallProposed,
                        &turn_id,
                        Some(&tc.id),
                        None,
                    );
                }
                turn
            }
            Err(error) => {
                append_trace_log_line(&format!("[TOOL_LOOP_STREAM_FALLBACK] {}", error));
                let mut fallback_req = req;
                fallback_req.stream = false;
                let resp = await_with_busy_input(
                    tui,
                    crate::ui_chat::chat_once_with_timeout(
                        client,
                        chat_url,
                        &fallback_req,
                        runtime_llm_config().tool_loop_timeout_s,
                    ),
                )
                .await?;
                let choice = resp.choices.get(0).context("No choices in response")?;
                // Task 470: Record ModelResponseReceived event for fallback path
                crate::event_log::record_model_event(
                    crate::event_log::ModelEventType::ModelResponseReceived,
                    &turn_id,
                    None,
                    None,
                );
                let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
                // Record ModelToolCallProposed for each tool call in fallback
                for tc in &tool_calls {
                    crate::event_log::record_model_event(
                        crate::event_log::ModelEventType::ModelToolCallProposed,
                        &turn_id,
                        Some(&tc.id),
                        None,
                    );
                }
                ToolLoopModelTurn {
                    content: choice.message.content.clone().unwrap_or_default(),
                    tool_calls,
                    reasoning_content: choice.message.reasoning_content.clone(),
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
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    max_tokens,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                tui.push_stop_notice(&format!("Tool call limit: {}", outcome.reason.as_str()));
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
                    crate::text_utils::normalize_shell_signal(&cmd)
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
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    max_tokens,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                tui.push_stop_notice(&format!("Stagnation: {}", outcome.reason.as_str()));
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
                let stagnation_info = stop_policy.stagnation_trace_info();
                trace(
                    args,
                    &format!(
                        "tool_loop: {} (no new tool signal)",
                        stagnation_info
                    ),
                );
                // Task 540: Surface stagnation warning to transcript if persistent
                if stop_policy.stagnation_runs() >= 3 {
                    tui.push_meta_event("STAGNATION", &stagnation_info);
                }
            }

            trace(
                args,
                &format!("tool_loop: {} tool call(s)", turn.tool_calls.len()),
            );
            // Preserve model narrative text alongside tool calls
            if !content.trim().is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: content.clone(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: turn.reasoning_content.clone(),
                    summarized: false,
                });
            }
            let mut first_tool_call = true;
            for tc in &turn.tool_calls {
                // ── Duplicate gate: skip if this exact command succeeded earlier ──
                // Prevents the model from re-running the same successful tool call
                // when it forgets (common with small models).
                // Exempts respond and summary — they are designed to be called
                // multiple times with different content.
                let sig = tool_signal(tc);
                if tc.function.name != "respond"
                    && tc.function.name != "summary"
                    && tc.function.name != "workspace_info"
                    && tc.function.name != "tool_search"
                {
                    if let Some((ok, prev)) = tool_outcomes.get(&sig) {
                        if *ok {
                            trace(args, &format!("tool_loop: duplicate skipped (already succeeded) signal={}", sig));
                            messages.push(ChatMessage::simple(
                                "system",
                                &format!("Already completed earlier — same result: {}", prev),
                            ));
                            continue;
                        } else {
                            // Task 537: If it failed before, allow retry but warn model
                            trace(args, &format!("tool_loop: duplicate detected (previous failure) signal={}", sig));
                            messages.push(ChatMessage::simple(
                                "system",
                                &format!("Note: Your previous attempt at '{}' failed. Only retry if you have changed the arguments or have a new strategy.", sig),
                            ));
                        }
                    }
                }
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
                            ctx_limit = ctx_max
                                .map(|v| v as usize)
                                .unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS);
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

                // Task 470: Record ToolStarted event
                crate::event_log::record_tool_event(
                    crate::event_log::ToolEventType::ToolStarted,
                    &turn_id,
                    &tc.id,
                    &tc.function.name,
                );

                let mut result = tool_calling::execute_tool_call(
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

                // Task 470: Record ToolFinished or ToolFailed event
                let tool_event_type = if result.ok {
                    crate::event_log::ToolEventType::ToolFinished
                } else {
                    crate::event_log::ToolEventType::ToolFailed
                };
                crate::event_log::record_tool_event(
                    tool_event_type,
                    &turn_id,
                    &tc.id,
                    &tc.function.name,
                );

                // Task 283: Flush tool result to session transcript and artifacts
                crate::session_flush::flush_tool_result(
                    &sess.root,
                    &tc.id,
                    &tc.function.name,
                    &result.content,
                    result.ok,
                );

                // Task 287: Add evidence ledger entry for tool result
                if tc.function.name != "respond"
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
                                exit_code: result.exit_code.unwrap_or(if result.ok {
                                    0
                                } else {
                                    1
                                }),
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
                        // Strip ANSI escape sequences from result content
                        let clean_content =
                            match strip_ansi_escapes::strip(result.content.as_bytes()) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                Err(_) => result.content.clone(), // Fallback: return raw if stripping fails
                            };
                        let entry = ledger.add_entry(source, &clean_content);
                        // Task 470: Record EvidenceEvent for the new ledger entry
                        let source_artifact =
                            entry.raw_path.as_deref().unwrap_or(&tc.function.name);
                        crate::event_log::record_evidence_event(
                            &turn_id,
                            &entry.summary,
                            source_artifact,
                        );
                    });
                }

                stop_policy.record_tool_result(tc, &result);

                // T333: Mark real tool calls & reset respond counter
                if tc.function.name != "respond"
                    && tc.function.name != "summary"
                    && tc.function.name != "update_todo_list"
                {
                    stop_policy.mark_real_tool_call();
                    stop_policy.reset_respond_counter();
                }

                // T333: evidence_required gate — block respond before evidence exists
                if tc.function.name == "respond"
                    && evidence_required
                    && !stop_policy.has_real_tool_calls_this_turn()
                {
                    // Replace respond result with correction
                    let correction = "You must collect evidence before answering.\n\
                        Use search, read, or shell to gather facts. Do not call 'respond' yet.";
                    result.content = correction.to_string();
                    trace(
                        args,
                        "tool_loop: evidence_required gate blocked respond before evidence",
                    );
                }

                // T333: respond abuse guard — inject correction after 3 consecutive responds
                if tc.function.name == "respond" {
                    stop_policy.increment_respond_counter();
                    if stop_policy.consecutive_respond_calls() >= 3
                        && !stop_policy.has_real_tool_calls_this_turn()
                    {
                        messages.push(ChatMessage::simple(
                            "user",
                            "⚠️ You have called 'respond' 3 times without collecting any evidence. \
                             You have not used search, read, shell, or any other tool to gather facts. \
                             Your respond messages are status updates, not evidence. \
                             Call a real tool now to answer the user's question, or reply with 'I cannot answer this.'",
                        ));
                        stop_policy.reset_respond_counter();
                        trace(
                            args,
                            "tool_loop: injected respond abuse correction after 3 consecutive responds",
                        );
                    }
                }

                // Task 422: Check respond content against evidence ledger
                // Uses model-free heuristic overlap scoring — no hardcoded keyword triggers.
                if tc.function.name == "respond"
                    && !result.content.is_empty()
                    && !stop_policy.has_real_tool_calls_this_turn()
                {
                    if let Some(ledger) = crate::evidence_ledger::get_session_ledger() {
                        if ledger.entries_count() > 0 {
                            let verdict = crate::evidence_ledger::enforce_evidence_grounding(
                                &result.content,
                                &ledger,
                            );
                            let ungrounded = verdict.ungrounded_claims();
                            if !ungrounded.is_empty() {
                                let reasons: Vec<&str> =
                                    ungrounded.iter().map(|c| c.statement.as_str()).collect();
                                let msg = format!(
                                    "ungrounded claims without evidence: {}",
                                    reasons.join(" | ")
                                );
                                trace(args, &format!("tool_loop: respond {}", msg));
                                tui.push_meta_event("EVIDENCE", &msg);
                                let correction = format!(
                                    "⚠️ Your previous response contains claims not supported by evidence. \
                                     You must call a real tool (shell, search, read) to gather facts \
                                     before making factual statements. Do not fabricate information."
                                );
                                result.content = correction;
                                trace(args, "tool_loop: respond blocked by evidence gate");
                            }
                        }
                    }
                }

                // summary tool = run final summary intel unit, then exit loop
                // respond tool ALWAYS continues the loop (interim status, not final)
                if tc.function.name == "summary" {
                    let raw_content = normalize_final_answer_candidate(&result.content);
                    if raw_content.is_empty() {
                        trace(
                            args,
                            "tool_loop: summary returned empty answer; continuing loop",
                        );
                    } else {
                        // For simple turns (few iterations, few tool calls), use
                        // the model's own summary content directly. The intel
                        // summarizer can produce structured output that replaces
                        // natural responses for simple conversational exchanges.
                        let is_simple_turn =
                            stop_policy.total_tool_calls() <= 2 && stop_policy.iteration() <= 2;
                        let final_answer = if is_simple_turn {
                            raw_content
                        } else {
                            let final_summary = run_final_summary_intel(
                                args,
                                client,
                                summarizer_cfg,
                                &original_user_request,
                                &raw_content,
                            )
                            .await;
                            final_summary.unwrap_or(raw_content)
                        };

                        // Task 540: Surface stop reason to transcript before exit
                        tui.push_meta_event("STOP", "Task completed via summary tool");

                        tui.push_stop_notice("Completed via summary tool");
                        return Ok(ToolLoopResult {
                            final_answer,
                            iterations: stop_policy.iteration(),
                            tool_calls_made: stop_policy.total_tool_calls(),
                            stopped_by_max: true,
                            stop_outcome: None,
                            total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                            timeout_reason: None,
                        });
                    }
                }
                // respond always continues the loop - it's for interim status, not final answer

                // ── Result gate: store outcomes for dedup ──
                let store_for_dedup = tc.function.name != "respond"
                    && tc.function.name != "summary"
                    && tc.function.name != "workspace_info"
                    && tc.function.name != "tool_search";
                
                if store_for_dedup {
                    let preview = result.content.chars().take(200).collect::<String>();
                    tool_outcomes.insert(sig, (result.ok, preview));
                }

                if result.ok {

                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: "".to_string(),
                        name: None,
                        tool_calls: Some(vec![tc.clone()]),
                        tool_call_id: None,
                        reasoning_content: None,
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
                    // Empty result guard: inject placeholder for ok-but-empty results
                    let model_content = if budgeted.content_for_model.trim().is_empty()
                        && tc.function.name != "respond"
                    {
                        "(empty result)".to_string()
                    } else {
                        budgeted.content_for_model
                    };

                    // Task 332: Append reflection to tool results
                    let reflection = crate::evidence_ledger::get_session_ledger()
                        .and_then(|ledger| ledger.get_latest_reflection())
                        .map(|r| format!("\n→ Reflection: {}", r))
                        .unwrap_or_default();

                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: format!("{}{}", model_content, reflection),
                        name: Some(tc.function.name.clone()),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                        reasoning_content: None,
                        summarized: false,
                    });
                } else {
                    // Failure: don't bloat context with full error output.
                    // Push a single compact message instead.
                    messages.push(ChatMessage::simple(
                        "system",
                        "That attempt failed. Try a different approach.",
                    ));
                }
            }

            update_context_estimate(&messages, tui);

            // Goal consistency check: fires every 18 tool calls
            if stop_policy.goal_consistency_check_needed() && goal_state.has_active_goal() {
                let recent_tool_summary = build_recent_tool_summary(&messages, 15);
                let profile = ad_hoc_profile(model_id, "goal_consistency");
                let steering = crate::intel_units::run_goal_consistency_check(
                    client,
                    &profile,
                    goal_state,
                    &recent_tool_summary,
                )
                .await;
                if let Some(steering_msg) = steering {
                    trace(
                        args,
                        &format!(
                            "tool_loop: goal consistency steering injected ({} chars)",
                            steering_msg.len()
                        ),
                    );
                    messages.push(ChatMessage::simple("user", &steering_msg));
                }
            }

            // T303: Inject strategy-shift hint if retry loop detected
            if stop_policy.is_retry_loop_detected() {
                if let Some(hint) = stop_policy.strategy_shift_hint() {
                    trace(args, &format!("tool_loop: {}", hint.replace('\n', " | ")));
                    messages.push(ChatMessage::simple("user", &hint));
                }
            }

            // T304: Force finalization after repeated failures to preserve output budget
            // If 5+ consecutive shell failures, force final answer before context is exhausted
            let consecutive_failures = stop_policy.consecutive_shell_failures();
            if consecutive_failures >= 5 {
                trace(
                    args,
                    &format!(
                        "tool_loop: forcing finalization after {} consecutive shell failures (T304 budget preservation)",
                        consecutive_failures
                    ),
                );
                messages.push(ChatMessage::simple(
                    "user",
                    "You've had 5+ consecutive shell failures. Stop trying shell commands and provide your final answer based on the evidence you already have. If you cannot answer reliably, explain what you found and what additional information would be needed."
                ));
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    max_tokens,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                tui.push_stop_notice("Forced finalization due to repeated shell failures");
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(StopOutcome {
                        reason: StopReason::RepeatedToolFailure,
                        stage_index: 0,
                        stage_skill: "general".to_string(),
                        summary: format!("Forced finalization after {} consecutive shell failures to preserve output budget", consecutive_failures),
                        next_step_hint: "Verify commands manually before retrying, or use a different approach (read/search tools instead of shell)".to_string(),
                    }),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            }

            // T306: Surface struggle detection as transcript row
            if stop_policy.is_struggling() {
                tui.push_meta_event("STRUGGLE", "Model detected as struggling (repeated failures/stagnation). Decomposition recommended.");
            }

            // Check for repeated tool failures after executing all calls
            let dbg_failures = stop_policy.tool_failures_count();
            if dbg_failures > 0 {
                trace(args, &format!("tool_loop: tool_failures={}", dbg_failures));
            }
            if let Some(outcome) = stop_policy.check_should_stop() {
                trace(
                    args,
                    &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
                );
                tui.push_meta_event(
                    "STOP",
                    &format!(
                        "Stopping: {} - {}",
                        outcome.reason.as_str(),
                        outcome.summary
                    ),
                );
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    max_tokens,
                )
                .await;
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

            // T333: Respond-only turn tracking — if no real tools were used this iteration
            if !stop_policy.has_real_tool_calls_this_turn() {
                if let Some(outcome) = stop_policy.record_respond_only_turn() {
                    trace(
                        args,
                        &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
                    );
                    messages.push(ChatMessage::simple(
                        "user",
                        "You've called 'respond' 5+ times without using any real tools (search, read, shell). \
                         Provide your final answer now based on what you know, even if incomplete.",
                    ));
                    let final_content = finalize_from_evidence_or_fallback(
                        args,
                        tui,
                        client,
                        chat_url,
                        model_id,
                        &original_user_request,
                        &messages,
                        max_tokens,
                    )
                    .await;
                    let trimmed = normalize_final_answer_candidate(&final_content);
                    // Record finalization events
                    crate::event_log::record_finalization(
                        crate::event_log::FinalizationEventType::FinalAnswerPrepared,
                        &turn_id,
                        outcome.reason.as_str(),
                    );
                    crate::event_log::record_finalization(
                        crate::event_log::FinalizationEventType::StopPolicyTriggered,
                        &turn_id,
                        outcome.reason.as_str(),
                    );
                    // Finalize turn lifecycle and persist
                    crate::event_log::record_lifecycle(
                        crate::event_log::LifecycleEventType::TurnFinished,
                        Some(&turn_id),
                    );
                    crate::event_log::clear_current_turn();
                    let _ = crate::event_log::persist(&sess.root);
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

/// Build a compact summary of the most recent N tool calls from messages.
/// Returns one line per tool call: "tool_name: arg_preview"
fn build_recent_tool_summary(messages: &[ChatMessage], count: usize) -> String {
    let mut lines = Vec::new();
    for msg in messages.iter().rev() {
        if lines.len() >= count {
            break;
        }
        if let Some(tcs) = &msg.tool_calls {
            for tc in tcs.iter().rev() {
                if lines.len() >= count {
                    break;
                }
                let preview = match tc.function.name.as_str() {
                    "shell" => {
                        let cmd = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                            .ok()
                            .and_then(|v| v["command"].as_str().map(|s| s.to_string()))
                            .unwrap_or_default();
                        let short = if cmd.len() > 80 {
                            format!("{}...", &cmd[..77])
                        } else {
                            cmd
                        };
                        format!("shell: {}", short)
                    }
                    "read" => {
                        let path =
                            serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                                .ok()
                                .and_then(|v| v["path"].as_str().map(|s| s.to_string()))
                                .unwrap_or_else(|| {
                                    tc.function.arguments.chars().take(60).collect()
                                });
                        format!("read: {}", path)
                    }
                    "search" => {
                        let pattern =
                            serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                                .ok()
                                .and_then(|v| v["pattern"].as_str().map(|s| s.to_string()))
                                .unwrap_or_else(|| {
                                    tc.function.arguments.chars().take(60).collect()
                                });
                        format!("search: {}", pattern)
                    }
                    other => {
                        format!(
                            "{}: {}",
                            other,
                            tc.function.arguments.chars().take(60).collect::<String>()
                        )
                    }
                };
                lines.push(preview);
            }
        }
    }
    lines.reverse();
    lines.join("\n")
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
        let a = crate::text_utils::normalize_shell_signal("ls sessions/s_1776868918_801751000/shell/");
        let b = crate::text_utils::normalize_shell_signal("ls sessions/s_1775151941_439997000/shell/");
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
                reasoning_content: None,
                summarized: false,
            },
        ];
        let out = build_fallback_from_recent_tool_evidence(&msgs);
        assert!(out.contains("line one"));
    }

    #[test]
    fn finalization_evidence_is_bounded_and_recent() {
        let old = "old evidence ".repeat(500);
        let recent = "recent evidence ".repeat(500);
        let mut msgs = vec![ChatMessage::simple("user", "summarize")];
        msgs.push(ChatMessage {
            role: "tool".to_string(),
            content: old,
            name: Some("read".to_string()),
            tool_calls: None,
            tool_call_id: Some("old".to_string()),
            reasoning_content: None,
            summarized: false,
        });
        msgs.push(ChatMessage {
            role: "tool".to_string(),
            content: recent,
            name: Some("read".to_string()),
            tool_calls: None,
            tool_call_id: Some("recent".to_string()),
            reasoning_content: None,
            summarized: false,
        });

        let block = build_bounded_final_evidence(&msgs);
        assert!(block.contains("recent evidence"));
        assert!(block.chars().count() <= FINAL_EVIDENCE_TOTAL_MAX_CHARS + 120);
        assert!(block.contains("omitted from finalization evidence"));
    }

    #[test]
    fn normalize_final_answer_strips_think_and_tool_call_blocks() {
        let raw = "<think>hidden</think>\nAnswer\n<tool_call>{\"name\":\"respond\"}</tool_call>";
        assert_eq!(normalize_final_answer_candidate(raw), "Answer");
    }

    #[test]
    fn tool_signal_respond_non_empty() {
        let tc = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "respond".to_string(),
                arguments: r#"{"answer":"Searching for undo tasks in the project"}"#.to_string(),
            },
        };
        let sig = tool_signal(&tc);
        assert!(!sig.is_empty(), "respond signal should be non-empty");
        assert!(
            sig.starts_with("respond:"),
            "respond signal should have prefix"
        );
        assert!(
            sig.contains("Searching"),
            "respond signal should contain answer snippet"
        );
    }

    #[test]
    fn tool_signal_respond_truncates() {
        let long_answer = "a".repeat(100);
        let tc = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "respond".to_string(),
                arguments: format!(r#"{{"answer":"{}"}}"#, long_answer),
            },
        };
        let sig = tool_signal(&tc);
        assert!(
            sig.len() <= "respond:".len() + 40,
            "respond signal should be truncated to 40 chars + prefix, got len {}",
            sig.len()
        );
        // With 100-char answer, signal should be exactly respond: + 40 chars
        assert_eq!(sig.len(), "respond:".len() + 40);
    }

    #[test]
    fn tool_signal_respond_different_messages_different_signals() {
        let tc1 = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "respond".to_string(),
                arguments: r#"{"answer":"Searching for tasks"}"#.to_string(),
            },
        };
        let tc2 = ToolCall {
            id: "c2".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "respond".to_string(),
                arguments: r#"{"answer":"Found the files"}"#.to_string(),
            },
        };
        assert_ne!(tool_signal(&tc1), tool_signal(&tc2));
    }

    #[test]
    fn tool_signal_respond_identical_messages_identical_signals() {
        let tc1 = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "respond".to_string(),
                arguments: r#"{"answer":"I am searching..."}"#.to_string(),
            },
        };
        let tc2 = ToolCall {
            id: "c2".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "respond".to_string(),
                arguments: r#"{"answer":"I am searching..."}"#.to_string(),
            },
        };
        assert_eq!(tool_signal(&tc1), tool_signal(&tc2));
    }
}

async fn run_final_summary_intel(
    args: &Args,
    client: &reqwest::Client,
    summarizer_cfg: Option<&Profile>,
    user_request: &str,
    model_provided_content: &str,
) -> Option<String> {
    use crate::intel_trait::execute_intel_text_from_user_content;

    let cfg = summarizer_cfg?;

    let evidence_summary = crate::evidence_ledger::get_session_ledger()
        .map(|ledger| {
            ledger
                .entries
                .iter()
                .map(|e| e.summary.clone())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let narrative = format!(
        r#"Generate a concise final summary (1-2 sentences max).

User request: {}

Evidence:
{}

Model's draft answer:
{}

Provide a short, direct answer."#,
        user_request,
        if evidence_summary.is_empty() {
            "(none)".to_string()
        } else {
            evidence_summary
        },
        model_provided_content
    );

    match execute_intel_text_from_user_content(client, cfg, narrative).await {
        Ok(summary) => Some(summary),
        Err(e) => {
            trace(args, &format!("final_summary_intel failed: {}", e));
            None
        }
    }
}
