//! @efficiency-role: domain-logic
//! Tool Loop — continuous execution loop using native tool calling.

use crate::auto_compact::{
    apply_compact, apply_compact_with_summarizer, CompactTracker, DEFAULT_COMPACT_BUFFER_TOKENS,
    DEFAULT_CONTEXT_WINDOW_TOKENS,
};
use crate::decomposition_pyramid::DecompositionPyramid;
use crate::evidence_ledger::{EvidenceLedger, EvidenceQuality};
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

/// Embedded GBNF grammar for action DSL output.
/// The config root resolves to the system config directory
/// (~/Library/Application Support/...), not the repo config/ tree,
/// so grammar is embedded in the binary rather than loaded from disk.
const ACTION_DSL_GBNF: &str = r##"root ::= r-line | l-line | s-line | y-line | x-block | ask-block | done-block | e-block
r-line ::= "R " rest "\n"
l-line ::= "L " rest "\n"
s-line ::= "S " rest "\n"
y-line ::= "Y " rest "\n"
x-block   ::= "X\n"    body "\n---END\n"
ask-block ::= "ASK\n"  body "\n---END\n"
done-block ::= "DONE\n" body "\n---END\n"
e-block ::= "E " rest "\n---OLD\n" body "\n---NEW\n" body "\n---END\n"
rest ::= [^\n]*
body ::= [^\n]*"##;

fn load_action_grammar() -> Option<String> {
    Some(ACTION_DSL_GBNF.to_string())
}

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

fn ensure_tool_loop_reasoning_format(req: &mut ChatCompletionRequest) {
    req.reasoning_format
        .get_or_insert_with(|| "none".to_string());
}

async fn request_tool_loop_model_turn_streaming(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    mut req: ChatCompletionRequest,
    timeout_s: u64,
    session: &SessionPaths,
    display_assistant_content: bool,
) -> Result<ToolLoopModelTurn> {
    req.stream = true;
    ensure_tool_loop_reasoning_format(&mut req);

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
    // Provider XML tool-call tags
    if lower.contains("<tool_call>")
        || lower.contains("</tool_call>")
        || lower.contains("<command>")
        || lower.contains("</command>")
    {
        return true;
    }
    // Provider JSON tool-call patterns
    if (lower.contains("\"name\"")
        && lower.contains("\"arguments\"")
        && (lower.contains("\"name\":\"shell\"")
            || lower.contains("\"name\":\"read\"")
            || lower.contains("\"name\":\"search\"")
            || lower.contains("\"name\":\"respond\"")
            || lower.contains("\"name\":\"update_todo_list\"")
            || lower.contains("\"name\":\"tool_search\"")
            || lower.contains("\"name\": \"shell\"")
            || lower.contains("\"name\": \"read\"")
            || lower.contains("\"name\": \"search\"")
            || lower.contains("\"name\": \"respond\"")
            || lower.contains("\"name\": \"update_todo_list\"")
            || lower.contains("\"name\": \"tool_search\"")))
    {
        return true;
    }
    // Action DSL commands in final answer (should be plain text only)
    if lower.starts_with("r path=")
        || lower.starts_with("l path=")
        || lower.starts_with("s q=")
        || lower.starts_with("y q=")
        || lower.starts_with("e path=")
        || lower.starts_with("x\n")
        || lower.starts_with("done")
        || lower.starts_with("ask\n")
    {
        return true;
    }
    false
}

fn normalize_final_answer_candidate(text: &str) -> String {
    crate::text_utils::strip_thinking_blocks(text)
        .trim()
        .to_string()
}

/// Build a clean finalization context from the evidence ledger and session state.
/// Includes only: user request, route/intent, executed tool summaries, artifact paths,
/// stop reason, and unresolved blockers. No stale tool-call history.
fn build_finalization_context(
    original_user_request: &str,
    route: &str,
    stop_outcome: Option<&StopOutcome>,
    evidence_ledger: Option<&EvidenceLedger>,
) -> String {
    let evidence_summary = evidence_ledger
        .map(|l| l.compact_summary())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(no evidence)".to_string());

    let fresh_entries = evidence_ledger
        .map(|l| l.fresh_entries())
        .unwrap_or_default();
    let fresh_count = fresh_entries.len();

    let stop_reason = stop_outcome
        .map(|o| format!("Stop reason: {:?}", o.reason))
        .unwrap_or_default();

    let next_step_hint = stop_outcome
        .map(|o| o.next_step_hint.as_str())
        .unwrap_or("");

    let context = format!(
        r#"USER REQUEST: {request}

ROUTE: {route}
INTENT: Use the evidence below to answer the user's request directly.

EVIDENCE GATHERED ({fresh_count} fresh entries):
{evidence_summary}

{stop_reason}
{next_step}

Answer concisely using only the evidence above. Do not call tools, emit XML, or output DSL commands."#,
        request = original_user_request,
        route = route,
        fresh_count = fresh_count,
        evidence_summary = evidence_summary,
        stop_reason = stop_reason,
        next_step = if next_step_hint.is_empty() {
            String::new()
        } else {
            format!("NEXT STEP HINT: {}", next_step_hint)
        },
    );
    context
}

fn final_answer_needs_retry(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.is_empty() || is_tool_call_markup(trimmed) || is_intent_only_response(trimmed)
}

fn build_fallback_from_recent_tool_evidence(messages: &[ChatMessage]) -> String {
    // Prefer EvidenceLedger compact_summary over raw message scanning
    if let Some(ledger) = crate::evidence_ledger::get_session_ledger() {
        let summary = ledger.compact_summary();
        if !summary.is_empty() {
            let fresh = ledger.fresh_entries();
            if fresh.is_empty() {
                return "I examined the workspace but found no fresh evidence to answer from. Please retry with a more specific prompt.".to_string();
            }
            let evidence_list: Vec<String> = fresh
                .iter()
                .map(|e| {
                    let raw = ledger.get_raw(&e.id).unwrap_or_else(|_| e.summary.clone());
                    let body = if raw.trim().is_empty() {
                        e.summary.clone()
                    } else {
                        raw.trim().chars().take(2000).collect::<String>()
                    };
                    format!("- {} {}\n{}", e.id, e.source, body)
                })
                .collect();
            return format!(
                "Based on the evidence gathered:\n{}\n\n(This is the best answer I could produce from the available evidence.)",
                evidence_list.join("\n")
            );
        }
    }
    // Fall back to raw message scanning if ledger is unavailable
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
    let finalization_context = build_finalization_context(
        original_user_request,
        "evidence_finalizer",
        None,
        crate::evidence_ledger::get_session_ledger().as_ref(),
    );
    append_trace_log_line(&format!(
        "[FINALIZATION_CONTEXT] len={} chars preview=\"{}\"",
        finalization_context.len(),
        finalization_context
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect::<String>()
    ));

    let clean_messages = vec![ChatMessage::simple("user", &finalization_context)];

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

async fn request_final_answer_without_tools(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
    force_plain_text: bool,
) -> Result<String> {
    // Use clean context when forcing plain text (retry path)
    if force_plain_text {
        // Extract user request from the first user message
        let original_request = messages
            .iter()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");
        let context = build_finalization_context(
            original_request,
            "plain_finalizer",
            None,
            crate::evidence_ledger::get_session_ledger().as_ref(),
        );
        append_trace_log_line(&format!(
            "[FINALIZATION_CONTEXT_RETRY] len={} chars",
            context.len()
        ));
        let profile = ad_hoc_profile(model_id, "tool_loop_plain_finalizer");
        let req = chat_request_from_profile(
            &profile,
            vec![ChatMessage::simple("user", &context)],
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
        return Ok(normalize_final_answer_candidate(
            &resp
                .choices
                .first()
                .map(|c| c.message.content.clone().unwrap_or_default())
                .unwrap_or_default(),
        ));
    }
    let mut req_messages = messages.to_vec();
    if force_plain_text {
        req_messages.push(ChatMessage::simple(
            "user",
            "Return plain terminal text only. Do not emit XML/JSON tool calls or function-call markup.",
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
    if let Some(ledger) = crate::evidence_ledger::get_session_ledger() {
        if ledger
            .fresh_entries()
            .iter()
            .any(|entry| matches!(entry.quality, EvidenceQuality::Direct))
        {
            return true;
        }
        if !ledger.fresh_entries().is_empty() {
            return false;
        }
    }

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
    pyramid: Option<&DecompositionPyramid>,
) -> Result<ToolLoopResult> {
    let budget = StageBudget::default();
    let total_timeout = Duration::from_secs(45 * 60);
    let loop_start = Instant::now();
    let original_user_request = user_message.to_string();
    trace(
        args,
        &format!(
            "tool_loop: starting max_iterations={} stagnation_threshold={} timeout=30m",
            budget.max_iterations, budget.max_stagnation_cycles
        ),
    );
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

    // Inject pyramid context if available (keeps the model on-track)
    if let Some(pyra) = pyramid {
        let ctx = pyra.render_context();
        if !ctx.trim().is_empty() {
            messages.push(ChatMessage::simple(
                "user",
                &format!("=== ACTIVE PLAN ===\n{}=== END PLAN ===", ctx),
            ));
        }
    }
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
        let elapsed = loop_start.elapsed();
        if elapsed > total_timeout {
            let elapsed_mins = elapsed.as_secs() as f64 / 60.0;
            return Ok(ToolLoopResult {
                final_answer: format!("Timeout after {:.1} minutes", elapsed_mins),
                iterations: stop_policy.iteration(),
                tool_calls_made: stop_policy.total_tool_calls(),
                stopped_by_max: false,
                stop_outcome: None,
                total_elapsed_s: elapsed.as_secs() as f64,
                timeout_reason: Some(format!("{}s", elapsed.as_secs())),
            });
        }
        if let Some(outcome) = stop_policy.start_iteration() {
            append_trace_log_line(&format!(
                "[FINALIZATION_STOP] reason={:?} iteration={} tool_calls={}",
                outcome.reason,
                stop_policy.iteration(),
                stop_policy.total_tool_calls()
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
            let ft = normalize_final_answer_candidate(&final_content);
            return Ok(ToolLoopResult {
                final_answer: if final_answer_needs_retry(&ft) {
                    build_fallback_from_recent_tool_evidence(&messages)
                } else {
                    ft
                },
                iterations: stop_policy.iteration(),
                tool_calls_made: stop_policy.total_tool_calls(),
                stopped_by_max: true,
                stop_outcome: Some(outcome),
                total_elapsed_s: elapsed.as_secs() as f64,
                timeout_reason: None,
            });
        }
        crate::command_budget::get_budget().start_turn();
        tracker.recalculate(&messages);
        let (should_compact, ctx, buf) = tracker.should_compact(ctx_max.map(|v| v as usize), None);
        if should_compact {
            let (new_messages, result) = if let Some(cfg) = summarizer_cfg {
                let ledger = crate::evidence_ledger::get_session_ledger();
                apply_compact_with_summarizer(&messages, 3, client, chat_url, cfg, ledger.as_ref())
                    .await
            } else {
                let ledger = crate::evidence_ledger::get_session_ledger();
                apply_compact(&messages, 3, ledger.as_ref())
            };
            if result.ok {
                let before_tokens = tracker.total_tokens;
                messages = new_messages;
                tracker.record_success();
                tracker.recalculate(&messages);
                let after_tokens = tracker.total_tokens;
                update_context_estimate(&messages, tui);
                tui.push_compaction_notice(&format!(
                    "compacted: {}→{} tokens (freed {})",
                    before_tokens, after_tokens, result.tokens_freed
                ));
            }
        }
        let profile = ad_hoc_profile(model_id, "tool_loop");
        let action_grammar = load_action_grammar();
        let req = chat_request_from_profile(
            &profile,
            messages.clone(),
            ChatRequestOptions {
                temperature: Some(temperature),
                top_p: Some(1.0),
                stream: Some(true),
                max_tokens: Some(max_tokens.min(runtime_llm_config().tool_loop_max_tokens_cap)),
                repeat_penalty: Some(None),
                reasoning_format: Some(Some("none".to_string())),
                tools: None,
                grammar: action_grammar,
                ..ChatRequestOptions::default()
            },
        );
        let turn = match request_tool_loop_model_turn_streaming(
            tui,
            client,
            chat_url,
            req.clone(),
            runtime_llm_config().tool_loop_timeout_s,
            sess,
            false,
        )
        .await
        {
            Ok(t) => t,
            Err(e) => {
                append_trace_log_line(&format!("[TOOL_LOOP_STREAM_FALLBACK] {}", e));
                let mut fb = req;
                fb.stream = false;
                let r = await_with_busy_input(
                    tui,
                    crate::ui_chat::chat_once_with_timeout(
                        client,
                        chat_url,
                        &fb,
                        runtime_llm_config().tool_loop_timeout_s,
                    ),
                )
                .await?;
                let c = r.choices.get(0).context("No choices")?;
                ToolLoopModelTurn {
                    content: c.message.content.clone().unwrap_or_default(),
                    tool_calls: c.message.tool_calls.clone().unwrap_or_default(),
                    reasoning_content: c.message.reasoning_content.clone(),
                }
            }
        };
        let content = turn.content;
        if turn.tool_calls.is_empty() {
            let cleaned = normalize_action_dsl_candidate(&content);
            let trimmed = cleaned.trim();
            if trimmed.is_empty() {
                if has_recent_tool_evidence(&messages) {
                    trace(args, "tool_loop: empty action DSL after evidence; finalizing from grounded tool output");
                    let raw = build_fallback_from_recent_tool_evidence(&messages);
                    let final_answer = run_final_summary_intel_or_fallback(
                        args,
                        client,
                        summarizer_cfg,
                        &original_user_request,
                        raw,
                    )
                    .await;
                    return Ok(ToolLoopResult {
                        final_answer,
                        iterations: stop_policy.iteration(),
                        tool_calls_made: stop_policy.total_tool_calls(),
                        stopped_by_max: false,
                        stop_outcome: None,
                        total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                        timeout_reason: None,
                    });
                }
                let ctx = crate::dsl::ParseContext {
                    dsl_variant: "action",
                    line: None,
                };
                let repair = render_focused_repair(ActionRepairKind::GeneralDsl, false);
                trace(
                    args,
                    &format!(
                        "tool_loop: model output was empty (raw content len={})",
                        content.len()
                    ),
                );
                stop_policy.record_dsl_failure_with_kind(ActionRepairKind::GeneralDsl);
                let simplify = simplify_hint_for_attempt(stop_policy.consecutive_dsl_failures());
                let repair_msg = if let Some(pyra) = pyramid {
                    format!(
                        "{}\n{}\n\n=== AVAILABLE TASKS ===\n{}\n=== END TASKS ===",
                        repair,
                        simplify,
                        pyra.render_task_menu()
                    )
                } else {
                    format!("{}\n{}", repair, simplify)
                };
                tui.push_dsl_repair_notice(&repair_msg);
                messages.push(ChatMessage::simple("user", &repair_msg));
                if let Some(outcome) = stop_policy.check_should_stop() {
                    return Ok(ToolLoopResult {
                        final_answer: build_exhaustion_answer(
                            &original_user_request,
                            stop_policy.consecutive_dsl_failures(),
                            &messages,
                        ),
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
            let ctx = crate::dsl::ParseContext {
                dsl_variant: "action",
                line: None,
            };
            let actions = match parse_actions_batch(trimmed, &ctx, 3) {
                Ok(actions) => {
                    // Emit transcript notice when model used provider XML markup
                    if trimmed.contains("<tool_call>") || trimmed.contains("<command>") {
                        tui.push_provider_markup_notice(
                            "provider markup normalized to native action DSL",
                        );
                    }
                    actions
                }
                Err(error) => {
                    if has_recent_tool_evidence(&messages) {
                        trace(
                            args,
                            "tool_loop: invalid action DSL after evidence; finalizing",
                        );
                        let raw = build_fallback_from_recent_tool_evidence(&messages);
                        let final_answer = run_final_summary_intel_or_fallback(
                            args,
                            client,
                            summarizer_cfg,
                            &original_user_request,
                            raw,
                        )
                        .await;
                        return Ok(ToolLoopResult {
                            final_answer,
                            iterations: stop_policy.iteration(),
                            tool_calls_made: stop_policy.total_tool_calls(),
                            stopped_by_max: false,
                            stop_outcome: None,
                            total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                            timeout_reason: None,
                        });
                    }
                    let kind = classify_action_repair(&error, &content);
                    stop_policy.record_dsl_failure_with_kind(kind);
                    let escalated = stop_policy.should_escalate_repair(kind);
                    let repair = render_focused_repair(kind, escalated);
                    let consecutive = stop_policy.consecutive_dsl_failures();
                    let simplify = simplify_hint_for_attempt(consecutive);
                    trace(args, &format!("tool_loop: invalid DSL parse kind={:?} escalated={} consecutive={} error={} raw='{}'", kind, escalated, consecutive, error.code, content.chars().take(300).collect::<String>().escape_default()));
                    let repair_msg = if let Some(pyra) = pyramid {
                        let task_menu = pyra.render_task_menu();
                        if escalated {
                            format!("{}\n{}\n\nAll retries exhausted. Pick a different task from your plan:\n{}\nUse NEXT task_id=<id> action=<action>", repair, simplify, task_menu)
                        } else {
                            format!(
                                "{}\n{}\n\n=== ACTIVE TASKS ===\n{}\n=== END TASKS ===",
                                repair, simplify, task_menu
                            )
                        }
                    } else {
                        format!("{}\n{}", repair, simplify)
                    };
                    tui.push_dsl_repair_notice(&repair_msg);
                    messages.push(ChatMessage::simple("user", &repair_msg));
                    if let Some(outcome) = stop_policy.check_should_stop() {
                        return Ok(ToolLoopResult {
                            final_answer: build_exhaustion_answer(
                                &original_user_request,
                                consecutive,
                                &messages,
                            ),
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
            };
            // Task 408: Record action verb for fuzzy stagnation detection.
            for act in &actions {
                let verb = match act {
                    AgentAction::ReadFile { .. } => "R",
                    AgentAction::ListFiles { .. } => "L",
                    AgentAction::SearchText { .. } => "S",
                    AgentAction::SearchSymbol { .. } => "Y",
                    AgentAction::EditFile { .. } => "E",
                    AgentAction::RunCommand { .. } => "X",
                    AgentAction::Ask { .. } => "ASK",
                    AgentAction::Done { .. } => "DONE",
                    AgentAction::Next { .. } => "NEXT",
                };
                stop_policy.record_action_verb(verb);
            }
            // Task 408: Check fuzzy stagnation after recording action verbs.
            if let Some(outcome) = stop_policy.check_fuzzy_stagnation() {
                return Ok(ToolLoopResult {
                    final_answer: if has_recent_tool_evidence(&messages) {
                        build_fallback_from_recent_tool_evidence(&messages)
                    } else {
                        "I'm repeating the same action type without progress. Let me start fresh with a different approach.".to_string()
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            }
            for action in actions {
                if let AgentAction::Ask { question } = action {
                    if evidence_required && !has_recent_tool_evidence(&messages) {
                        let repair = "INVALID_DSL detail=\"ASK used without evidence\" hint=\"gather facts with R, L, S, Y, or X first, then use DONE or ASK\"";
                        stop_policy.record_dsl_failure("action_ask");
                        tui.push_dsl_repair_notice(repair);
                        messages.push(ChatMessage::simple("user", repair));
                        continue;
                    }
                    let final_answer =
                        if question.trim().starts_with('"') && question.trim().ends_with('"') {
                            format!(
                                "I have a question: {}",
                                &question.trim()[1..question.trim().len() - 1]
                            )
                        } else {
                            format!("I have a question: {}", question)
                        };
                    return Ok(ToolLoopResult {
                        final_answer,
                        iterations: stop_policy.iteration(),
                        tool_calls_made: stop_policy.total_tool_calls(),
                        stopped_by_max: false,
                        stop_outcome: None,
                        total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                        timeout_reason: None,
                    });
                } else if let AgentAction::Done { summary } = action {
                    if evidence_required && !has_recent_tool_evidence(&messages) {
                        let repair = "INVALID_DSL detail=\"DONE used before collecting evidence\" hint=\"use R, L, S, Y, or X before DONE\"";
                        stop_policy.record_dsl_failure("action_done");
                        tui.push_dsl_repair_notice(repair);
                        messages.push(ChatMessage::simple("user", repair));
                        continue;
                    }
                    let raw = normalize_final_answer_candidate(&summary);
                    let final_answer = if !raw.is_empty() {
                        run_final_summary_intel_or_fallback(
                            args,
                            client,
                            summarizer_cfg,
                            &original_user_request,
                            raw,
                        )
                        .await
                    } else {
                        raw
                    };
                    return Ok(ToolLoopResult {
                        final_answer,
                        iterations: stop_policy.iteration(),
                        tool_calls_made: stop_policy.total_tool_calls(),
                        stopped_by_max: false,
                        stop_outcome: None,
                        total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                        timeout_reason: None,
                    });
                } else if let AgentAction::Next {
                    task_id,
                    action: next_action,
                    reason,
                } = action
                {
                    tui.push_decomposition_notice(&format!(
                        "next task: task_id={task_id} action={next_action} reason=\"{reason}\""
                    ));
                    continue;
                } else {
                    let action = normalize_action_for_execution(workdir, action);
                    let (trace_name, trace_command) = action_trace_preview(&action);
                    if should_emit_tool_trace_row(&trace_name) {
                        tui.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
                            name: trace_name.clone(),
                            command: trace_command,
                        });
                        let _ = tui.pump_ui();
                    }
                    let execution =
                        execute_agent_action(args, workdir, sess, client, chat_url, action, tui)
                            .await;
                    let (tool_call, result) = match execution {
                        Ok(AgentActionExecution::Continue { tool_call, result }) => {
                            if should_emit_tool_trace_row(&trace_name) {
                                tui.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
                                    name: trace_name.clone(),
                                    success: result.ok,
                                    output: truncate_tool_trace_output(&result.content),
                                });
                                let _ = tui.pump_ui();
                            }
                            (tool_call, result)
                        }
                        Ok(AgentActionExecution::Ask { question }) => {
                            let fa = if question.trim().starts_with('"')
                                && question.trim().ends_with('"')
                            {
                                format!(
                                    "I have a question: {}",
                                    &question.trim()[1..question.trim().len() - 1]
                                )
                            } else {
                                format!("I have a question: {}", question)
                            };
                            return Ok(ToolLoopResult {
                                final_answer: fa,
                                iterations: stop_policy.iteration(),
                                tool_calls_made: stop_policy.total_tool_calls(),
                                stopped_by_max: false,
                                stop_outcome: None,
                                total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                                timeout_reason: None,
                            });
                        }
                        Ok(AgentActionExecution::Done { summary }) => {
                            let raw = normalize_final_answer_candidate(&summary);
                            let fa = if !raw.is_empty() {
                                run_final_summary_intel_or_fallback(
                                    args,
                                    client,
                                    summarizer_cfg,
                                    &original_user_request,
                                    raw,
                                )
                                .await
                            } else {
                                raw
                            };
                            return Ok(ToolLoopResult {
                                final_answer: fa,
                                iterations: stop_policy.iteration(),
                                tool_calls_made: stop_policy.total_tool_calls(),
                                stopped_by_max: false,
                                stop_outcome: None,
                                total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                                timeout_reason: None,
                            });
                        }
                        Err(error) => {
                            let repair = dsl_execution_repair(&error);
                            if should_emit_tool_trace_row(&trace_name) {
                                tui.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
                                    name: trace_name.clone(),
                                    success: false,
                                    output: truncate_tool_trace_output(&repair),
                                });
                                let _ = tui.pump_ui();
                            }
                            stop_policy.record_dsl_failure("action_execution");
                            tui.push_dsl_repair_notice(&repair);
                            messages.push(ChatMessage::simple("user", &repair));
                            if let Some(outcome) = stop_policy.check_should_stop() {
                                return Ok(ToolLoopResult {
                                    final_answer: if has_recent_tool_evidence(&messages) {
                                        build_fallback_from_recent_tool_evidence(&messages)
                                    } else {
                                        "I could not continue because action execution failed repeatedly. No workspace changes were made.".to_string()
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
                    };
                    if let Some(outcome) =
                        stop_policy.record_tool_calls(std::slice::from_ref(&tool_call))
                    {
                        return Ok(ToolLoopResult {
                            final_answer: if has_recent_tool_evidence(&messages) {
                                build_fallback_from_recent_tool_evidence(&messages)
                            } else {
                                "Maximum tool calls reached before enough evidence was available for a reliable answer.".to_string()
                            },
                            iterations: stop_policy.iteration(),
                            tool_calls_made: stop_policy.total_tool_calls(),
                            stopped_by_max: true,
                            stop_outcome: Some(outcome),
                            total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                            timeout_reason: None,
                        });
                    }
                    stop_policy.mark_real_tool_call();
                    stop_policy.reset_respond_counter();
                    crate::session_flush::flush_tool_result(
                        &sess.root,
                        &tool_call.id,
                        &tool_call.function.name,
                        &result.content,
                        result.ok,
                    );
                    record_evidence_for_tool_result(&tool_call, &result);
                    stop_policy.record_tool_result(&tool_call, &result);
                    if result.ok {
                        stop_policy.reset_dsl_failure_streak();
                    }
                    append_tool_result_messages(
                        sess,
                        &mut messages,
                        &tool_call,
                        &result,
                        turn.reasoning_content.clone(),
                    );
                    update_context_estimate(&messages, tui);
                    stop_policy.record_new_signals();
                }
            }
            continue;
        }

        // Provider-native tool_calls → AgentAction boundary adapter.
        // Some providers return structured tool_calls even when tools: None was
        // requested. Convert each to an AgentAction and execute through the same
        // action pipeline used by text DSL.
        for tc in &turn.tool_calls {
            let action = match convert_tool_call_to_action(tc) {
                Some(a) => a,
                None => {
                    trace(
                        args,
                        &format!(
                            "tool_loop: unhandled provider tool_call name={}",
                            tc.function.name
                        ),
                    );
                    continue;
                }
            };

            // Show normalization transcript row (collapsible, visible)
            let norm_preview = format!("provider tool_call({}) → AgentAction", tc.function.name);
            tui.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
                name: "normalize".to_string(),
                command: norm_preview.clone(),
            });
            let _ = tui.pump_ui();
            tui.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
                name: "normalize".to_string(),
                success: true,
                output: format!("{}: {}", tc.function.name, tool_trace_command_preview(tc)),
            });
            let _ = tui.pump_ui();

            // Execute through the same action pipeline as DSL actions
            let action = normalize_action_for_execution(workdir, action);
            let (trace_name, trace_command) = action_trace_preview(&action);
            if should_emit_tool_trace_row(&trace_name) {
                tui.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
                    name: trace_name.clone(),
                    command: trace_command,
                });
                let _ = tui.pump_ui();
            }
            let execution =
                execute_agent_action(args, workdir, sess, client, chat_url, action, tui).await;
            let (tool_call, result) = match execution {
                Ok(AgentActionExecution::Continue { tool_call, result }) => {
                    if should_emit_tool_trace_row(&trace_name) {
                        tui.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
                            name: trace_name.clone(),
                            success: result.ok,
                            output: truncate_tool_trace_output(&result.content),
                        });
                        let _ = tui.pump_ui();
                    }
                    (tool_call, result)
                }
                Ok(AgentActionExecution::Ask { question }) => {
                    let fa = if question.trim().starts_with('"') && question.trim().ends_with('"') {
                        format!(
                            "I have a question: {}",
                            &question.trim()[1..question.trim().len() - 1]
                        )
                    } else {
                        format!("I have a question: {}", question)
                    };
                    return Ok(ToolLoopResult {
                        final_answer: fa,
                        iterations: stop_policy.iteration(),
                        tool_calls_made: stop_policy.total_tool_calls(),
                        stopped_by_max: false,
                        stop_outcome: None,
                        total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                        timeout_reason: None,
                    });
                }
                Ok(AgentActionExecution::Done { summary }) => {
                    let raw = normalize_final_answer_candidate(&summary);
                    let fa = if !raw.is_empty() {
                        run_final_summary_intel_or_fallback(
                            args,
                            client,
                            summarizer_cfg,
                            &original_user_request,
                            raw,
                        )
                        .await
                    } else {
                        raw
                    };
                    return Ok(ToolLoopResult {
                        final_answer: fa,
                        iterations: stop_policy.iteration(),
                        tool_calls_made: stop_policy.total_tool_calls(),
                        stopped_by_max: false,
                        stop_outcome: None,
                        total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                        timeout_reason: None,
                    });
                }
                Err(error) => {
                    let repair = dsl_execution_repair(&error);
                    if should_emit_tool_trace_row(&trace_name) {
                        tui.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
                            name: trace_name.clone(),
                            success: false,
                            output: truncate_tool_trace_output(&repair),
                        });
                        let _ = tui.pump_ui();
                    }
                    stop_policy.record_dsl_failure("action_execution");
                    tui.push_dsl_repair_notice(&repair);
                    messages.push(ChatMessage::simple("user", &repair));
                    if let Some(outcome) = stop_policy.check_should_stop() {
                        return Ok(ToolLoopResult {
                            final_answer: if has_recent_tool_evidence(&messages) {
                                build_fallback_from_recent_tool_evidence(&messages)
                            } else {
                                "I could not continue because action execution failed repeatedly. No workspace changes were made.".to_string()
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
            };
            if let Some(outcome) = stop_policy.record_tool_calls(std::slice::from_ref(&tool_call)) {
                return Ok(ToolLoopResult {
                    final_answer: if has_recent_tool_evidence(&messages) {
                        build_fallback_from_recent_tool_evidence(&messages)
                    } else {
                        "Maximum tool calls reached before enough evidence was available for a reliable answer.".to_string()
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                });
            }
            stop_policy.mark_real_tool_call();
            stop_policy.reset_respond_counter();
            crate::session_flush::flush_tool_result(
                &sess.root,
                &tool_call.id,
                &tool_call.function.name,
                &result.content,
                result.ok,
            );
            record_evidence_for_tool_result(&tool_call, &result);
            stop_policy.record_tool_result(&tool_call, &result);
            if result.ok {
                stop_policy.reset_dsl_failure_streak();
            }
            append_tool_result_messages(
                sess,
                &mut messages,
                &tool_call,
                &result,
                turn.reasoning_content.clone(),
            );
            update_context_estimate(&messages, tui);
            stop_policy.record_new_signals();
        }
        update_context_estimate(&messages, tui);
    }
}

fn normalize_action_dsl_candidate(text: &str) -> String {
    let ctx = crate::dsl::ParseContext {
        dsl_variant: "action",
        line: None,
    };
    let raw = text.trim();
    if !raw.is_empty() && parse_actions_batch(raw, &ctx, 3).is_ok() {
        return raw.to_string();
    }
    let reasoning_stripped = crate::text_utils::strip_reasoning_blocks(text);
    let reasoning_stripped = reasoning_stripped.trim();
    if !reasoning_stripped.is_empty()
        && reasoning_stripped != raw
        && parse_actions_batch(reasoning_stripped, &ctx, 3).is_ok()
    {
        return reasoning_stripped.to_string();
    }
    let candidates = crate::text_utils::structured_output_candidates(text);
    for c in &candidates {
        if parse_actions_batch(c, &ctx, 3).is_ok() {
            return c.trim().to_string();
        }
    }
    candidates.first().cloned().unwrap_or_else(|| {
        crate::text_utils::strip_thinking_blocks(text)
            .trim()
            .to_string()
    })
}

fn should_emit_tool_trace_row(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "shell" | "read" | "search" | "list" | "ls" | "edit"
    )
}

fn tool_trace_command_preview(tc: &ToolCall) -> String {
    let args: serde_json::Value =
        serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);
    match tc.function.name.as_str() {
        "shell" => args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or(&tc.function.arguments)
            .to_string(),
        _ => tc.function.arguments.clone(),
    }
}

fn action_trace_preview(action: &AgentAction) -> (String, String) {
    match action {
        AgentAction::ReadFile { path } => ("read".to_string(), format!("$ cat {path}")),
        AgentAction::ListFiles { path, depth } => {
            ("list".to_string(), format!("$ ls {path} (depth={depth})"))
        }
        AgentAction::SearchText { q, path } => {
            ("search".to_string(), format!("$ rg -i -l -F {q} {path}"))
        }
        AgentAction::SearchSymbol { q, path } => {
            ("search".to_string(), format!("$ rg -w -l {q} {path}"))
        }
        AgentAction::EditFile { path, .. } => ("edit".to_string(), format!("> edit {path}")),
        AgentAction::RunCommand { command } => ("shell".to_string(), command.clone()),
        AgentAction::Ask { .. } => ("ask".to_string(), String::new()),
        AgentAction::Done { .. } => ("done".to_string(), String::new()),
        AgentAction::Next {
            task_id, action, ..
        } => (
            "next".to_string(),
            format!("task_id={task_id} action={action}"),
        ),
    }
}

fn truncate_tool_trace_output(output: &str) -> String {
    let max = 2000;
    if output.len() <= max {
        return output.to_string();
    }
    let mut out: String = output.chars().take(max).collect();
    out.push_str("\n...(truncated)");
    out
}

/// Truncate search output for model context, preserving the full output
/// in session artifacts via apply_tool_result_budget's persistence.
fn truncate_search_output(content: &str) -> String {
    if content.len() <= MAX_SEARCH_OUTPUT_CHARS {
        return content.to_string();
    }
    let mut out: String = content.chars().take(MAX_SEARCH_OUTPUT_CHARS).collect();
    let omitted = content.len() - MAX_SEARCH_OUTPUT_CHARS;
    out.push_str(&format!("\n... (truncated {} more chars)", omitted));
    out
}

fn dsl_execution_repair(error: &anyhow::Error) -> String {
    format!("DSL_EXECUTION_ERROR: {}", error)
}

fn normalize_action_for_execution(workdir: &Path, action: AgentAction) -> AgentAction {
    match action {
        AgentAction::ReadFile { path } => {
            if crate::resolve_workspace_path(workdir, &path)
                .map(|f| f.is_dir())
                .unwrap_or(false)
            {
                AgentAction::ListFiles { path, depth: 1 }
            } else {
                AgentAction::ReadFile { path }
            }
        }
        other => other,
    }
}

#[derive(Debug)]
pub(crate) enum AgentActionExecution {
    Continue {
        tool_call: ToolCall,
        result: crate::tool_calling::ToolExecutionResult,
    },
    Ask {
        question: String,
    },
    Done {
        summary: String,
    },
}

pub(crate) async fn execute_agent_action(
    args: &Args,
    workdir: &PathBuf,
    session: &SessionPaths,
    _client: &reqwest::Client,
    _chat_url: &Url,
    action: AgentAction,
    _tui: &mut crate::ui_terminal::TerminalUI,
) -> Result<AgentActionExecution> {
    match action {
        AgentAction::Ask { question } => Ok(AgentActionExecution::Ask { question }),
        AgentAction::Done { summary } => Ok(AgentActionExecution::Done { summary }),
        AgentAction::ReadFile { path } => {
            let full = crate::resolve_workspace_path(workdir, &path).map_err(anyhow::Error::msg)?;
            let (content, _) = crate::document_adapter::read_file_smart(&full)
                .map_err(|e| anyhow::anyhow!("Error reading {}: {}", full.display(), e))?;
            crate::record_session_read(&session_key(session), &path);
            let tc = ToolCall {
                id: "dsl-read".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "read".into(),
                    arguments: serde_json::json!({"path": path}).to_string(),
                },
            };
            Ok(AgentActionExecution::Continue {
                tool_call: tc,
                result: crate::tool_calling::ToolExecutionResult {
                    tool_call_id: "dsl-read".into(),
                    tool_name: "read".into(),
                    content,
                    ok: true,
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
                },
            })
        }
        AgentAction::ListFiles { path, depth } => {
            let target =
                crate::resolve_workspace_path(workdir, &path).map_err(anyhow::Error::msg)?;
            let content = list_workspace_entries(workdir, &target, depth)
                .map_err(|e| anyhow::anyhow!("Error listing {}: {}", target.display(), e))?;
            let tc = ToolCall {
                id: "dsl-list".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "list".into(),
                    arguments: serde_json::json!({"path": path, "depth": depth}).to_string(),
                },
            };
            Ok(AgentActionExecution::Continue {
                tool_call: tc,
                result: crate::tool_calling::ToolExecutionResult {
                    tool_call_id: "dsl-list".into(),
                    tool_name: "list".into(),
                    content,
                    ok: true,
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
                },
            })
        }
        AgentAction::SearchText { q, path } => {
            let target =
                crate::resolve_workspace_path(workdir, &path).map_err(anyhow::Error::msg)?;
            let (ok, code, content, path) =
                run_rg_search_with_bounded_broadening(workdir, &target, &q, &path, false).await?;
            let content = truncate_search_output(&content);
            let tc = ToolCall {
                id: "dsl-search".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "search".into(),
                    arguments: serde_json::json!({"pattern": q, "path": path}).to_string(),
                },
            };
            Ok(AgentActionExecution::Continue {
                tool_call: tc,
                result: crate::tool_calling::ToolExecutionResult {
                    tool_call_id: "dsl-search".into(),
                    tool_name: "search".into(),
                    content,
                    ok,
                    exit_code: code,
                    timed_out: false,
                    signal_killed: None,
                },
            })
        }
        AgentAction::SearchSymbol { q, path } => {
            let target =
                crate::resolve_workspace_path(workdir, &path).map_err(anyhow::Error::msg)?;
            let (ok, code, content, path) =
                run_rg_search_with_bounded_broadening(workdir, &target, &q, &path, true).await?;
            let content = truncate_search_output(&content);
            let tc = ToolCall {
                id: "dsl-search-sym".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "search".into(),
                    arguments: serde_json::json!({"pattern": q, "path": path}).to_string(),
                },
            };
            Ok(AgentActionExecution::Continue {
                tool_call: tc,
                result: crate::tool_calling::ToolExecutionResult {
                    tool_call_id: "dsl-search-sym".into(),
                    tool_name: "search".into(),
                    content,
                    ok,
                    exit_code: code,
                    timed_out: false,
                    signal_killed: None,
                },
            })
        }
        AgentAction::EditFile { path, old, new } => {
            let key = session_key(session);
            crate::require_session_read_before_edit(&key, &path).map_err(anyhow::Error::msg)?;
            let _ = crate::ensure_session_edit_snapshot(&key, session, workdir, "dsl-edit")
                .map_err(anyhow::Error::msg)?;
            match crate::apply_exact_edit(workdir, &path, &old, &new) {
                Ok(outcome) => {
                    let content = format!("{}\n{}", outcome.summary, outcome.diff);
                    let tc = ToolCall {
                        id: "dsl-edit".into(),
                        call_type: "function".into(),
                        function: ToolFunctionCall {
                            name: "edit".into(),
                            arguments: serde_json::json!({"path": path}).to_string(),
                        },
                    };
                    Ok(AgentActionExecution::Continue {
                        tool_call: tc,
                        result: crate::tool_calling::ToolExecutionResult {
                            tool_call_id: "dsl-edit".into(),
                            tool_name: "edit".into(),
                            content,
                            ok: true,
                            exit_code: None,
                            timed_out: false,
                            signal_killed: None,
                        },
                    })
                }
                Err(e) => Err(anyhow::anyhow!("Edit failed: {}", e)),
            }
        }
        AgentAction::RunCommand { command } => {
            trace(args, &format!("dsl_action: executing command {}", command));
            const COMMAND_TIMEOUT_SECS: u64 = 30;
            match crate::execute_command_policy_async(
                workdir,
                &command,
                crate::CommandPolicy::Strict,
                COMMAND_TIMEOUT_SECS,
            )
            .await
            {
                Ok(outcome) => {
                    let ok = outcome.exit_code == Some(0);
                    let content = if ok {
                        if outcome.stdout.trim().is_empty() && !outcome.stderr.trim().is_empty() {
                            outcome.stderr.clone()
                        } else {
                            outcome.stdout.clone()
                        }
                    } else {
                        format!(
                            "Command failed (exit {}):\n{}",
                            outcome.exit_code.unwrap_or(-1),
                            if outcome.stderr.trim().is_empty() {
                                outcome.stdout.clone()
                            } else {
                                outcome.stderr.clone()
                            }
                        )
                    };
                    let tc = ToolCall {
                        id: "dsl-shell".into(),
                        call_type: "function".into(),
                        function: ToolFunctionCall {
                            name: "shell".into(),
                            arguments: serde_json::json!({"command": command}).to_string(),
                        },
                    };
                    Ok(AgentActionExecution::Continue {
                        tool_call: tc,
                        result: crate::tool_calling::ToolExecutionResult {
                            tool_call_id: "dsl-shell".into(),
                            tool_name: "shell".into(),
                            content,
                            ok,
                            exit_code: outcome.exit_code,
                            timed_out: false,
                            signal_killed: None,
                        },
                    })
                }
                Err(e) => {
                    let tc = ToolCall {
                        id: "dsl-shell".into(),
                        call_type: "function".into(),
                        function: ToolFunctionCall {
                            name: "shell".into(),
                            arguments: serde_json::json!({"command": command}).to_string(),
                        },
                    };
                    Ok(AgentActionExecution::Continue {
                        tool_call: tc,
                        result: crate::tool_calling::ToolExecutionResult {
                            tool_call_id: "dsl-shell".into(),
                            tool_name: "shell".into(),
                            content: e,
                            ok: false,
                            exit_code: None,
                            timed_out: true,
                            signal_killed: None,
                        },
                    })
                }
            }
        }
        AgentAction::Next {
            task_id,
            action: _next_action,
            reason,
        } => Ok(AgentActionExecution::Ask {
            question: format!("Suggested next task_id={task_id}: {reason}"),
        }),
    }
}

fn session_key(session: &SessionPaths) -> String {
    session
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| session.root.display().to_string())
}

const MAX_DSL_LIST_ENTRIES_FOR_MODEL: usize = 500;
const MAX_SEARCH_OUTPUT_CHARS: usize = 50_000;

fn list_workspace_entries(workdir: &Path, target: &Path, depth: u8) -> Result<String> {
    if !target.is_dir() {
        anyhow::bail!("target is not a directory");
    }
    let root = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    let mut pending = vec![(target.to_path_buf(), 1u8)];
    let mut entries = Vec::new();
    while let Some((dir, level)) = pending.pop() {
        let mut local = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            local.push(entry?.path());
        }
        local.sort();
        for path in local {
            let display = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            if path.is_dir() {
                entries.push(format!("{display}/"));
                if level < depth {
                    pending.push((path, level + 1));
                }
            } else {
                entries.push(display);
            }
        }
    }
    entries.sort();
    if entries.len() > MAX_DSL_LIST_ENTRIES_FOR_MODEL {
        let omitted = entries.len() - MAX_DSL_LIST_ENTRIES_FOR_MODEL;
        entries.truncate(MAX_DSL_LIST_ENTRIES_FOR_MODEL);
        entries.push(format!("... truncated {omitted} more entries"));
    }
    Ok(if entries.is_empty() {
        "(empty directory)".to_string()
    } else {
        entries.join("\n")
    })
}

async fn run_rg_search(
    workdir: &Path,
    target: &Path,
    query: &str,
    whole_word: bool,
) -> Result<(bool, Option<i32>, String)> {
    let mut cmd = tokio::process::Command::new("rg");
    cmd.kill_on_drop(true);
    cmd.current_dir(workdir)
        .arg("-i")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--color=never");
    if whole_word {
        cmd.arg("-w");
    }
    cmd.arg("-F").arg(query).arg(target);
    let output = match tokio::time::timeout(Duration::from_secs(20), cmd.output()).await {
        Ok(output) => output.map_err(|e| anyhow::anyhow!("Search error: {}", e))?,
        Err(_) => return Ok((false, None, "Search timed out after 20 seconds".to_string())),
    };
    let exit_code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok((true, exit_code, stdout))
    } else if exit_code == Some(1) {
        Ok((true, exit_code, format!("No matches found for: {query}")))
    } else {
        Ok((
            false,
            exit_code,
            format!(
                "Search failed (exit {}):\n{}",
                exit_code.unwrap_or(-1),
                stderr
            ),
        ))
    }
}

async fn run_rg_search_with_bounded_broadening(
    workdir: &Path,
    target: &Path,
    query: &str,
    requested_path: &str,
    whole_word: bool,
) -> Result<(bool, Option<i32>, String, String)> {
    let (ok, code, content) = run_rg_search(workdir, target, query, whole_word).await?;
    if requested_path != "." && search_result_is_no_match(&content) {
        let (wide_ok, wide_code, wide_content) =
            run_rg_search(workdir, workdir, query, whole_word).await?;
        if !search_result_is_no_match(&wide_content) {
            return Ok((wide_ok, wide_code, wide_content, ".".to_string()));
        }
    }
    Ok((ok, code, content, requested_path.to_string()))
}

fn search_result_is_no_match(content: &str) -> bool {
    content
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("no matches found")
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
        .map(|l| {
            l.entries
                .iter()
                .map(|e| e.summary.clone())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    let narrative = format!("Generate a concise final summary (1-2 sentences max).\n\nUser request: {}\n\nEvidence:\n{}\n\nModel's draft answer:\n{}\n\nProvide a short, direct answer.", user_request, if evidence_summary.is_empty() { "(none)".to_string() } else { evidence_summary }, model_provided_content);
    match execute_intel_text_from_user_content(client, cfg, narrative).await {
        Ok(s) => Some(s),
        Err(e) => {
            trace(args, &format!("final_summary_intel failed: {}", e));
            None
        }
    }
}

async fn run_final_summary_intel_or_fallback(
    args: &Args,
    client: &reqwest::Client,
    summarizer_cfg: Option<&Profile>,
    user_request: &str,
    fallback: String,
) -> String {
    if fallback.trim().is_empty() {
        return fallback;
    }

    let Some(candidate) =
        run_final_summary_intel(args, client, summarizer_cfg, user_request, &fallback).await
    else {
        return fallback;
    };

    if final_candidate_preserves_evidence(&candidate, &fallback) {
        candidate
    } else {
        trace(
            args,
            "final_summary_intel rejected: candidate dropped concrete evidence anchors",
        );
        fallback
    }
}

fn final_candidate_preserves_evidence(candidate: &str, fallback: &str) -> bool {
    if final_answer_needs_retry(candidate) {
        return false;
    }

    let anchors = evidence_anchor_lines(fallback);
    if anchors.len() < 2 {
        return true;
    }

    let candidate_lc = candidate.to_ascii_lowercase();
    let hits = anchors
        .iter()
        .filter(|anchor| candidate_lc.contains(&anchor.to_ascii_lowercase()))
        .count();
    hits >= 2
}

fn evidence_anchor_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            line.trim()
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim()
        })
        .filter(|line| is_evidence_anchor_line(line))
        .take(12)
        .map(ToString::to_string)
        .collect()
}

fn is_evidence_anchor_line(line: &str) -> bool {
    if line.len() < 3 || line.len() > 120 {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("based on ")
        || lower.starts_with("(this is ")
        || lower.starts_with("here is ")
        || lower.starts_with("user request")
        || lower.starts_with("route:")
        || lower.starts_with("intent:")
    {
        return false;
    }
    line.chars()
        .any(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.'))
}

fn record_evidence_for_tool_result(
    tc: &ToolCall,
    result: &crate::tool_calling::ToolExecutionResult,
) {
    if tc.function.name == "respond"
        || tc.function.name == "update_todo_list"
        || tc.function.name == "tool_search"
    {
        return;
    }
    let source = match tc.function.name.as_str() {
        "shell" => {
            let cmd = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                .ok()
                .and_then(|v| v["command"].as_str().map(String::from))
                .unwrap_or_default();
            crate::evidence_ledger::EvidenceSource::Shell {
                command: cmd,
                exit_code: result.exit_code.unwrap_or(if result.ok { 0 } else { 1 }),
            }
        }
        "read" => {
            let path = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                .ok()
                .and_then(|v| v["path"].as_str().map(String::from))
                .unwrap_or_default();
            crate::evidence_ledger::EvidenceSource::Read { path }
        }
        "search" => {
            let args = serde_json::from_str::<serde_json::Value>(&tc.function.arguments).ok();
            let pattern = args
                .as_ref()
                .and_then(|v| v["pattern"].as_str().map(String::from))
                .unwrap_or_default();
            let path = args
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
        let clean = match strip_ansi_escapes::strip(result.content.as_bytes()) {
            Ok(b) => String::from_utf8_lossy(&b).to_string(),
            Err(_) => result.content.clone(),
        };
        ledger.add_entry(source, &clean);
    });
}

fn append_tool_result_messages(
    sess: &SessionPaths,
    messages: &mut Vec<ChatMessage>,
    tc: &ToolCall,
    result: &crate::tool_calling::ToolExecutionResult,
    reasoning: Option<String>,
) {
    let budgeted = apply_tool_result_budget(
        sess,
        &tc.id,
        &tc.function.name,
        &result.content,
        DEFAULT_MAX_RESULT_SIZE_CHARS,
    );
    let model_content = if result.ok
        && budgeted.content_for_model.trim().is_empty()
        && tc.function.name != "respond"
    {
        "(empty result)".to_string()
    } else {
        budgeted.content_for_model
    };
    messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: "".to_string(),
        name: None,
        tool_calls: Some(vec![tc.clone()]),
        tool_call_id: None,
        reasoning_content: reasoning,
        summarized: false,
    });
    messages.push(ChatMessage {
        role: "tool".to_string(),
        content: model_content,
        name: Some(tc.function.name.clone()),
        tool_calls: None,
        tool_call_id: Some(tc.id.clone()),
        reasoning_content: None,
        summarized: false,
    });
}

/// Generate a progressive simplification hint based on how many consecutive
/// DSL failures the model has produced. The hint escalates:
///   attempt 1: repeat the standard repair observation (no simplification yet)
///   attempt 2: ask for the simplest single command (L, R, or S)
///   attempt 3: ask for ASK verbalization to break any format loops
fn simplify_hint_for_attempt(consecutive_failures: usize) -> String {
    match consecutive_failures {
        0 | 1 => String::new(),
        2 => "Simplify: emit a single simple command: L path=\".\" depth=1 or R path=\"filename\""
            .to_string(),
        _ => "If you're stuck, use ASK reason=\"what should I do?\" to verbalize the next step."
            .to_string(),
    }
}

/// Build a grounded exhaustion answer when the DSL stop policy fires.
/// Includes user intent, stop reason, and any partial evidence gathered.
fn build_exhaustion_answer(
    original_user_request: &str,
    consecutive_failures: usize,
    messages: &[ChatMessage],
) -> String {
    if has_recent_tool_evidence(messages) {
        return build_fallback_from_recent_tool_evidence(messages);
    }
    let intent = original_user_request
        .lines()
        .next()
        .unwrap_or("unknown request")
        .chars()
        .take(200)
        .collect::<String>();
    format!(
        "I couldn't determine the right action after {} attempts. Here's what I understood: \"{intent}\". Can you rephrase or give me more direction?",
        consecutive_failures
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_search_output_short_content() {
        let content = "hello world";
        let result = truncate_search_output(content);
        assert_eq!(result, content);
    }

    #[test]
    fn truncate_search_output_long_content() {
        let long: String = "a".repeat(60_000);
        let result = truncate_search_output(&long);
        assert!(result.contains("truncated"));
        assert!(result.len() < long.len());
        assert!(result.len() > 49_000);
    }

    #[test]
    fn truncate_search_output_boundary() {
        let exact = "a".repeat(MAX_SEARCH_OUTPUT_CHARS);
        let result = truncate_search_output(&exact);
        assert_eq!(result.len(), MAX_SEARCH_OUTPUT_CHARS);
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn truncate_tool_trace_output_short() {
        let result = truncate_tool_trace_output("short");
        assert_eq!(result, "short");
    }

    #[test]
    fn truncate_tool_trace_output_long() {
        let long: String = "x".repeat(5000);
        let result = truncate_tool_trace_output(&long);
        assert!(result.contains("truncated"));
        assert!(result.len() < long.len());
    }

    #[test]
    fn final_candidate_rejects_summary_that_drops_list_evidence() {
        let fallback = "Based on the evidence gathered:\n- Cargo.toml\nsrc/\n_tasks/\nREADME.md";
        let candidate = "The directory was listed successfully.";
        assert!(!final_candidate_preserves_evidence(candidate, fallback));
    }

    #[test]
    fn final_candidate_accepts_summary_with_concrete_evidence() {
        let fallback = "Based on the evidence gathered:\n- Cargo.toml\nsrc/\n_tasks/\nREADME.md";
        let candidate = "The top-level entries include Cargo.toml, src/, _tasks/, and README.md.";
        assert!(final_candidate_preserves_evidence(candidate, fallback));
    }

    #[test]
    fn search_no_match_detector_is_exact() {
        assert!(search_result_is_no_match("No matches found for: missing"));
        assert!(!search_result_is_no_match("src/main.rs:12:match found"));
    }
}
