use crate::*;
use std::collections::BTreeMap;
use std::time::Duration;
use futures::stream::StreamExt;
use crate::session_display::save_thinking_display;
use crate::ui_trace::append_trace_log_line;

const THINK_OPEN_TAG: &str = "<think>";
const THINK_CLOSE_TAG: &str = "</think>";
const THINKING_OPEN_TAG: &str = "<thinking>";
const THINKING_CLOSE_TAG: &str = "</thinking>";
const REASONING_OPEN_TAG: &str = "<reasoning>";
const REASONING_CLOSE_TAG: &str = "</reasoning>";
const THOUGHT_OPEN_TAG: &str = "<thought>";
const THOUGHT_CLOSE_TAG: &str = "</thought>";

fn match_reasoning_open(rest: &str) -> Option<usize> {
    [
        THINK_OPEN_TAG,
        THINKING_OPEN_TAG,
        REASONING_OPEN_TAG,
        THOUGHT_OPEN_TAG,
    ]
    .into_iter()
    .find_map(|tag| rest.starts_with(tag).then_some(tag.len()))
}

fn match_reasoning_close(rest: &str) -> Option<usize> {
    [
        THINK_CLOSE_TAG,
        THINKING_CLOSE_TAG,
        REASONING_CLOSE_TAG,
        THOUGHT_CLOSE_TAG,
    ]
    .into_iter()
    .find_map(|tag| rest.starts_with(tag).then_some(tag.len()))
}

fn has_reasoning_tag_prefix(rest: &str) -> bool {
    [
        THINK_OPEN_TAG,
        THINK_CLOSE_TAG,
        THINKING_OPEN_TAG,
        THINKING_CLOSE_TAG,
        REASONING_OPEN_TAG,
        REASONING_CLOSE_TAG,
        THOUGHT_OPEN_TAG,
        THOUGHT_CLOSE_TAG,
    ]
    .into_iter()
    .any(|tag| tag.starts_with(rest))
}

pub(crate) fn process_stream_content_chunk(
    chunk: &str,
    in_think_block: &mut bool,
    pending_tag: &mut String,
) -> (String, String) {
    let mut input = String::with_capacity(pending_tag.len() + chunk.len());
    input.push_str(pending_tag);
    input.push_str(chunk);
    pending_tag.clear();

    let mut assistant = String::new();
    let mut thinking = String::new();
    let mut i = 0usize;

    while i < input.len() {
        let rest = &input[i..];
        let Some(rel_lt) = rest.find('<') else {
            if *in_think_block {
                thinking.push_str(rest);
            } else {
                assistant.push_str(rest);
            }
            break;
        };

        if rel_lt > 0 {
            let before = &rest[..rel_lt];
            if *in_think_block {
                thinking.push_str(before);
            } else {
                assistant.push_str(before);
            }
            i += rel_lt;
        }

        let rest = &input[i..];
        if let Some(tag_len) = match_reasoning_open(rest) {
            *in_think_block = true;
            i += tag_len;
            continue;
        }
        if let Some(tag_len) = match_reasoning_close(rest) {
            *in_think_block = false;
            i += tag_len;
            continue;
        }

        if has_reasoning_tag_prefix(rest) {
            pending_tag.push_str(rest);
            break;
        }

        if *in_think_block {
            thinking.push('<');
        } else {
            assistant.push('<');
        }
        i += 1;
    }

    (assistant, thinking)
}

pub(crate) struct ToolLoopModelTurn {
    pub content: String,
    pub content_raw: String,
    pub tool_calls: Vec<ToolCall>,
    pub reasoning_content: Option<String>,
    pub thinking_content: String,
}

#[derive(Default)]
pub(crate) struct StreamingToolCallPart {
    pub id: Option<String>,
    pub call_type: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}

pub(crate) fn append_streaming_tool_call_delta(
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

pub(crate) fn finish_streaming_tool_calls(parts: BTreeMap<usize, StreamingToolCallPart>) -> Vec<ToolCall> {
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

pub(crate) async fn request_tool_loop_model_turn_streaming(
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
    let mut content_raw = String::new(); // Raw content with think tags preserved
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
                    content_raw.push_str(raw_content);
                    let (assistant_delta, thinking_delta) =
                        process_stream_content_chunk(
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
        let _ = tui.pump_ui();
    }
    if content_started {
        tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantFinished { is_ephemeral: true });
        let _ = tui.pump_ui();
    }

    let captured_thinking = std::mem::take(&mut thinking_accumulated);

    Ok(ToolLoopModelTurn {
        content: content.trim().to_string(),
        content_raw: content_raw.trim().to_string(),
        tool_calls: finish_streaming_tool_calls(tool_call_parts),
        reasoning_content: if reasoning_content_full.is_empty() {
            None
        } else {
            Some(reasoning_content_full)
        },
        thinking_content: captured_thinking,
    })
}

pub(crate) async fn request_tool_loop_final_answer_streaming(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    req: ChatCompletionRequest,
    timeout_s: u64,
) -> Result<String> {
    let input_estimate: usize = req
        .messages
        .iter()
        .map(|m| crate::token_counter::count_tokens(&m.content))
        .sum::<usize>()
        .max(1);
    tui.update_input_tokens(input_estimate);
    let mut req = req;
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
    let mut in_think_block = false;
    let mut pending_think_tag = String::new();
    let mut content_started = false;
    let mut thinking_started = false;

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
                let reasoning = delta
                    .get("reasoning_content")
                    .or_else(|| delta.get("reasoning"))
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
                        reasoning.to_string(),
                    ));
                    let _ = tui.pump_ui();
                }

                if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                    content.push_str(text);
                    let (assistant_delta, thinking_delta) =
                        process_stream_content_chunk(
                            text,
                            &mut in_think_block,
                            &mut pending_think_tag,
                        );
                    if !thinking_delta.is_empty() {
                        if !thinking_started {
                            thinking_started = true;
                            tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingStarted);
                            let _ = tui.pump_ui();
                        }
                        tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingDelta(
                            thinking_delta,
                        ));
                        let _ = tui.pump_ui();
                    }
                    if !assistant_delta.is_empty() {
                        if thinking_started && !in_think_block {
                            thinking_started = false;
                            tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingFinished);
                            let _ = tui.pump_ui();
                        }
                        if !content_started {
                            content_started = true;
                        }
                        tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantContentDelta(
                            assistant_delta,
                        ));
                        let _ = tui.pump_ui();
                    }
                }
            }
        }
    }

    if !pending_think_tag.is_empty() {
        let (assistant_delta, thinking_delta) =
            process_stream_content_chunk(
                "",
                &mut in_think_block,
                &mut pending_think_tag,
            );
        if !assistant_delta.is_empty() {
            content.push_str(&assistant_delta);
        }
        if !thinking_delta.is_empty() {
            if !thinking_started {
                thinking_started = true;
                tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingStarted);
            }
            tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingDelta(thinking_delta));
        }
    }

    if thinking_started {
        tui.handle_ui_event(crate::claude_ui::UiEvent::ThinkingFinished);
        let _ = tui.pump_ui();
    }
    if content_started {
        tui.handle_ui_event(crate::claude_ui::UiEvent::AssistantFinished { is_ephemeral: false });
        let _ = tui.pump_ui();
    }

    let cleaned = crate::text_utils::strip_thinking_blocks(&content);
    Ok(cleaned.trim().to_string())
}
