//! @efficiency-role: domain-logic
//! Tool Loop — continuous execution loop using native tool calling.

use crate::auto_compact::{
    apply_compact, CompactTracker, DEFAULT_COMPACT_BUFFER_TOKENS, DEFAULT_CONTEXT_WINDOW_TOKENS,
};
use crate::tool_calling::build_tool_definitions;
use crate::tool_result_storage::{apply_tool_result_budget, DEFAULT_MAX_RESULT_SIZE_CHARS};
use crate::ui_state::{
    get_total_intel_failures, increment_intel_failure_count, reset_intel_failure_counts,
};
use crate::*;
use std::future::Future;
use std::time::Duration;

const MAX_TOOL_ITERATIONS: usize = 15;

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

pub(crate) struct ToolLoopResult {
    pub(crate) final_answer: String,
    pub(crate) iterations: usize,
    pub(crate) tool_calls_made: usize,
    pub(crate) stopped_by_max: bool,
}

pub(crate) async fn run_tool_loop(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    system_prompt: &str,
    user_message: &str,
    workdir: &PathBuf,
    session: &SessionPaths,
    temperature: f64,
    max_tokens: u32,
    tui: &mut crate::ui_terminal::TerminalUI,
) -> Result<ToolLoopResult> {
    let tools = build_tool_definitions(workdir);
    trace(
        args,
        &format!("tool_loop: starting max_iterations={}", MAX_TOOL_ITERATIONS),
    );
    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage::simple("system", system_prompt),
        ChatMessage::simple("user", user_message),
    ];
    let mut total_tool_calls = 0;
    let mut tracker = CompactTracker::new();

    for iteration in 0..MAX_TOOL_ITERATIONS {
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
            let (new_messages, result) = apply_compact(&messages, 3); // Keep last 3 turns
            if result.ok {
                let before_count = messages.len();
                messages = new_messages;
                tracker.record_success();
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
                iteration + 1,
                MAX_TOOL_ITERATIONS
            ),
        );
        let req = ChatCompletionRequest {
            model: model_id.to_string(),
            messages: messages.clone(),
            temperature,
            top_p: 1.0,
            stream: false,
            max_tokens,
            n_probs: None,
            repeat_penalty: None,
            reasoning_format: Some("none".to_string()),
            grammar: None,
            tools: Some(tools.clone()),
        };
        let resp = await_with_busy_input(
            tui,
            crate::ui_chat::chat_once_with_timeout(client, chat_url, &req, 120),
        )
        .await?;
        let choice = resp.choices.get(0).context("No choices in response")?;
        let content = choice.message.content.clone().unwrap_or_default();
        if let Some(tool_calls) = &choice.message.tool_calls {
            if !tool_calls.is_empty() {
                trace(
                    args,
                    &format!("tool_loop: {} tool call(s)", tool_calls.len()),
                );
                total_tool_calls += tool_calls.len();
                for tc in tool_calls {
                    let result = tool_calling::execute_tool_call(
                        args,
                        tc,
                        workdir,
                        session,
                        client,
                        chat_url,
                        user_message,
                        Some(&mut *tui),
                    )
                    .await;

                    // respond tool = final answer, exit the loop immediately (only if non-empty)
                    if tc.function.name == "respond" {
                        let trimmed = result.content.trim().to_string();
                        if trimmed.is_empty() {
                            trace(
                                args,
                                "tool_loop: respond returned empty answer; continuing loop",
                            );
                        } else {
                            return Ok(ToolLoopResult {
                                final_answer: trimmed,
                                iterations: iteration + 1,
                                tool_calls_made: total_tool_calls,
                                stopped_by_max: false,
                            });
                        }
                    }

                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: "".to_string(),
                        name: None,
                        tool_calls: Some(vec![tc.clone()]),
                        tool_call_id: None,
                    });
                    // Apply result budget — persist large outputs to disk, keep inline for small ones
                    let budgeted = apply_tool_result_budget(
                        session,
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
                    });
                }
                continue;
            }
        }
        if !content.trim().is_empty() {
            return Ok(ToolLoopResult {
                final_answer: content.trim().to_string(),
                iterations: iteration + 1,
                tool_calls_made: total_tool_calls,
                stopped_by_max: false,
            });
        }
    }
    trace(args, "tool_loop: max iterations reached");
    messages.push(ChatMessage::simple(
        "user",
        "You've reached the maximum number of tool calls. Please provide your final answer.",
    ));
    let final_req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: messages.clone(),
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
    let final_resp = await_with_busy_input(
        tui,
        crate::ui_chat::chat_once_with_timeout(client, chat_url, &final_req, 60),
    )
    .await?;
    let mut final_content = final_resp
        .choices
        .get(0)
        .map(|c| c.message.content.clone().unwrap_or_default())
        .unwrap_or_else(|| "Maximum iterations reached.".to_string());
    if final_content.trim().is_empty() {
        final_content =
            "I completed the tool loop but could not generate a final summary text.".to_string();
    }
    Ok(ToolLoopResult {
        final_answer: final_content.trim().to_string(),
        iterations: MAX_TOOL_ITERATIONS + 1,
        tool_calls_made: total_tool_calls,
        stopped_by_max: true,
    })
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
