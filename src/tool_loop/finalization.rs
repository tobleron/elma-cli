use crate::*;
use std::path::{Path, PathBuf};
use crate::llm_config::{ad_hoc_profile, chat_request_from_profile, ChatRequestOptions};
use crate::ui_trace::trace;
use crate::stop_policy::StopReason;
use crate::tool_loop::streaming::request_tool_loop_final_answer_streaming;
use crate::tool_loop::await_with_busy_input;
use std::collections::HashSet;

pub(crate) const FINAL_EVIDENCE_MAX_ITEMS: usize = 12;
pub(crate) const FINAL_EVIDENCE_ITEM_MAX_CHARS: usize = 3_000;
pub(crate) const FINAL_EVIDENCE_TOTAL_MAX_CHARS: usize = 24_000;

pub(crate) fn truncate_chars(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    let omitted = chars.count();
    if omitted == 0 {
        input.to_string()
    } else {
        format!("{truncated}\n[... {omitted} chars omitted from finalization evidence ...]")
    }
}

pub(crate) fn normalize_final_answer_candidate(text: &str) -> String {
    crate::text_utils::strip_thinking_blocks(text).trim().to_string()
}

pub(crate) fn final_answer_needs_retry(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.is_empty() 
        || crate::tool_loop::is_tool_call_markup(trimmed) 
        || crate::tool_loop::is_intent_only_response(trimmed)
}

pub(crate) fn build_evidence_progress_summary(messages: &[ChatMessage]) -> Option<String> {
    let mut facts = Vec::new();
    for msg in messages.iter().rev() {
        if msg.role != "tool" {
            continue;
        }
        let line = msg
            .content
            .lines()
            .next()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty());
        if let Some(l) = line {
            facts.push(l);
            if facts.len() >= 5 {
                break;
            }
        }
    }
    facts.reverse();
    if facts.is_empty() {
        None
    } else {
        Some(format!(
            "[Previously gathered evidence]\nYou already gathered the following information in a prior attempt:\n{}\n\nDo NOT repeat these steps. Continue from where you left off.",
            facts
                .iter()
                .map(|f| format!("  • {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

pub(crate) fn build_fallback_from_recent_tool_evidence(
    messages: &[ChatMessage],
    stop_reason: Option<&StopReason>,
) -> String {
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

    let budget_exhausted = matches!(
        stop_reason,
        Some(
            StopReason::IterationLimitReached
                | StopReason::StageBudgetExceeded
                | StopReason::TaskBudgetExceeded
                | StopReason::WallClockExceeded
        )
    );

    if budget_exhausted && facts.is_empty() {
        "I didn't complete this task — the iteration budget was exhausted before any tool calls completed. Try rephrasing with a narrower scope, or increase the complexity tier.".to_string()
    } else if facts.is_empty() {
        "I couldn't produce a reliable answer. No evidence was gathered.".to_string()
    } else {
        format!(
            "[I found the following information, but the answer could not be finalized. Here's what I know:]\n{}\n",
            facts
                .iter()
                .map(|f| format!("- {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

pub(crate) fn build_bounded_final_evidence(messages: &[ChatMessage]) -> String {
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

pub(crate) async fn request_final_answer_from_evidence(
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    original_user_request: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
) -> Result<String> {
    let evidence_block = build_bounded_final_evidence(messages);

    let clean_messages = vec![ChatMessage::simple(
        "user",
        &format!(
            "{}\n\n--- Evidence gathered so far ---\n{}\n--- End evidence ---\n\nAnswer in a natural conversational tone. Use complete sentences. Acknowledge what was found or done. Ground your answer only in the evidence above. Use clean terminal-friendly formatting. Prefer simple lists and short sections over walls of text. Do not call tools.",
            original_user_request, evidence_block
        ),
    )];

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
    request_tool_loop_final_answer_streaming(
        tui,
        client,
        chat_url,
        req,
        runtime_llm_config().final_answer_timeout_s,
    )
    .await
}

pub(crate) async fn finalize_from_evidence_or_fallback(
    args: &Args,
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    original_user_request: &str,
    messages: &[ChatMessage],
    workdir: &Path,
    max_tokens: u32,
    stop_reason: Option<&StopReason>,
) -> String {
    let missing_artifacts = crate::artifact_verifier::find_missing_artifacts(workdir);
    if !missing_artifacts.is_empty() {
        trace(
            args,
            &format!(
                "finalization_stage=artifact_synthesis count={}",
                missing_artifacts.len()
            ),
        );
        synthesize_missing_artifacts(
            args,
            tui,
            client,
            chat_url,
            model_id,
            messages,
            workdir,
            &missing_artifacts,
            stop_reason,
        )
        .await;
    }

    let required_artifacts = crate::artifact_verifier::get_required_artifacts();
    let all_complete = crate::artifact_verifier::are_all_artifacts_complete(workdir);
    let missing_after = crate::artifact_verifier::find_missing_artifacts(workdir);
    if !required_artifacts.is_empty() && all_complete {
        trace(args, "finalization_stage=deterministic_artifact_completion");
        return build_required_artifact_completion_answer(&required_artifacts);
    }
    if !required_artifacts.is_empty() && missing_after.is_empty() && !all_complete {
        trace(
            args,
            "finalization_stage=partial_artifact_completion artifact_state=partial_evidence_recovery",
        );
        return build_partial_artifact_completion_answer(&required_artifacts, workdir);
    }

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
            let compact_packet = crate::turn_context_packet::build_turn_context_packet(
                original_user_request,
                "Provide final answer",
                &[],
                &[],
                &[],
                "finalization_retry",
            );
            let packet_msg =
                crate::turn_context_packet::render_turn_context_packet(&compact_packet);
            let compact_prompt = format!(
                "Provide a final answer based on the evidence gathered.\n\n{}",
                packet_msg
            );
            let mut retry_msgs = messages.to_vec();
            retry_msgs.push(ChatMessage::simple("user", &compact_prompt));
            match request_final_answer_without_tools(
                tui,
                client,
                chat_url,
                model_id,
                &retry_msgs,
                max_tokens,
                true,
            )
            .await
            {
                Ok(content) => content,
                Err(e2) => {
                    trace(
                        args,
                        &format!("finalization_failed_nonfatal stage=retry error={}", e2),
                    );
                    build_fallback_from_recent_tool_evidence(messages, stop_reason)
                }
            }
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
                build_fallback_from_recent_tool_evidence(messages, stop_reason)
            }
        };
    }

    let verified =
        crate::finalization_verifier::verify_final_answer(&final_content, messages, workdir);
    let mut verified = verified;
    persist_missing_required_artifacts(args, workdir, &verified).await;
    let missing = crate::artifact_verifier::find_missing_artifacts(workdir);
    if !missing.is_empty() {
        trace(
            args,
            &format!(
                "finalization_missing_artifacts count={} paths={}",
                missing.len(),
                missing
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        );
        verified.push_str(&crate::artifact_verifier::build_missing_artifact_notice(
            &missing,
        ));
    }

    verified
}

pub(crate) fn build_required_artifact_completion_answer(required_artifacts: &[String]) -> String {
    let mut answer =
        String::from("Completed the requested artifact work.\n\nCreated or updated:\n");
    for artifact in required_artifacts {
        answer.push_str(&format!("- `{}`\n", artifact));
    }
    answer
}

pub(crate) fn build_partial_artifact_completion_answer(
    required_artifacts: &[String],
    workdir: &Path,
) -> String {
    let mut answer = String::from(
        "Partial completion: Some deliverables exist but could not be fully substantiated.\n\n",
    );
    for artifact in required_artifacts {
        let full_path = workdir.join(artifact);
        if !full_path.exists() {
            answer.push_str(&format!("- `{}` (missing)\n", artifact));
        } else if crate::artifact_verifier::is_evidence_recovery_file(&full_path) {
            answer.push_str(&format!(
                "- `{}` (evidence recovery — contains raw tool output, not a substantive report)\n",
                artifact
            ));
        } else if crate::artifact_verifier::is_empty_file(&full_path) {
            answer.push_str(&format!("- `{}` (empty)\n", artifact));
        } else {
            answer.push_str(&format!("- `{}`\n", artifact));
        }
    }
    answer.push_str(
        "\nNote: The tool-call budget was exhausted before all artifacts could be fully completed. \
         The deliverables above marked as 'evidence recovery' contain collected tool evidence \
         rather than a synthesized report.",
    );
    answer
}

pub(crate) async fn synthesize_missing_artifacts(
    args: &Args,
    tui: &mut crate::ui_terminal::TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    messages: &[ChatMessage],
    workdir: &Path,
    missing: &[(String, PathBuf)],
    stop_reason: Option<&StopReason>,
) {
    for (artifact_name, full_path) in missing {
        if full_path.exists() {
            crate::artifact_verifier::mark_artifact_verified(artifact_name);
            continue;
        }
        trace(
            args,
            &format!("artifact_synth_attempt path={}", artifact_name),
        );
        if let Some(parent) = full_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                trace(
                    args,
                    &format!("artifact_synth_failed mkdir {}: {}", parent.display(), e),
                );
                continue;
            }
        }
        let synth_prompt = format!(
            "Write the content for file `{}`. Use the tool evidence from the conversation.\n\
             Output ONLY the file content — no explanations, no markdown wrappers.",
            artifact_name
        );
        let mut synth_msgs: Vec<ChatMessage> = messages
            .iter()
            .filter(|m| m.role == "user" || m.role == "tool")
            .cloned()
            .collect();
        if let Some(first) = messages.first() {
            synth_msgs.insert(0, first.clone());
        }
        synth_msgs.push(ChatMessage::simple("user", &synth_prompt));

        let profile = crate::llm_config::ad_hoc_profile(model_id, "artifact_synthesis");
        let req = crate::llm_config::chat_request_from_profile(
            &profile,
            synth_msgs,
            crate::llm_config::ChatRequestOptions {
                max_tokens: Some(2048),
                stream: Some(false),
                temperature: Some(0.1),
                ..Default::default()
            },
        );
        let mut wrote_artifact = false;
        crate::ui_trace::append_trace_log_line("[ARTIFACT_SYNTH] retry_budget=1 max_attempts=1");
        match crate::ui::ui_chat::chat_once_with_timeout_single(client, chat_url, &req, 15).await {
            Ok(resp) => {
                if let Some(choice) = resp.choices.get(0) {
                    if let Some(ref content) = choice.message.content {
                        let stripped = crate::text_utils::strip_thinking_blocks(content);
                        let (sanitized, _) = crate::final_answer::sanitize_final_answer(&stripped);
                        if !sanitized.trim().is_empty() {
                            if let Err(e) = tokio::fs::write(full_path, sanitized.trim()).await {
                                trace(
                                    args,
                                    &format!(
                                        "artifact_synth_failed write {}: {}",
                                        artifact_name, e
                                    ),
                                );
                            } else {
                                trace(
                                    args,
                                    &format!("artifact_synthesized path={}", artifact_name),
                                );
                                crate::artifact_verifier::mark_artifact_verified(artifact_name);
                                wrote_artifact = true;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                trace(
                    args,
                    &format!("artifact_synth_failed api {}: {}", artifact_name, e),
                );
            }
        }
        if !wrote_artifact {
            crate::ui_trace::append_trace_log_line(&format!(
                "[ARTIFACT_SYNTH] fallback_reason=model_failed path={} artifact_state=evidence_recovery",
                artifact_name
            ));
            let fallback =
                build_artifact_fallback_from_tool_evidence(artifact_name, messages, stop_reason);
            if let Err(e) = tokio::fs::write(full_path, fallback.trim()).await {
                trace(
                    args,
                    &format!(
                        "artifact_synth_failed write {}: {} artifact_state=not_completed",
                        artifact_name, e
                    ),
                );
            } else {
                trace(
                    args,
                    &format!(
                        "artifact_synth_fallback_written path={} artifact_state=evidence_recovery",
                        artifact_name
                    ),
                );
                crate::artifact_verifier::mark_artifact_verified(artifact_name);
            }
        } else {
            crate::ui_trace::append_trace_log_line(&format!(
                "[ARTIFACT_SYNTH] path={} artifact_state=model_authored",
                artifact_name
            ));
        }
    }
}


pub(crate) fn build_artifact_fallback_from_tool_evidence(
    artifact_name: &str,
    messages: &[ChatMessage],
    stop_reason: Option<&StopReason>,
) -> String {
    let user_objective = messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| m.content.chars().take(500).collect::<String>())
        .unwrap_or_default();

    let evidence: Vec<(String, String)> = messages
        .iter()
        .rev()
        .filter(|m| m.role == "tool")
        .take(12)
        .filter_map(|m| {
            let name = m.name.as_deref().unwrap_or("tool").to_string();
            let preview = m.content.chars().take(2000).collect::<String>();
            Some((name, preview.trim().to_string()))
        })
        .collect();

    let stop_reason_note = match stop_reason {
        Some(reason) => format!("Stop reason: `{}`", reason.as_str()),
        None => "Stop reason: unknown".to_string(),
    };

    let file_refs: Vec<String> = messages
        .iter()
        .filter(|m| m.role == "tool")
        .flat_map(|m| {
            m.content
                .lines()
                .filter(|l| {
                    let t = l.trim();
                    !t.is_empty()
                        && (t.contains('/')
                            || t.contains(".rs")
                            || t.contains(".md")
                            || t.contains(".toml")
                            || t.contains(".json")
                            || t.contains(".py"))
                        && (t.chars().filter(|&c| c == '/').count() == 1
                            || t.starts_with("src/")
                            || t.starts_with("tests/")
                            || t.starts_with("config/")
                            || t.starts_with("docs/"))
                        && !t.starts_with("error")
                        && !t.starts_with("[")
                        && !t.starts_with("Tool result")
                })
                .take(10)
                .map(|l| l.trim().to_string())
        })
        .collect::<Vec<_>>();

    let evidence_block = if evidence.is_empty() {
        "No tool evidence was available when this fallback artifact was generated.".to_string()
    } else {
        evidence
            .iter()
            .map(|(name, content)| format!("## Tool Evidence: {}\n\n{}", name, content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    };

    let files_section = if file_refs.is_empty() {
        String::new()
    } else {
        let mut seen: Vec<&str> = Vec::new();
        let unique: Vec<&str> = file_refs
            .iter()
            .map(|s| s.as_str())
            .filter(|p| {
                if seen.contains(p) {
                    false
                } else {
                    seen.push(p);
                    true
                }
            })
            .collect();
        format!(
            "\n\n## Files Referenced in Evidence\n\n{}\n",
            unique
                .iter()
                .map(|p| format!("- `{}`", p))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    format!(
        "# Recovered Artifact: {}\n\n\
         Artifact state: **evidence-recovery** — model-based synthesis did not produce complete content.\n\
         This file was generated from captured tool evidence.\n\n\
         ## Direct Answer to Objective\n\n\
         The objective was: {}\n\n\
         Based on the evidence gathered below, specific findings with file paths and line numbers \
         are listed where available. Where evidence is insufficient, this is explicitly noted.\n\n\
         ## Session Context\n\n\
         {}{}\n\n\
         ## Evidence\n\n\
         {}\n",
        artifact_name, user_objective, stop_reason_note, files_section, evidence_block,
    )
}

pub(crate) async fn persist_missing_required_artifacts(args: &Args, workdir: &Path, content: &str) {
    let missing = crate::artifact_verifier::find_missing_artifacts(workdir);
    if missing.is_empty() || content.trim().is_empty() {
        return;
    }

    for (artifact, full_path) in missing {
        if let Some(parent) = full_path.parent() {
            if let Err(err) = tokio::fs::create_dir_all(parent).await {
                trace(
                    args,
                    &format!(
                        "artifact_persist_failed path={} stage=mkdir error={}",
                        artifact, err
                    ),
                );
                continue;
            }
        }

        if let Err(err) = tokio::fs::write(&full_path, content.trim()).await {
            trace(
                args,
                &format!(
                    "artifact_persist_failed path={} stage=write error={}",
                    artifact, err
                ),
            );
        } else {
            trace(args, &format!("artifact_persisted path={}", artifact));
        }
    }
}

pub(crate) async fn request_final_answer_without_tools(
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
            "Use clean terminal-friendly formatting. Prefer simple lists and short sections over walls of text. Do not emit XML/JSON tool calls or function-call markup.",
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
