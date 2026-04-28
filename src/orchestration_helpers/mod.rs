//! @efficiency-role: orchestrator

use anyhow::Context;
use futures::stream::StreamExt;
use serde_json::Value;
use std::time::Duration;

use crate::claude_ui::strip_thinking_tags_preserve_spacing;
use crate::*;

mod grounding;
pub(crate) use grounding::*;

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

pub(crate) async fn request_program_or_repair(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    prompt: &str,
    use_grammar: bool,
) -> Result<(Program, String)> {
    let grammar = if use_grammar {
        Some(json_program_grammar())
    } else {
        None
    };

    let orch_req = chat_request_system_user(
        orchestrator_cfg,
        &orchestrator_cfg.system_prompt,
        prompt,
        ChatRequestOptions {
            grammar,
            ..ChatRequestOptions::default()
        },
    );
    let (program, json_text) = chat_json_with_repair_text_timeout(
        client,
        chat_url,
        &orch_req,
        orchestrator_cfg.timeout_s.min(45),
    )
    .await?;
    Ok((program, json_text))
}

pub(crate) async fn request_recovery_program(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    prompt: &str,
    failed_steps: &[StepResult], // NEW: Track failed steps to forbid repetition
) -> Result<Program> {
    // Build list of failed commands to explicitly forbid
    let failed_commands: Vec<String> = failed_steps
        .iter()
        .filter(|s| s.kind == "shell" && !s.ok)
        .filter_map(|s| s.command.clone())
        .collect();

    let failed_commands_str = if failed_commands.is_empty() {
        "None".to_string()
    } else {
        failed_commands
            .iter()
            .map(|c| format!("- {}", c))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let recovery_system = format!(
                "{}\n\nRECOVERY MODE:\n\
                - A previous workflow attempt failed or was unusable.\n\
                - Return ONLY one valid Program JSON object.\n\
                - Do not output reply-only for a non-CHAT route unless asking one concise clarifying question is the only safe next step.\n\
                - Use current_program_steps and observed_step_results to repair the workflow, not to restate or hallucinate completion.\n\
                - DO NOT repeat previously FAILED commands (see list below).\n\
                - If the task asks to choose, rank, prioritize, or select workspace items, inspect evidence first, then decide or summarize, then reply.\n\
                - Prefer the smallest valid program that can still satisfy the request.\n\n\
                PREVIOUSLY FAILED COMMANDS (DO NOT REPEAT):\n{}\n",
                orchestrator_cfg.system_prompt,
                failed_commands_str
            );
    let recovery_req = chat_request_system_user(
        orchestrator_cfg,
        &recovery_system,
        &prompt,
        ChatRequestOptions {
            temperature: Some(0.0),
            top_p: Some(1.0),
            max_tokens: Some(orchestrator_cfg.max_tokens.min(1536)),
            ..ChatRequestOptions::default()
        },
    );
    chat_json_with_repair_timeout(
        client,
        chat_url,
        &recovery_req,
        orchestrator_cfg.timeout_s.min(45),
    )
    .await
}

pub(crate) async fn request_critic_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    _line: &str,
    _route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    _sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> Result<CriticVerdict> {
    let narrative = crate::intel_narrative::build_critic_narrative(
        &program.objective,
        program,
        step_results,
        attempt,
        2, // max_retries
    );

    let critic_req = chat_request_system_user(
        critic_cfg,
        &critic_cfg.system_prompt,
        &narrative,
        ChatRequestOptions::default(),
    );
    chat_json_with_repair_for_profile_timeout(
        client,
        chat_url,
        &critic_req,
        &critic_cfg.name,
        critic_cfg.timeout_s,
    )
    .await
}

pub(crate) async fn request_reviewer_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    reviewer_cfg: &Profile,
    program: &Program,
    step_results: &[StepResult],
    review_type: &str,
) -> Result<CriticVerdict> {
    let narrative = crate::intel_narrative::build_reviewer_narrative(
        &program.objective,
        program,
        step_results,
        review_type,
    );

    let reviewer_req = chat_request_system_user(
        reviewer_cfg,
        &reviewer_cfg.system_prompt,
        &narrative,
        ChatRequestOptions::default(),
    );
    chat_json_with_repair_for_profile_timeout(
        client,
        chat_url,
        &reviewer_req,
        &reviewer_cfg.name,
        reviewer_cfg.timeout_s,
    )
    .await
}

pub(crate) async fn request_risk_review(
    client: &reqwest::Client,
    chat_url: &Url,
    risk_cfg: &Profile,
    _line: &str,
    _route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    _attempt: u32,
) -> Result<RiskReviewVerdict> {
    let narrative = crate::intel_narrative::build_reviewer_narrative(
        &program.objective,
        program,
        step_results,
        "risk",
    );

    let risk_req = chat_request_system_user(
        risk_cfg,
        &risk_cfg.system_prompt,
        &narrative,
        ChatRequestOptions::default(),
    );
    chat_json_with_repair_for_profile_timeout(
        client,
        chat_url,
        &risk_req,
        &risk_cfg.name,
        risk_cfg.timeout_s,
    )
    .await
}

pub(crate) async fn request_chat_final_text_streaming(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    system_content: &str,
    line: &str,
    reply_instructions: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<(String, Option<u64>)> {
    use crate::claude_ui::UiEvent;
    use futures::stream::StreamExt;

    let reply_user = format!(
        "User message:\n{}\n\nInstructions:\n{}\n\nRespond conversationally and directly.",
        line,
        if reply_instructions.trim().is_empty() {
            "Reply naturally and helpfully."
        } else {
            reply_instructions.trim()
        }
    );
    let reply_req = chat_request_system_user(
        elma_cfg,
        system_content,
        &reply_user,
        ChatRequestOptions {
            stream: Some(true),
            ..ChatRequestOptions::default()
        },
    );

    if let Some(ref mut tui) = tui {
        tui.handle_ui_event(UiEvent::TurnStarted);
        let _ = tui.pump_ui();
    }

    let url = chat_url.to_string();
    let response = client
        .post(url.clone())
        .json(&reply_req)
        .timeout(Duration::from_secs(elma_cfg.timeout_s))
        .send()
        .await
        .context("Streaming request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API error {}: {}", status, body);
    }

    let mut byte_stream = response.bytes_stream();
    let mut full_content = String::new();
    let mut full_thinking = String::new();
    let mut buffer = String::new();
    let mut thinking_started = false;
    let mut in_think_block = false;
    let mut pending_think_tag = String::new();

    loop {
        let chunk_result_opt = tokio::select! {
            chunk = byte_stream.next() => chunk,
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                if let Some(ref mut t) = tui {
                    let _ = t.pump_ui();
                    if let Ok(Some(queued)) = t.poll_busy_submission() {
                        t.enqueue_submission(queued);
                    }
                }
                continue;
            }
        };

        let Some(chunk_result) = chunk_result_opt else {
            break;
        };
        let chunk_bytes = match chunk_result {
            Ok(b) => b,
            Err(e) => {
                append_trace_log_line(&format!("[STREAM_ERROR] {}", e));
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

            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                    for choice in choices {
                        if let Some(delta) = choice.get("delta") {
                            // 1. Handle Thinking / Reasoning
                            let reasoning = delta
                                .get("reasoning_content")
                                .or_else(|| delta.get("reasoning"))
                                .or_else(|| delta.get("thought"))
                                .and_then(|v| v.as_str());

                            if let Some(reason) = reasoning {
                                let reason = strip_thinking_tags_preserve_spacing(reason);
                                if !reason.is_empty() {
                                    if !thinking_started {
                                        thinking_started = true;
                                        if let Some(ref mut tui) = tui {
                                            tui.handle_ui_event(UiEvent::ThinkingStarted);
                                            let _ = tui.pump_ui();
                                        }
                                    }
                                    full_thinking.push_str(&reason);
                                    if let Some(ref mut tui) = tui {
                                        tui.handle_ui_event(UiEvent::ThinkingDelta(reason));
                                        let _ = tui.pump_ui();
                                    }
                                }
                            }

                            // 2. Handle Assistant Content
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                if !content.is_empty() {
                                    let (assistant_delta, thinking_delta) =
                                        process_stream_content_chunk(
                                            content,
                                            &mut in_think_block,
                                            &mut pending_think_tag,
                                        );

                                    let thinking_delta =
                                        strip_thinking_tags_preserve_spacing(&thinking_delta);
                                    if !thinking_delta.is_empty() {
                                        if !thinking_started {
                                            thinking_started = true;
                                            if let Some(ref mut tui) = tui {
                                                tui.handle_ui_event(UiEvent::ThinkingStarted);
                                                let _ = tui.pump_ui();
                                            }
                                        }
                                        full_thinking.push_str(&thinking_delta);
                                        if let Some(ref mut tui) = tui {
                                            tui.handle_ui_event(UiEvent::ThinkingDelta(
                                                thinking_delta,
                                            ));
                                            let _ = tui.pump_ui();
                                        }
                                    }

                                    if !assistant_delta.is_empty()
                                        && thinking_started
                                        && !in_think_block
                                    {
                                        thinking_started = false;
                                        if let Some(ref mut tui) = tui {
                                            tui.handle_ui_event(UiEvent::ThinkingFinished);
                                            let _ = tui.pump_ui();
                                        }
                                    }

                                    if !assistant_delta.is_empty() {
                                        full_content.push_str(&assistant_delta);
                                        if let Some(ref mut tui) = tui {
                                            tui.handle_ui_event(UiEvent::AssistantContentDelta(
                                                assistant_delta,
                                            ));
                                            let _ = tui.pump_ui();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if thinking_started {
        if let Some(ref mut tui) = tui {
            tui.handle_ui_event(UiEvent::ThinkingFinished);
            let _ = tui.pump_ui();
        }
    }

    if let Some(ref mut tui) = tui {
        tui.handle_ui_event(UiEvent::AssistantFinished);
        let _ = tui.pump_ui();
    }

    Ok((full_content.trim().to_string(), None))
}

pub(crate) async fn request_chat_final_text(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    system_content: &str,
    line: &str,
    step_results: &[StepResult],
    reply_instructions: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<(String, Option<u64>)> {
    // Try streaming first
    let stream_tui = tui.as_deref_mut();
    match request_chat_final_text_streaming(
        client,
        chat_url,
        elma_cfg,
        system_content,
        line,
        reply_instructions,
        stream_tui,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(e) => {
            append_trace_log_line(&format!("[STREAM_FALLBACK] {}", e));
            if let Some(ref mut t) = tui {
                t.handle_ui_event(crate::claude_ui::UiEvent::ThinkingStarted);
                let _ = t.pump_ui();
            }
            // Fallback to non-streaming
            let reply_user = format!(
                "User message:\n{}\n\nInstructions:\n{}\n\nRespond conversationally.",
                line,
                if reply_instructions.trim().is_empty() {
                    "Reply naturally."
                } else {
                    reply_instructions.trim()
                }
            );
            let reply_req = chat_request_system_user(
                elma_cfg,
                system_content,
                &reply_user,
                ChatRequestOptions::default(),
            );
            let parsed = {
                let fut = chat_once_with_timeout(client, chat_url, &reply_req, elma_cfg.timeout_s);
                tokio::pin!(fut);
                loop {
                    tokio::select! {
                        r = &mut fut => break r?,
                        _ = tokio::time::sleep(Duration::from_millis(40)) => {
                            if let Some(ref mut t) = tui {
                                let _ = t.pump_ui();
                                if let Ok(Some(queued)) = t.poll_busy_submission() {
                                    t.enqueue_submission(queued);
                                }
                            }
                        }
                    }
                }
            };
            let usage_total = parsed.usage.as_ref().and_then(|u| u.total_tokens);
            let msg = &parsed.choices.get(0).context("No choices[0]")?.message;
            let extraction = crate::thinking_content::extract_thinking(
                msg.content.as_deref(),
                msg.reasoning_content.as_deref(),
            );
            let thinking = extraction.thinking.unwrap_or_default();
            let content = extraction.final_answer.trim().to_string();

            if let Some(ref mut t) = tui {
                if !thinking.is_empty() {
                    t.handle_ui_event(crate::claude_ui::UiEvent::ThinkingDelta(thinking));
                    let _ = t.pump_ui();
                }
                t.handle_ui_event(crate::claude_ui::UiEvent::ThinkingFinished);
                let _ = t.pump_ui();
                if !content.is_empty() {
                    t.handle_ui_event(crate::claude_ui::UiEvent::AssistantContentDelta(
                        content.clone(),
                    ));
                    let _ = t.pump_ui();
                }
                t.handle_ui_event(crate::claude_ui::UiEvent::AssistantFinished);
                let _ = t.pump_ui();
            }

            // Return None for usage to avoid overwriting cumulative estimate with per-request tokens
            Ok((content, None))
        }
    }
}

pub(crate) async fn maybe_revise_presented_result(
    client: &reqwest::Client,
    chat_url: &Url,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    runtime_context: &Value,
    evidence_mode: &EvidenceModeDecision,
    response_advice: &ExpertAdvisorAdvice,
    step_results: &[StepResult],
    reply_instructions: &str,
    final_text: String,
    workspace_facts: &str,
    workspace_brief: &str,
) -> String {
    if let Ok(verdict) = claim_check_once(
        client,
        chat_url,
        claim_checker_cfg,
        line,
        evidence_mode,
        step_results,
        &final_text,
    )
    .await
    {
        if verdict.status.eq_ignore_ascii_case("revise") {
            let revised = present_result_via_unit(
                client,
                presenter_cfg,
                line,
                route_decision,
                runtime_context,
                evidence_mode,
                response_advice,
                step_results,
                &format!(
                    "{}\n\nRevision guidance:\n{}",
                    reply_instructions,
                    if verdict.rewrite_instructions.trim().is_empty() {
                        verdict.reason.trim()
                    } else {
                        verdict.rewrite_instructions.trim()
                    }
                ),
                workspace_facts,
                workspace_brief,
            )
            .await
            .unwrap_or_default();
            if !revised.trim().is_empty() {
                return revised;
            }
        }
    }
    final_text
}

pub(crate) async fn decide_evidence_mode_via_unit(
    client: &reqwest::Client,
    evidence_mode_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
    workspace_facts: &str,
    workspace_brief: &str,
) -> Result<EvidenceModeDecision> {
    let has_command_request = user_message
        .to_lowercase()
        .split_whitespace()
        .any(|w| ["run", "execute", "show", "display", "print"].contains(&w));
    let has_command_execution = step_results
        .iter()
        .any(|s| s.command.as_ref().is_some_and(|c| !c.is_empty()));
    let has_artifact = step_results
        .iter()
        .any(|s| s.artifact_path.as_ref().is_some_and(|p| !p.is_empty()));

    let narrative = crate::intel_narrative::build_evidence_mode_narrative(
        user_message,
        route_decision,
        reply_instructions,
        step_results,
        has_command_request,
        has_command_execution,
        has_artifact,
    );

    let unit = EvidenceModeUnit::new(evidence_mode_cfg.clone());
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        workspace_facts.to_string(),
        workspace_brief.to_string(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("narrative", narrative)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse evidence mode decision: {}", e))
}

pub(crate) async fn request_response_advice_via_unit(
    client: &reqwest::Client,
    expert_advisor_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
    workspace_facts: &str,
    workspace_brief: &str,
) -> Result<ExpertAdvisorAdvice> {
    let unit = ExpertAdvisorUnit::new(expert_advisor_cfg.clone());
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        workspace_facts.to_string(),
        workspace_brief.to_string(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("evidence_mode", evidence_mode)?
    .with_extra(
        "step_results",
        step_results
            .iter()
            .map(step_result_json)
            .collect::<Vec<_>>(),
    )?
    .with_extra("reply_instructions", reply_instructions)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse expert responder advice: {}", e))
}

pub(crate) async fn present_result_via_unit(
    client: &reqwest::Client,
    presenter_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    runtime_context: &Value,
    evidence_mode: &EvidenceModeDecision,
    response_advice: &ExpertAdvisorAdvice,
    step_results: &[StepResult],
    reply_instructions: &str,
    workspace_facts: &str,
    workspace_brief: &str,
) -> Result<String> {
    let unit = ResultPresenterUnit::new(presenter_cfg.clone());
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        workspace_facts.to_string(),
        workspace_brief.to_string(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("runtime_context", runtime_context)?
    .with_extra("evidence_mode", evidence_mode)?
    .with_extra("response_advice", response_advice)?
    .with_extra(
        "step_results",
        step_results
            .iter()
            .map(step_result_json)
            .collect::<Vec<_>>(),
    )?
    .with_extra("reply_instructions", reply_instructions)?;
    let output = unit.execute_with_fallback(&context).await?;
    let final_text = preserve_exact_grounded_path(
        output.get_str("final_text").unwrap_or_default().to_string(),
        step_results,
        reply_instructions,
    );
    Ok(preserve_requested_summary_and_entry_point(
        final_text,
        step_results,
        reply_instructions,
    ))
}

pub(crate) async fn maybe_format_final_text(
    client: &reqwest::Client,
    _chat_url: &Url,
    formatter_cfg: &Profile,
    line: &str,
    final_text: String,
    usage_total: Option<u64>,
) -> (String, Option<u64>) {
    // If the user explicitly asked for markdown, preserve it as-is
    if user_requested_markdown(line) {
        return (final_text, usage_total);
    }

    // Try automated plain-text transformation first
    let plain_text = plain_terminal_text(&final_text);

    let already_terminal_ready = plain_text.lines().count() <= 8
        || plain_text.contains("Entry point:")
        || plain_text
            .lines()
            .any(|line| line.trim_start().starts_with("- "))
        || plain_text.contains("_stress_testing/");
    if already_terminal_ready {
        return (plain_text, usage_total);
    }

    // If the current config has a managed formatter, use it for final cleaning
    let unit = FormatterUnit::new(formatter_cfg.clone());
    let context = IntelContext::new(
        plain_text.clone(),
        RouteDecision::default(), // Dummy as formatter doesn't need it
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    );

    match unit.execute_with_fallback(&context).await {
        Ok(output) => (
            output
                .get_str("formatted_text")
                .unwrap_or(&plain_text)
                .to_string(),
            usage_total, // Note: Usage tracking is omitted for now for simplicity, as it's a minor call
        ),
        _ => (plain_text, usage_total),
    }
}

#[cfg(test)]
mod tests {
    use super::process_stream_content_chunk;

    #[test]
    fn splits_inline_think_tags_into_thinking_stream() {
        let mut in_think = false;
        let mut pending = String::new();

        let (assistant, thinking) =
            process_stream_content_chunk("A<think>B</think>C", &mut in_think, &mut pending);

        assert_eq!(assistant, "AC");
        assert_eq!(thinking, "B");
        assert!(!in_think);
        assert!(pending.is_empty());
    }

    #[test]
    fn handles_think_tags_split_across_stream_chunks() {
        let mut in_think = false;
        let mut pending = String::new();

        let (assistant_1, thinking_1) =
            process_stream_content_chunk("Hi <thi", &mut in_think, &mut pending);
        assert_eq!(assistant_1, "Hi ");
        assert_eq!(thinking_1, "");
        assert_eq!(pending, "<thi");

        let (assistant_2, thinking_2) =
            process_stream_content_chunk("nk>plan</think> done", &mut in_think, &mut pending);
        assert_eq!(assistant_2, " done");
        assert_eq!(thinking_2, "plan");
        assert!(!in_think);
        assert!(pending.is_empty());
    }

    #[test]
    fn supports_alternative_reasoning_tags_in_stream_chunks() {
        let mut in_think = false;
        let mut pending = String::new();

        let (assistant, thinking) = process_stream_content_chunk(
            "X<thinking>Y</thinking><reasoning>Z</reasoning>W",
            &mut in_think,
            &mut pending,
        );
        assert_eq!(assistant, "XW");
        assert_eq!(thinking, "YZ");
        assert!(!in_think);
        assert!(pending.is_empty());
    }
}
