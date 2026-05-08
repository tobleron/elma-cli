use crate::*;
use std::future::Future;
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use crate::stop_policy::{StopOutcome, StopPolicy, StopReason, StageBudget};
use crate::llm_config::{ad_hoc_profile, chat_request_from_profile, ChatRequestOptions, runtime_llm_config};
use crate::auto_compact::{CompactTracker, apply_compact, apply_compact_with_summarizer, DEFAULT_CONTEXT_WINDOW_TOKENS};
use crate::ui_trace::{trace, append_trace_log_line};
use crate::tool_result_storage::{apply_tool_result_budget, DEFAULT_MAX_RESULT_SIZE_CHARS};
use crate::tool_loop::streaming::{request_tool_loop_model_turn_streaming, ToolLoopModelTurn};
use crate::tool_loop::finalization::{finalize_from_evidence_or_fallback, build_evidence_progress_summary, build_fallback_from_recent_tool_evidence, build_bounded_final_evidence, normalize_final_answer_candidate, final_answer_needs_retry, FINAL_EVIDENCE_TOTAL_MAX_CHARS};
use crate::tool_loop::coverage::{sync_loop_summary_coverage, scope_coverage_blocks_finalization, build_scope_coverage_nudge, update_scope_coverage_from_tool, read_call_requests_broad_scope, extract_read_paths_from_args, extract_ls_scope_paths};
use crate::tools::ToolExecutionResult;

pub mod streaming;
pub mod finalization;
pub mod coverage;

pub(crate) struct ToolLoopResult {
    pub(crate) final_answer: String,
    pub(crate) iterations: usize,
    pub(crate) tool_calls_made: usize,
    pub(crate) stopped_by_max: bool,
    pub(crate) stop_outcome: Option<StopOutcome>,
    pub(crate) total_elapsed_s: f64,
    pub(crate) timeout_reason: Option<String>,
    pub(crate) evidence_progress_summary: Option<String>,
    pub(crate) loop_summary: ToolLoopSummary,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ToolLoopSummary {
    pub tool_calls_made: usize,
    pub tool_call_ids: Vec<String>,
    pub successful_reads: Vec<String>,
    pub successful_searches: Vec<String>,
    pub failed_operations: Vec<(String, String)>,
    pub duplicate_suppressions: usize,
    pub coverage: Option<(usize, usize)>,
    pub stop_reason: String,
    pub stop_iteration: usize,
}

pub(crate) async fn await_with_busy_input<T, F>(
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
                tui.process_pending_input_events();
                let _ = tui.pump_ui();
                if let Ok(Some(queued)) = tui.poll_busy_submission() {
                    tui.enqueue_submission(queued);
                }
            }
        }
    }
}

pub(crate) fn is_tool_call_markup(text: &str) -> bool {
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

pub(crate) fn is_intent_only_response(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
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

pub(crate) fn tool_signal(tc: &ToolCall) -> String {
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
        "read" => {
            let single = parsed
                .get("path")
                .or_else(|| parsed.get("filePath"))
                .and_then(|v| v.as_str());
            if let Some(s) = single {
                s.trim().to_string()
            } else if let Some(arr) = parsed.get("paths").and_then(|v| v.as_array()) {
                arr.first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            } else {
                String::new()
            }
        }
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
        other => format!("{other}:{}", tc.function.arguments),
    };
    if fn_name == "respond" {
        return key;
    }
    if fn_name == "shell" {
        format!(
            "{fn_name}:{}",
            crate::text_utils::normalize_shell_signal(&key)
        )
    } else {
        format!("{fn_name}:{key}")
    }
}

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

fn has_recent_tool_evidence(messages: &[ChatMessage]) -> bool {
    for msg in messages.iter().rev().take(5) {
        if msg.role == "tool" {
            let content = msg.content.trim();
            if !content.is_empty() && !content.contains("<tool_call>") && !content.contains("```") {
                return true;
            }
        }
    }
    false
}

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
    raw_user_request: Option<&str>,
    mut work_graph_runner: crate::work_graph_runner::WorkGraphRunner,
) -> Result<ToolLoopResult> {
    let mut budget = StageBudget::from_complexity(complexity);
    if work_graph_runner.is_graph_driven() {
        budget.max_iterations = 1000;
    }
    let total_timeout = Duration::from_secs(45 * 60);
    let loop_start = Instant::now();
    let original_user_request = user_message.to_string();
    let artifact_request = raw_user_request.unwrap_or(user_message);
    crate::artifact_verifier::init_artifact_tracking();
    crate::tool_repair::reset_empty_read_validation_failures();
    let required_artifacts =
        crate::artifact_verifier::extract_required_artifacts_from_request(artifact_request);
    if !required_artifacts.is_empty() {
        crate::artifact_verifier::require_artifacts(&required_artifacts);
        trace(
            args,
            &format!(
                "artifact_tracking: required={}",
                required_artifacts.join(",")
            ),
        );
    }
    trace(
        args,
        &format!(
            "tool_loop: starting max_iterations={} stagnation_threshold={} timeout={}m",
            budget.max_iterations, budget.max_stagnation_cycles, 30
        ),
    );

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
    let mut tool_outcomes: std::collections::HashMap<String, (bool, String)> =
        std::collections::HashMap::new();
    let mut outcome_history = crate::tool_repair::ToolOutcomeHistory::default();
    let mut failure_circuit = crate::tool_repair::ToolFailureCircuit::new();
    let mut empty_read_count: usize = 0;

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
    let mut continuation_count: u32 = 0;
    const MAX_CONTINUATIONS: u32 = 3;

    fn build_continuation_message(
        messages: &[ChatMessage],
        original_intent: &str,
        continuation_num: u32,
        max_continuations: u32,
        session_root: &std::path::Path,
    ) -> String {
        let has_evidence = messages
            .iter()
            .filter(|m| m.role == "tool")
            .any(|m| !m.content.is_empty());
        let failed_tools: Vec<String> = messages
            .iter()
            .filter(|m| m.role == "tool" && m.content.contains("error"))
            .map(|m| m.name.as_deref().unwrap_or("tool").to_string())
            .collect();
        let successful_tools: Vec<String> = messages
            .iter()
            .filter(|m| m.role == "tool" && !m.content.contains("error"))
            .map(|m| m.name.as_deref().unwrap_or("tool").to_string())
            .collect();

        let packet = crate::turn_context_packet::build_turn_context_packet(
            original_intent,
            if has_evidence {
                "Continue from existing evidence"
            } else {
                original_intent
            },
            &crate::artifact_verifier::get_required_artifacts(),
            &successful_tools,
            &failed_tools,
            "budget_exceeded",
        );

        crate::turn_context_packet::persist_turn_context_packet(session_root, &packet);

        let mut continuation = crate::turn_context_packet::build_continuation_from_packet(
            &packet,
            continuation_num,
            max_continuations,
        );

        // Task 774: Inject Evidence Ledger summary
        if let Some(ledger) = crate::evidence_ledger::get_session_ledger() {
            continuation.push_str("\n\n=== EVIDENCE LEDGER (What we already know) ===\n");
            continuation.push_str(&ledger.compact_summary());
            continuation.push_str("\n\nDo NOT repeat these successful tool calls. Continue from where you left off.\n");
        }
        continuation
    }

    let mut consecutive_read_duplicates: u32 = 0;
    let mut consecutive_empty_read_signals: u32 = 0;
    let mut read_stuck_hint_injected: bool = false;
    let mut loop_summary_tracker = ToolLoopSummary::default();
    let mut scope_coverage = crate::scope_coverage::ScopeCoverageLedger::new();
    let mut read_scope_required = false;

    loop {
        turn_counter += 1;
        let turn_id = format!("turn_{}", turn_counter);
        crate::event_log::set_current_turn(&turn_id);
        crate::event_log::record_lifecycle(
            crate::event_log::LifecycleEventType::TurnStarted,
            Some(&turn_id),
        );

        let elapsed = loop_start.elapsed();
        if elapsed > total_timeout {
            let elapsed_mins = elapsed.as_secs() as f64 / 60.0;
            let timeout_reason = format!(
                "45-minute timeout exceeded after {:.1} minutes",
                elapsed_mins
            );
            trace(args, &format!("tool_loop: TIMEOUT {}", timeout_reason));
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
            tui.push_meta_event(
                "STOP",
                &format!("Stopping: {}", timeout_reason),
            );
            sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
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
                evidence_progress_summary: build_evidence_progress_summary(&messages),
                loop_summary: loop_summary_tracker.clone(),
            });
        }

        if let Some(outcome) = stop_policy.start_iteration() {
            trace(
                args,
                &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
            );

            let has_evidence = has_recent_tool_evidence(&messages);
            let is_recoverable = outcome.reason.is_budget_recoverable();
            let can_continue = is_recoverable
                && has_evidence
                && continuation_count < MAX_CONTINUATIONS
                && !complexity.eq_ignore_ascii_case("DIRECT");

            let mutation_type =
                crate::mutation_contract::detect_mutating_request(&original_user_request);
            let mutation_needed =
                mutation_type.is_some() && !crate::mutation_contract::mutation_performed();
            let has_required_artifact_deliverable =
                !crate::artifact_verifier::get_required_artifacts().is_empty();

            if mutation_needed
                && !has_required_artifact_deliverable
                && continuation_count < MAX_CONTINUATIONS
            {
                continuation_count += 1;
                let msg = crate::mutation_contract::build_mutation_required_message(
                    mutation_type.as_deref().unwrap_or("unknown"),
                );
                messages.push(ChatMessage::simple("user", &msg));
                tui.push_stop_notice(
                    "Mutation required: continuing until mutating tool call is made",
                );
                trace(args, "tool_loop: mutation enforced continuation");
                let new_budget = StageBudget::from_complexity(complexity);
                stop_policy = StopPolicy::new(new_budget);
                failure_circuit.clear();
                continue;
            } else if mutation_needed && has_required_artifact_deliverable {
                trace(
                    args,
                    "tool_loop: mutation enforcement deferred to required artifact finalization",
                );
            }

            if can_continue && !has_required_artifact_deliverable {
                continuation_count += 1;
                let cont_msg = if scope_coverage_blocks_finalization(
                    read_scope_required,
                    &scope_coverage,
                ) {
                    sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                    tui.push_meta_event(
                        "COVERAGE",
                        &format!("continuing - {}", scope_coverage.render_summary()),
                    );
                    format!(
                        "Continue from existing evidence. Scope coverage is still incomplete, so do not finalize yet.\n\n{}",
                        build_scope_coverage_nudge(&scope_coverage)
                    )
                } else {
                    build_continuation_message(
                        &messages,
                        &original_user_request,
                        continuation_count,
                        MAX_CONTINUATIONS,
                        &sess.root,
                    )
                };
                messages.push(ChatMessage::simple("user", &cont_msg));
                tui.push_stop_notice(&format!(
                    "Budget continued ({}/{}): meaningful progress detected",
                    continuation_count, MAX_CONTINUATIONS
                ));
                trace(
                    args,
                    &format!(
                        "tool_loop: budget continuation {}/{} after {} iterations",
                        continuation_count,
                        MAX_CONTINUATIONS,
                        stop_policy.iteration()
                    ),
                );
                let new_budget = StageBudget::from_complexity(complexity);
                stop_policy = StopPolicy::new(new_budget);
                failure_circuit.clear();
                continue;
            } else if can_continue && has_required_artifact_deliverable {
                let all_complete = crate::artifact_verifier::are_all_artifacts_complete(workdir);
                let incomplete = crate::artifact_verifier::find_incomplete_artifacts(workdir);
                if !all_complete && !incomplete.is_empty() && continuation_count < MAX_CONTINUATIONS
                {
                    continuation_count += 1;
                    let cont_msg = format!(
                        "You reached the tool-call budget but the following required deliverables are still incomplete:\n{}\n\n\
                         Continue working to complete these deliverables. Focus on completing them directly.",
                        incomplete
                            .iter()
                            .map(|(name, _, state)| { format!("- `{}` ({})", name, state) })
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    messages.push(ChatMessage::simple("user", &cont_msg));
                    tui.push_stop_notice(&format!(
                        "Artifact continuation ({}/{}): completing incomplete deliverables",
                        continuation_count, MAX_CONTINUATIONS
                    ));
                    trace(
                        args,
                        &format!(
                            "tool_loop: artifact continuation {}/{} ({} incomplete artifacts)",
                            continuation_count,
                            MAX_CONTINUATIONS,
                            incomplete.len()
                        ),
                    );
                    let new_budget = StageBudget::from_complexity(complexity);
                    stop_policy = StopPolicy::new(new_budget);
                    failure_circuit.clear();
                    continue;
                }
                trace(
                    args,
                    "tool_loop: budget continuation deferred to required artifact finalization (all artifacts complete or continuations exhausted)",
                );
            }

            let graph_incomplete = work_graph_runner.finalization_is_premature();
            if scope_coverage_blocks_finalization(read_scope_required, &scope_coverage) || graph_incomplete {
                sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                tui.push_meta_event(
                    "COVERAGE",
                    &format!("incomplete at stop - {}", scope_coverage.render_summary()),
                );
                let cont_msg = if graph_incomplete {
                    format!(
                        "{}\n\n{}",
                        work_graph_runner.build_relaxed_continuation(),
                        build_scope_coverage_nudge(&scope_coverage)
                    )
                } else {
                    format!(
                        "You've reached the maximum number of tool calls with incomplete scope coverage. Produce a clearly partial progress report.\n\n{}",
                        build_scope_coverage_nudge(&scope_coverage)
                    )
                };
                messages.push(ChatMessage::simple("user", &cont_msg));
            } else {
                messages.push(ChatMessage::simple(
                    "user",
                    "You've reached the maximum number of tool calls. Please provide your final answer.",
                ));
            }
            let final_content = finalize_from_evidence_or_fallback(
                args,
                tui,
                client,
                chat_url,
                model_id,
                &original_user_request,
                &messages,
                workdir,
                max_tokens,
                Some(&outcome.reason),
            )
            .await;
            let final_trimmed = normalize_final_answer_candidate(&final_content);
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
            crate::event_log::record_lifecycle(
                crate::event_log::LifecycleEventType::TurnFinished,
                Some(&turn_id),
            );
            crate::event_log::clear_current_turn();
            let _ = crate::event_log::persist(&sess.root);
            let missing_after = crate::artifact_verifier::find_missing_artifacts(workdir);
            if !missing_after.is_empty() {
                trace(
                    args,
                    &format!(
                        "tool_loop: partial completion — {} required artifacts still missing after finalization",
                        missing_after.len()
                    ),
                );
                tui.push_stop_notice(&format!(
                    "Partial completion: {} deliverables not completed",
                    missing_after.len()
                ));
            } else {
                tui.push_stop_notice(&format!("Budget limit: {}", outcome.reason.as_str()));
            }
            tui.push_meta_event(
                "STOP",
                &format!(
                    "Stopping: {} - {}",
                    outcome.reason.as_str(),
                    outcome.summary
                ),
            );
            sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
            return Ok(ToolLoopResult {
                final_answer: if final_answer_needs_retry(&final_trimmed) {
                    build_fallback_from_recent_tool_evidence(&messages, Some(&outcome.reason))
                } else {
                    final_trimmed
                },
                iterations: stop_policy.iteration(),
                tool_calls_made: stop_policy.total_tool_calls(),
                stopped_by_max: true,
                stop_outcome: Some(outcome),
                total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                timeout_reason: None,
                evidence_progress_summary: build_evidence_progress_summary(&messages),
                loop_summary: loop_summary_tracker.clone(),
            });
        }

        crate::command_budget::get_budget().start_turn();

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
                tui.push_meta_event(
                    "COMPACTION",
                    &format!(
                        "Auto-compact triggered: {} tokens freed",
                        result.tokens_freed
                    ),
                );
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
        let iter = stop_policy.iteration();
        if max_iter > 0 && iter == (max_iter * 2 / 3).max(1) {
            tui.push_budget_notice(&format!(
                "Approaching iteration limit ({}/{})",
                iter, max_iter
            ));
        }

        tui.process_pending_input_events();

        if work_graph_runner.is_graph_driven() {
            if work_graph_runner.current_progress.is_none() {
                if work_graph_runner.advance_to_next_node().is_some() {
                    trace(
                        args,
                        "work_graph_runner: advanced to next pending graph node",
                    );
                    work_graph_runner.seed_coverage_from_graph();
                }
            }
            work_graph_runner.record_iteration();
        }

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
                crate::event_log::record_model_event(
                    crate::event_log::ModelEventType::ModelResponseReceived,
                    &turn_id,
                    None,
                    None,
                );
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
                crate::event_log::record_model_event(
                    crate::event_log::ModelEventType::ModelResponseReceived,
                    &turn_id,
                    None,
                    None,
                );
                let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
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
                    content_raw: choice.message.content.clone().unwrap_or_default(),
                    tool_calls,
                    reasoning_content: choice.message.reasoning_content.clone(),
                    thinking_content: String::new(),
                }
            }
        };
        let content = turn.content;

        let combined_thinking = {
            let mut ct = String::new();
            if let Some(ref reasoning) = turn.reasoning_content {
                ct.push_str(reasoning);
            }
            if !turn.thinking_content.is_empty() {
                if !ct.is_empty() {
                    ct.push('\n');
                }
                ct.push_str(&turn.thinking_content);
            }
            if ct.trim().is_empty() && !turn.content_raw.is_empty() {
                if let Some(start) = turn.content_raw.find("<think>") {
                    if let Some(end) = turn.content_raw.rfind("</think>") {
                        ct.push_str(&turn.content_raw[start + 7..end]);
                    }
                }
            }
            ct
        };
        if !combined_thinking.is_empty() {
            match crate::llm_config::auxiliary_chat_url() {
                Ok(Some(aux_url)) => {
                    let aux_profile = crate::llm_config::auxiliary_profile("thought_summary");
                    let prompt = format!(
                        "Summarize this thinking in one sentence, less than 90 words, third person (describe what the user asked):\n{}",
                        combined_thinking
                    );
                    let req = crate::llm_config::chat_request_from_profile(
                        &aux_profile,
                        vec![crate::ChatMessage::simple("user", &prompt)],
                        crate::llm_config::ChatRequestOptions {
                            stream: Some(false),
                            temperature: Some(0.0),
                            max_tokens: Some(512),
                            ..Default::default()
                        },
                    );
                    if let Ok(resp) = crate::ui::ui_chat::chat_once_with_timeout(
                        client,
                        &aux_url,
                        &req,
                        aux_profile.timeout_s,
                    )
                    .await
                    {
                        if let Some(choice) = resp.choices.get(0) {
                            if let Some(ref content) = choice.message.content {
                                let stripped = crate::text_utils::strip_thinking_blocks(content);
                                let text = if stripped.trim().is_empty() {
                                    content
                                } else {
                                    &stripped
                                };
                                let clean = text.replace("\\\"", "\"").replace("\\n", "\n");
                                if !clean.trim().is_empty() {
                                    tui.push_thought_summary(&clean);
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    trace(args, "auxiliary_helper_disabled");
                }
                Err(error) => {
                    trace(
                        args,
                        &format!("auxiliary_llm_disabled_or_invalid error={error:#}"),
                    );
                }
            }
        }

        if !turn.tool_calls.is_empty() {
            if let Some(outcome) = stop_policy.record_tool_calls(&turn.tool_calls) {
                trace(
                    args,
                    &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
                );
                let stop_reason = Some(&outcome.reason);
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    workdir,
                    max_tokens,
                    stop_reason,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                tui.push_stop_notice(&format!("Tool call limit: {}", outcome.reason.as_str()));
                tui.push_meta_event(
                    "STOP",
                    &format!(
                        "Stopping: {} - {}",
                        outcome.reason.as_str(),
                        outcome.summary
                    ),
                );
                let evidence_summary = build_evidence_progress_summary(&messages);
                sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages, stop_reason)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                    evidence_progress_summary: evidence_summary,
                    loop_summary: loop_summary_tracker.clone(),
                });
            }

            let mut new_signal_seen = false;
            for tc in &turn.tool_calls {
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
                let stop_reason = Some(&outcome.reason);
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    workdir,
                    max_tokens,
                    stop_reason,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                tui.push_stop_notice(&format!("Stagnation: {}", outcome.reason.as_str()));
                tui.push_meta_event(
                    "STOP",
                    &format!(
                        "Stopping: {} - {}",
                        outcome.reason.as_str(),
                        outcome.summary
                    ),
                );
                let evidence_summary = build_evidence_progress_summary(&messages);
                sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages, stop_reason)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: false,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                    evidence_progress_summary: evidence_summary,
                    loop_summary: loop_summary_tracker.clone(),
                });
            } else {
                let stagnation_info = stop_policy.stagnation_trace_info();
                trace(
                    args,
                    &format!("tool_loop: {} (no new tool signal)", stagnation_info),
                );
                if stop_policy.stagnation_runs() >= 3 {
                    tui.push_meta_event("STAGNATION", &stagnation_info);
                }

                if stop_policy.stagnation_runs() >= 2
                    && work_graph_runner.coverage.has_pending()
                {
                    let pending: Vec<String> = work_graph_runner
                        .coverage
                        .items
                        .iter()
                        .filter(|i| i.status == crate::scope_coverage::CoverageStatus::Pending)
                        .take(5)
                        .map(|i| format!("  - `{}`", i.item))
                        .collect();
                    if !pending.is_empty() {
                        let total = work_graph_runner.coverage.count_by_status(
                            crate::scope_coverage::CoverageStatus::Pending,
                        );
                        let hint = format!(
                            "You have {} unread files. Read one of these next:\n{}\n\
                             Pick any pending file and read it. Do NOT re-read files you already read.",
                            total,
                            pending.join("\n"),
                        );
                        messages.push(ChatMessage::simple("system", &hint));
                        trace(
                            args,
                            &format!(
                                "tool_loop: injected stagnation hint with {} pending files",
                                pending.len()
                            ),
                        );
                    }
                }
            }

            trace(
                args,
                &format!("tool_loop: {} tool call(s)", turn.tool_calls.len()),
            );
            if !content.trim().is_empty() {
                let clean_content = crate::text_utils::strip_thinking_blocks(&content);
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: clean_content,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: turn.reasoning_content.clone(),
                    summarized: false,
                });
            }

            for tc in &turn.tool_calls {
                let sig = tool_signal(tc);
                if tc.function.name != "respond"
                    && tc.function.name != "workspace_info"
                    && tc.function.name != "tool_search"
                {
                    if let Some((ok, prev)) = tool_outcomes.get(&sig) {
                        if *ok {
                            let is_read = tc.function.name == "read";
                            if is_read {
                                consecutive_read_duplicates += 1;
                            }
                            loop_summary_tracker.duplicate_suppressions += 1;
                            let dup_path = if is_read {
                                crate::tool_repair::extract_path_from_args(&tc.function.arguments)
                            } else {
                                sig.clone()
                            };
                            trace(
                                args,
                                &format!(
                                    "tool_loop: duplicate skipped (already succeeded) signal={} consecutive_read_dups={} dup_path={}",
                                    sig, consecutive_read_duplicates, dup_path
                                ),
                            );
                            if is_read
                                && consecutive_read_duplicates >= 2
                                && !read_stuck_hint_injected
                            {
                                read_stuck_hint_injected = true;
                                let mut exclude_set: std::collections::HashSet<String> =
                                    std::collections::HashSet::new();
                                let current_path = crate::tool_repair::extract_path_from_args(
                                    &tc.function.arguments,
                                );
                                if !current_path.is_empty() {
                                    exclude_set.insert(current_path.clone());
                                }
                                let mut alt_hint = String::new();
                                for msg in messages.iter().rev().take(12) {
                                    if msg.role == "tool"
                                        && (msg.name.as_deref() == Some("ls")
                                            || msg.name.as_deref() == Some("glob")
                                            || msg.name.as_deref() == Some("search"))
                                    {
                                        let alt_paths: Vec<String> = msg
                                            .content
                                            .lines()
                                            .map(|l| l.trim())
                                            .filter(|l| {
                                                !l.is_empty()
                                                    && !l.starts_with("total")
                                                    && !l.starts_with("===")
                                                    && !l.starts_with("File:")
                                                    && !l.ends_with("/")
                                                    && l.contains(".")
                                                    && l.len() < 200
                                            })
                                            .filter(|p| !exclude_set.contains(*p))
                                            .take(3)
                                            .map(|s| s.to_string())
                                            .collect();
                                        if !alt_paths.is_empty() {
                                            let formatted: Vec<String> = alt_paths
                                                .iter()
                                                .map(|p| format!("  - `{}`", p))
                                                .collect();
                                            alt_hint = format!(
                                                "You have already read `{}`. Try OTHER unread files from the listing:\n{}\nUse read with a different path, e.g. `{}`.",
                                                if current_path.is_empty() {
                                                    "this file"
                                                } else {
                                                    &current_path
                                                },
                                                formatted.join("\n"),
                                                alt_paths[0]
                                            );
                                            trace(
                                                args,
                                                &format!(
                                                    "tool_loop: read_stuck_hint_injected dup_path={} suggested={}",
                                                    current_path, alt_paths[0]
                                                ),
                                            );
                                            break;
                                        }
                                    }
                                }
                                if alt_hint.is_empty() {
                                    alt_hint = format!(
                                        "You have already read `{}`. {} consecutive duplicate reads detected.\nUse glob or search to discover new files, then read them.",
                                        if current_path.is_empty() {
                                            "this file"
                                        } else {
                                            &current_path
                                        },
                                        consecutive_read_duplicates
                                    );
                                    trace(
                                        args,
                                        &format!(
                                            "tool_loop: read_stuck_hint_generic dup_path={} count={}",
                                            current_path, consecutive_read_duplicates
                                        ),
                                    );
                                }
                                messages.push(ChatMessage::simple("system", &alt_hint));
                                continue;
                            }
                            messages.push(ChatMessage::simple(
                                "system",
                                &format!("Already completed earlier — same result: {}", prev),
                            ));
                            continue;
                        } else {
                            let is_empty_read_retry = sig == "read:"
                                && crate::tool_repair::extract_path_from_args(
                                    &tc.function.arguments,
                                )
                                .is_empty();
                            if is_empty_read_retry {
                                let search_paths = outcome_history.get_existing_search_paths(3);
                                let trace_note = if search_paths.is_empty() {
                                    "empty_read_suppressed no_candidate_exists".to_string()
                                } else {
                                    format!(
                                        "empty_read_repaired_from_evidence candidates=[{}]",
                                        search_paths.join(", ")
                                    )
                                };
                                trace(args, &format!("tool_loop: {} signal={}", trace_note, sig));
                                let hint = if search_paths.is_empty() {
                                    "The same empty read call already failed. No valid file paths found in recent evidence. Use 'glob' or 'search' to discover files first.".to_string()
                                } else {
                                    format!(
                                        "The same empty read call already failed. Use 'read' with one of: {}",
                                        search_paths.join(", ")
                                    )
                                };
                                messages.push(ChatMessage::simple("system", &hint));
                                continue;
                            } else {
                                trace(
                                    args,
                                    &format!(
                                        "tool_loop: duplicate skipped (previous failure) signal={}",
                                        sig
                                    ),
                                );
                                let error_hint = if prev.len() > 10 {
                                    format!(
                                        "Your previous call to this tool failed with: {}. \
                                         Do NOT repeat the same call. Change your arguments or use a different tool.",
                                        prev.chars().take(120).collect::<String>()
                                    )
                                } else {
                                    format!(
                                        "Your previous attempt at '{}' failed. Do NOT repeat it. \
                                         Change your arguments or use a different tool.",
                                        sig
                                    )
                                };
                                messages.push(ChatMessage::simple("system", &error_hint));
                                continue;
                            }
                        }
                    }
                }
                if tc.function.name == "shell" {
                    let (is_risky, reason) =
                        CompactTracker::forecast_shell_output_risk(&tc.function.arguments);
                    if is_risky {
                        tui.push_budget_notice(&format!(
                            "High-risk command detected: {}. Forecast: high volume.",
                            reason
                        ));

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

                if failure_circuit.is_open(&tc.function.name) {
                    trace(
                        args,
                        &format!(
                            "tool_loop: circuit open for {}, injecting strategy shift",
                            tc.function.name
                        ),
                    );
                    if tc.function.name == "shell" {
                        messages.push(ChatMessage::simple(
                            "system",
                            "The shell tool circuit is open. Do not call shell again for this objective. Use non-shell tools if they can complete the work, otherwise provide a bounded failure report with the exact blocker.",
                        ));
                        continue;
                    }
                    let shift_msg = format!(
                        "Tool '{}' has failed repeatedly. \
                         Stop using it and switch to a completely different approach. \
                         Try: shell cat/head for reading files, or a different search strategy.",
                        tc.function.name
                    );
                    messages.push(ChatMessage::simple("system", &shift_msg));
                    let repaired_tc = ToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: ToolFunctionCall {
                            name: "shell".to_string(),
                            arguments: format!(
                                r#"{{"command": "echo '{}'; echo 'Switching strategy per circuit breaker.'"}}"#,
                                tc.function.name
                            ),
                        },
                    };
                    let _ = crate::tool_calling::execute_tool_call(
                        args,
                        &repaired_tc,
                        workdir,
                        sess,
                        client,
                        chat_url,
                        user_message,
                        Some(&mut *tui),
                    )
                    .await;
                    messages.push(ChatMessage::simple(
                        "system",
                        &format!(
                            "Used shell fallback because tool '{}' circuit is open.",
                            tc.function.name
                        ),
                    ));
                    continue;
                }

                let tc = {
                    let repaired_json = crate::tool_repair::repair_tool_call_args(
                        &tc.function.name,
                        &tc.function.arguments,
                        &outcome_history,
                    );
                    match repaired_json {
                        Some(new_args) => ToolCall {
                            id: tc.id.clone(),
                            call_type: tc.call_type.clone(),
                            function: ToolFunctionCall {
                                name: tc.function.name.clone(),
                                arguments: new_args,
                            },
                        },
                        None => tc.clone(),
                    }
                };

                if crate::tool_repair::should_block_empty_read(
                    &tc.function.name,
                    &tc.function.arguments,
                    empty_read_count,
                ) {
                    let search_path = outcome_history.last_search_path().map(|s| s.to_string());
                    let hint = crate::tool_repair::empty_read_fallback_hint(search_path.as_deref());
                    trace(
                        args,
                        &format!("tool_loop: blocked empty read (count={})", empty_read_count),
                    );
                    messages.push(ChatMessage::simple("system", &hint));
                    if let Some(candidate) = search_path
                        .or_else(|| outcome_history.last_written_path().map(|s| s.to_string()))
                    {
                        let escaped = shlex::quote(&candidate).to_string();
                        let fallback_cmd = format!("cat {}", escaped);
                        let fallback_tc = ToolCall {
                            id: tc.id.clone(),
                            call_type: tc.call_type.clone(),
                            function: ToolFunctionCall {
                                name: "shell".to_string(),
                                arguments: format!(r#"{{"command": "{}"}}"#, fallback_cmd),
                            },
                        };
                        let fallback_result = crate::tool_calling::execute_tool_call(
                            args,
                            &fallback_tc,
                            workdir,
                            sess,
                            client,
                            chat_url,
                            user_message,
                            Some(&mut *tui),
                        )
                        .await;
                        if fallback_result.ok {
                            messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: fallback_result.content.clone(),
                                name: Some("shell".to_string()),
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
                                reasoning_content: None,
                                summarized: false,
                            });
                        }
                    }
                    empty_read_count += 1;
                    continue;
                }
                if tc.function.name == "read"
                    && crate::tool_repair::extract_path_from_args(&tc.function.arguments).is_empty()
                {
                    empty_read_count += 1;
                }

                if tc.function.name == "read" && stop_policy.consecutive_read_failures() >= 2 {
                    if let Some(fp) = (|| -> Option<String> {
                        let parsed: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments).ok()?;
                        parsed.get("filePath")?.as_str().map(|s| s.to_string())
                    })() {
                        let escaped = shlex::quote(&fp).to_string();
                        let limit = (|| -> Option<u64> {
                            let parsed: serde_json::Value =
                                serde_json::from_str(&tc.function.arguments).ok()?;
                            parsed.get("limit").and_then(|v| v.as_u64())
                        })()
                        .unwrap_or(0);
                        let fallback_cmd = if limit > 0 {
                            format!("head -n {} {}", limit, escaped)
                        } else {
                            format!("cat {}", escaped)
                        };
                        trace(
                            args,
                            &format!("tool_loop: read→shell fallback cmd={}", fallback_cmd),
                        );
                        let fallback_tc = ToolCall {
                            id: tc.id.clone(),
                            call_type: tc.call_type.clone(),
                            function: ToolFunctionCall {
                                name: "shell".to_string(),
                                arguments: format!(r#"{{"command": "{}"}}"#, fallback_cmd),
                            },
                        };
                        let result = crate::tool_calling::execute_tool_call(
                            args,
                            &fallback_tc,
                            workdir,
                            sess,
                            client,
                            chat_url,
                            user_message,
                            Some(&mut *tui),
                        )
                        .await;

                        crate::event_log::record_tool_event(
                            crate::event_log::ToolEventType::ToolStarted,
                            &turn_id,
                            &tc.id,
                            "read",
                        );
                        crate::event_log::record_tool_event(
                            if result.ok {
                                crate::event_log::ToolEventType::ToolFinished
                            } else {
                                crate::event_log::ToolEventType::ToolFailed
                            },
                            &turn_id,
                            &tc.id,
                            "read",
                        );

                        stop_policy.record_tool_result(&tc, &result);
                        let _ = tui.push_tool_finish(
                            "shell",
                            result.ok,
                            &result.content,
                            Some(result.duration_ms),
                        );
                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: "".to_string(),
                            name: None,
                            tool_calls: Some(vec![tc.clone()]),
                            tool_call_id: None,
                            reasoning_content: None,
                            summarized: false,
                        });
                        let preview = result.content.chars().take(200).collect::<String>();
                        let sig = format!("read:{}", fp);
                        tool_outcomes.insert(sig, (result.ok, preview));

                        if result.ok {
                            messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: result.content.clone(),
                                name: Some("shell".to_string()),
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
                                reasoning_content: None,
                                summarized: false,
                            });
                        } else {
                            messages.push(ChatMessage::simple(
                                "system",
                                "That attempt failed. Try a different approach.",
                            ));
                        }
                        continue;
                    }
                }

                crate::event_log::record_tool_event(
                    crate::event_log::ToolEventType::ToolStarted,
                    &turn_id,
                    &tc.id,
                    &tc.function.name,
                );

                let mut result = crate::tool_calling::execute_tool_call(
                    args,
                    &tc,
                    workdir,
                    sess,
                    client,
                    chat_url,
                    user_message,
                    Some(&mut *tui),
                )
                .await;

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

                crate::session_flush::flush_tool_result(
                    &sess.root,
                    &tc.id,
                    &tc.function.name,
                    &result.content,
                    result.ok,
                );

                let read_paths_for_coverage = if tc.function.name == "read" {
                    extract_read_paths_from_args(&tc.function.arguments)
                } else {
                    Vec::new()
                };
                if tc.function.name == "read"
                    && read_call_requests_broad_scope(&read_paths_for_coverage)
                {
                    read_scope_required = true;
                    if !result.ok {
                        messages.push(ChatMessage::simple(
                            "system",
                            "The broad read attempt did not resolve concrete files. Discover concrete paths with ls or glob, then read the remaining files in batches.",
                        ));
                    }
                }
                update_scope_coverage_from_tool(
                    &mut scope_coverage,
                    &tc.function.name,
                    &tc.function.arguments,
                    &result,
                    workdir,
                    read_scope_required,
                );
                if scope_coverage.total() > 0 {
                    sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                    scope_coverage.persist(&sess.root);
                    if !read_scope_required {
                        read_scope_required = true;
                    }
                    tui.push_meta_event("COVERAGE", &work_graph_runner.render_progress());
                }

                if work_graph_runner.is_graph_driven() {
                    work_graph_runner.sync_external_coverage(&scope_coverage);
                    work_graph_runner.record_tool_call(result.ok);

                    let is_discovery = result.ok
                        && (tc.function.name == "ls" || tc.function.name == "glob");
                    if is_discovery {
                        let paths: Vec<String> = scope_coverage
                            .items
                            .iter()
                            .map(|i| i.item.clone())
                            .collect();
                        let expanded =
                            work_graph_runner.expand_instructions_from_discovery(&paths);
                        if expanded > 0 {
                            work_graph_runner.seed_coverage_from_graph();
                            trace(
                                args,
                                &format!(
                                    "work_graph_runner: expanded {} instructions from discovery",
                                    expanded
                                ),
                            );
                        }
                    }
                }

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
                                crate::tool_repair::extract_path_from_args(&tc.function.arguments);
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
                        let clean_content =
                            match strip_ansi_escapes::strip(result.content.as_bytes()) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                                Err(_) => result.content.clone(),
                            };
                        let entry = ledger.add_entry(source, &clean_content);
                        let source_artifact =
                            entry.raw_path.as_deref().unwrap_or(&tc.function.name);
                        crate::event_log::record_evidence_event(
                            &turn_id,
                            &entry.summary,
                            source_artifact,
                        );
                    });
                }

                stop_policy.record_tool_result(&tc, &result);

                if !result.ok && result.content.contains("required field") {
                    let fail_count = stop_policy.consecutive_identical_errors();
                    if fail_count == 1 {
                        let hint = match tc.function.name.as_str() {
                            "read" => "The 'read' tool requires a filePath argument. Use 'shell cat <path>' instead. Example: shell command='cat docs/ARCHITECTURE.md'".to_string(),
                            "exists" => "The 'exists' tool requires a 'path' argument. Example: exists path='project_tmp/GEMINI.md'. Use 'shell test -f <path>' as an alternative.".to_string(),
                            n => format!("Tool '{}' requires specific arguments. Check the schema and try again.", n),
                        };
                        messages.push(ChatMessage::simple("system", &hint));
                    }
                }

                if tc.function.name == "read"
                    && !result.ok
                    && crate::tool_repair::is_empty_read_validation_stagnation()
                {
                    let stagnation_hint = format!(
                        "The read tool has failed multiple times with missing filePath. \
                         Stop using read. Use 'shell cat' or 'shell head' instead."
                    );
                    messages.push(ChatMessage::simple("system", &stagnation_hint));
                    crate::tool_repair::reset_empty_read_validation_failures();
                }

                if tc.function.name != "respond" && tc.function.name != "update_todo_list" {
                    stop_policy.mark_real_tool_call();
                    stop_policy.reset_respond_counter();
                }

                if tc.function.name == "respond"
                    && evidence_required
                    && !stop_policy.has_real_tool_calls_this_turn()
                {
                    let correction = "You must collect evidence before answering.\n\
                        Use search, read, or shell to gather facts. Do not call 'respond' yet.";
                    result.content = correction.to_string();
                    trace(
                        args,
                        "tool_loop: evidence_required gate blocked respond before evidence",
                    );
                }

                if tc.function.name == "respond" {
                    stop_policy.increment_respond_counter();
                    if stop_policy.consecutive_respond_calls() >= 3
                        && !stop_policy.has_real_tool_calls_this_turn()
                    {
                        messages.push(ChatMessage::simple(
                            "user",
                            "! You have called 'respond' 3 times without collecting any evidence. \
                             You have not used search, read, shell, or any other tool to gather facts. \
                             Call a real tool now to answer the user's question, or reply with 'I cannot answer this.'",
                        ));
                        stop_policy.reset_respond_counter();
                        trace(
                            args,
                            "tool_loop: injected respond abuse correction after 3 consecutive responds",
                        );
                    }
                }

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
                                    "! Your previous response contains claims not supported by evidence. \
                                     You must call a real tool (shell, search, read) to gather facts \
                                     before making factual statements. Do not fabricate information."
                                );
                                result.content = correction;
                                trace(args, "tool_loop: respond blocked by evidence gate");
                            }
                        }
                    }
                }

                if tc.function.name == "respond"
                    && !result.content.is_empty()
                    && (stop_policy.has_real_tool_calls_this_turn()
                        || has_recent_tool_evidence(&messages))
                {
                    let raw_content = normalize_final_answer_candidate(&result.content);
                    if !raw_content.is_empty() {
                        let coverage_incomplete =
                            scope_coverage_blocks_finalization(read_scope_required, &scope_coverage);
                        let graph_incomplete =
                            work_graph_runner.finalization_is_premature();
                        if coverage_incomplete || graph_incomplete {
                            sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                            let nudge = if graph_incomplete {
                                let mut msg = work_graph_runner.build_relaxed_continuation();
                                if coverage_incomplete {
                                    msg.push_str(&format!(
                                        "\n\n{}",
                                        build_scope_coverage_nudge(&scope_coverage)
                                    ));
                                }
                                msg
                            } else {
                                build_scope_coverage_nudge(&scope_coverage)
                            };
                            tui.push_meta_event("COVERAGE", &work_graph_runner.render_progress());
                            if graph_incomplete {
                                tui.push_meta_event(
                                    "FOCUS",
                                    "Graph-driven: finalization blocked — work incomplete",
                                );
                            }
                            messages.push(ChatMessage::simple("system", &nudge));
                            trace(
                                args,
                                &format!(
                                    "tool_loop: blocked respond finalization coverage_incomplete={} graph_incomplete={}",
                                    coverage_incomplete, graph_incomplete
                                ),
                            );
                            continue;
                        }
                        tui.remove_last_assistant_message();
                        let final_content = finalize_from_evidence_or_fallback(
                            args,
                            tui,
                            client,
                            chat_url,
                            model_id,
                            &original_user_request,
                            &messages,
                            workdir,
                            max_tokens,
                            None,
                        )
                        .await;
                        let trimmed_final = normalize_final_answer_candidate(&final_content);
                        sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                        return Ok(ToolLoopResult {
                            final_answer: if final_answer_needs_retry(&trimmed_final) {
                                build_fallback_from_recent_tool_evidence(&messages, None)
                            } else {
                                trimmed_final
                            },
                            iterations: stop_policy.iteration(),
                            tool_calls_made: stop_policy.total_tool_calls(),
                            stopped_by_max: false,
                            stop_outcome: None,
                            total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                            timeout_reason: None,
                            evidence_progress_summary: build_evidence_progress_summary(&messages),
                            loop_summary: loop_summary_tracker.clone(),
                        });
                    }
                }

                let store_for_dedup = tc.function.name != "respond"
                    && tc.function.name != "workspace_info"
                    && tc.function.name != "tool_search";

                if store_for_dedup {
                    let preview = result.content.chars().take(200).collect::<String>();
                    tool_outcomes.insert(sig, (result.ok, preview));
                }

                outcome_history.record(&tc.function.name, &tc.function.arguments, result.ok);
                if result.ok {
                    outcome_history.record_from_result(
                        &tc.function.name,
                        &result.content,
                        result.ok,
                    );
                }
                if result.ok && crate::mutation_contract::is_mutating_tool(&tc.function.name) {
                    crate::mutation_contract::mark_mutation_performed();
                }
                if result.ok {
                    failure_circuit.record_success(&tc.function.name);
                } else {
                    let error_signal = result.content.chars().take(120).collect::<String>();
                    failure_circuit.record_failure(&tc.function.name, &error_signal);
                }

                if result.ok {
                    loop_summary_tracker.tool_calls_made += 1;
                    loop_summary_tracker.tool_call_ids.push(tc.id.clone());
                    match tc.function.name.as_str() {
                        "read" => {
                            let path = crate::tool_repair::extract_path_from_args(&tc.function.arguments);
                            loop_summary_tracker.successful_reads.push(path.clone());
                            work_graph_runner.mark_instruction_by_path(&path);
                        }
                        "search" => {
                            loop_summary_tracker
                                .successful_searches
                                .push(tc.function.arguments.chars().take(200).collect());
                        }
                        _ => {}
                    }
                    if tc.function.name != "read" {
                        consecutive_read_duplicates = 0;
                        consecutive_empty_read_signals = 0;
                    }
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: "".to_string(),
                        name: None,
                        tool_calls: Some(vec![tc.clone()]),
                        tool_call_id: None,
                        reasoning_content: None,
                        summarized: false,
                    });
                    let budgeted = apply_tool_result_budget(
                        sess,
                        &tc.id,
                        &tc.function.name,
                        &result.content,
                        DEFAULT_MAX_RESULT_SIZE_CHARS,
                    );
                    let model_content = if budgeted.content_for_model.trim().is_empty()
                        && tc.function.name != "respond"
                    {
                        "(empty result)".to_string()
                    } else {
                        budgeted.content_for_model
                    };

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
                    messages.push(ChatMessage::simple(
                        "system",
                        "That attempt failed. Try a different approach.",
                    ));
                }
            }

            update_context_estimate(&messages, tui);

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

            if stop_policy.is_retry_loop_detected() {
                if let Some(hint) = stop_policy.strategy_shift_hint() {
                    trace(args, &format!("tool_loop: {}", hint.replace('\n', " | ")));
                    messages.push(ChatMessage::simple("user", &hint));
                }
            }

            if stop_policy.is_identical_error_loop() {
                let last_tool = stop_policy.last_failed_tool_signal();
                if last_tool == "read" {
                    let shift = "The 'read' tool has failed 3+ times with the same error. \
                        Stop using 'read' and use 'shell cat <path>' instead to read files. \
                        Example: shell command='cat docs/ARCHITECTURE.md'";
                    trace(
                        args,
                        &format!("tool_loop: identical-error loop detected for read"),
                    );
                    messages.push(ChatMessage::simple("user", shift));
                } else {
                    let shift = format!(
                        "Tool '{}' has failed 3+ times with the same error. Stop using it and try a completely different approach.",
                        last_tool
                    );
                    trace(
                        args,
                        &format!("tool_loop: identical-error loop detected for {}", last_tool),
                    );
                    messages.push(ChatMessage::simple("user", &shift));
                }
            }

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
                    workdir,
                    max_tokens,
                    None,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                tui.push_stop_notice("Forced finalization due to repeated shell failures");
                tui.push_meta_event(
                    "STOP",
                    "Stopping: Repeated shell failures - Forced finalization to preserve output budget",
                );
                sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages, None)
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
                    evidence_progress_summary: build_evidence_progress_summary(&messages),
                    loop_summary: loop_summary_tracker.clone(),
                });
            }

            if stop_policy.is_struggling() {
                tui.push_meta_event("STRUGGLE", "Model detected as struggling (repeated failures/stagnation). Decomposition recommended.");
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
                let stop_reason = Some(&outcome.reason);
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    workdir,
                    max_tokens,
                    stop_reason,
                )
                .await;
                let trimmed = normalize_final_answer_candidate(&final_content);
                sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed) {
                        build_fallback_from_recent_tool_evidence(&messages, stop_reason)
                    } else {
                        trimmed
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: true,
                    stop_outcome: Some(outcome),
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                    evidence_progress_summary: build_evidence_progress_summary(&messages),
                    loop_summary: loop_summary_tracker.clone(),
                });
            }

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
                        workdir,
                        max_tokens,
                        None,
                    )
                    .await;
                    let trimmed = normalize_final_answer_candidate(&final_content);
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
                    crate::event_log::record_lifecycle(
                        crate::event_log::LifecycleEventType::TurnFinished,
                        Some(&turn_id),
                    );
                    crate::event_log::clear_current_turn();
                    let _ = crate::event_log::persist(&sess.root);
                    sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                    return Ok(ToolLoopResult {
                        final_answer: if final_answer_needs_retry(&trimmed) {
                            build_fallback_from_recent_tool_evidence(
                                &messages,
                                Some(&outcome.reason),
                            )
                        } else {
                            trimmed
                        },
                        iterations: stop_policy.iteration(),
                        tool_calls_made: stop_policy.total_tool_calls(),
                        stopped_by_max: true,
                        stop_outcome: Some(outcome),
                        total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                        timeout_reason: None,
                        evidence_progress_summary: build_evidence_progress_summary(&messages),
                        loop_summary: loop_summary_tracker.clone(),
                    });
                }
            }

            if work_graph_runner.is_graph_driven()
                && work_graph_runner.finalization_is_premature()
            {
                let pending = work_graph_runner.pending_instruction_labels(5);
                if !pending.is_empty() {
                    let remaining: Vec<String> =
                        pending.iter().map(|l| format!("- {}", l)).collect();
                    let msg = format!(
                        "Continue working on the plan. Remaining instructions:\n{}",
                        remaining.join("\n")
                    );
                    messages.push(ChatMessage::simple("system", &msg));
                }
            }

            continue;
        }
        if !content.trim().is_empty() {
            if work_graph_runner.is_graph_driven()
                && work_graph_runner.finalization_is_premature()
            {
                let nudge = work_graph_runner.build_relaxed_continuation();
                messages.push(ChatMessage::simple("system", &nudge));
                tui.push_meta_event(
                    "FOCUS",
                    "Graph-driven: finalization blocked — work incomplete",
                );
                trace(
                    args,
                    &format!(
                        "tool_loop: blocked bare-text finalization graph_incomplete=true"
                    ),
                );
                continue;
            }

            let trimmed = content.trim();
            if is_intent_only_response(&trimmed) && !has_recent_tool_evidence(&messages) {
                trace(
                    args,
                    "tool_loop: detected intent-only response without evidence, continuing to gather proof",
                );
                messages.push(ChatMessage::simple("user", "You haven't executed any tools yet. Please execute the necessary tools to answer my request accurately."));
                continue;
            }
            if has_recent_tool_evidence(&messages) {
                let coverage_incomplete =
                    scope_coverage_blocks_finalization(read_scope_required, &scope_coverage);
                let graph_incomplete = work_graph_runner.finalization_is_premature();
                if coverage_incomplete || graph_incomplete {
                    sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                    let nudge = if graph_incomplete {
                        let mut msg = work_graph_runner.build_relaxed_continuation();
                        if coverage_incomplete {
                            msg.push_str(&format!(
                                "\n\n{}",
                                build_scope_coverage_nudge(&scope_coverage)
                            ));
                        }
                        msg
                    } else {
                        build_scope_coverage_nudge(&scope_coverage)
                    };
                    tui.push_meta_event("COVERAGE", &work_graph_runner.render_progress());
                    if graph_incomplete {
                        tui.push_meta_event(
                            "FOCUS",
                            "Graph-driven: finalization blocked — work incomplete",
                        );
                    }
                    messages.push(ChatMessage::simple("system", &nudge));
                    trace(
                        args,
                        &format!(
                            "tool_loop: blocked voluntary finalization for pending scope coverage {}",
                            scope_coverage.render_summary()
                        ),
                    );
                    continue;
                }
                trace(
                    args,
                    "tool_loop: routing voluntary stop through evidence finalizer (Task 601)",
                );
                tui.remove_last_assistant_message();
                let final_content = finalize_from_evidence_or_fallback(
                    args,
                    tui,
                    client,
                    chat_url,
                    model_id,
                    &original_user_request,
                    &messages,
                    workdir,
                    max_tokens,
                    None,
                )
                .await;
                let trimmed_final = normalize_final_answer_candidate(&final_content);
                sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
                return Ok(ToolLoopResult {
                    final_answer: if final_answer_needs_retry(&trimmed_final) {
                        build_fallback_from_recent_tool_evidence(&messages, None)
                    } else {
                        trimmed_final
                    },
                    iterations: stop_policy.iteration(),
                    tool_calls_made: stop_policy.total_tool_calls(),
                    stopped_by_max: false,
                    stop_outcome: None,
                    total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                    timeout_reason: None,
                    evidence_progress_summary: build_evidence_progress_summary(&messages),
                    loop_summary: loop_summary_tracker.clone(),
                });
            }
            sync_loop_summary_coverage(&mut loop_summary_tracker, &scope_coverage);
            return Ok(ToolLoopResult {
                final_answer: normalize_final_answer_candidate(&content),
                iterations: stop_policy.iteration(),
                tool_calls_made: stop_policy.total_tool_calls(),
                stopped_by_max: false,
                stop_outcome: None,
                total_elapsed_s: loop_start.elapsed().as_secs() as f64,
                timeout_reason: None,
                evidence_progress_summary: build_evidence_progress_summary(&messages),
                loop_summary: loop_summary_tracker.clone(),
            });
        }
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
        let a =
            crate::text_utils::normalize_shell_signal("ls sessions/s_1776868918_801751000/shell/");
        let b =
            crate::text_utils::normalize_shell_signal("ls sessions/s_1775151941_439997000/shell/");
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
        let out = build_fallback_from_recent_tool_evidence(&msgs, None);
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
        assert!(!sig.is_empty(), "respond_signal_should_be_non_empty");
        assert!(
            sig.starts_with("respond:"),
            "respond_signal_should_have_prefix"
        );
        assert!(
            sig.contains("Searching"),
            "respond_signal_should_contain_answer_snippet"
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
            "respond_signal_should_be_truncated_to_40_chars_plus_prefix"
        );
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

    #[test]
    fn broad_read_scope_blocks_until_discovered_files_are_covered() {
        let mut ledger = crate::scope_coverage::ScopeCoverageLedger::new();
        assert!(read_call_requests_broad_scope(&vec![
            "docs/a.md".to_string(),
            "docs/b.md".to_string(),
        ]));
        assert!(scope_coverage_blocks_finalization(true, &ledger));

        ledger.register_items(&["docs/a.md".to_string(), "docs/b.md".to_string()], "file");
        ledger.mark_covered("docs/a.md");
        assert!(scope_coverage_blocks_finalization(true, &ledger));

        ledger.mark_covered("docs/b.md");
        assert!(!scope_coverage_blocks_finalization(true, &ledger));
    }

    #[test]
    fn ls_scope_paths_are_concrete_workspace_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs").join("a.md"), "a").unwrap();
        std::fs::write(root.join("docs").join("b.md"), "b").unwrap();
        std::fs::create_dir_all(root.join("docs").join("nested")).unwrap();

        let output = "docs/  (3 item(s))\n    nested/\n    a.md  (1 B, now)\n    b.md  (1 B, now)";
        let paths = extract_ls_scope_paths(r#"{"path":"docs"}"#, output, root);
        assert_eq!(
            paths,
            vec!["docs/a.md".to_string(), "docs/b.md".to_string()]
        );
    }

    #[test]
    fn shell_scope_output_registers_concrete_workspace_files_when_active() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs").join("a.md"), "a").unwrap();
        std::fs::write(root.join("docs").join("b.md"), "b").unwrap();

        let mut ledger = crate::scope_coverage::ScopeCoverageLedger::new();
        let result = ToolExecutionResult::new_ok("c1", "shell", "docs/a.md\ndocs/b.md\n");
        update_scope_coverage_from_tool(&mut ledger, "shell", "{}", &result, root, true);

        assert_eq!(ledger.total(), 2);
        assert!(scope_coverage_blocks_finalization(true, &ledger));
    }
}
