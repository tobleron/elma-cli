use crate::app::AppRuntime;
use crate::ui_terminal::{TerminalUI, MessageRole};
use crate::ui_state::HeaderInfo;
use crate::app_chat_handlers::handle_chat_command;
use crate::app_chat_orchestrator::build_program;
use crate::app_chat_helpers::*;
use crate::app_chat_trace::*;
use crate::goal_seeding::*;
use crate::session_write::save_goal_state;
use crate::types_api::ChatMessage;
use crate::complexity_gate::{ComplexityLevel, complexity_level_label};
use crate::*;
use anyhow::{Context, Result};
use shlex::quote;
use std::collections::VecDeque;
use std::future::Future;

pub(crate) struct ChatStateMachine<'a> {
    runtime: &'a mut AppRuntime,
    tui: TerminalUI,
    queued_inputs: VecDeque<String>,
    turn_number: u64,
}

impl<'a> ChatStateMachine<'a> {
    pub(crate) fn new(runtime: &'a mut AppRuntime, tui: TerminalUI) -> Self {
        Self {
            runtime,
            tui,
            queued_inputs: VecDeque::new(),
            turn_number: 0,
        }
    }

    /// Check for user interrupt (ESC key) and handle it if detected.
    /// Returns `true` if the turn should be aborted.
    fn check_interrupt(&mut self) -> Result<bool> {
        if self.tui.drain_interrupt() {
            self.tui.set_activity("Stopped", "ESC — stopping...");
            self.tui.push_stop_notice("Interrupted by user (Esc)");
            self.tui.push_meta_event("INTERRUPT", "user_esc_during_execution");
            self.tui.pump_ui()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        // Initialize safe mode from CLI flag / env var
        if self.runtime.args.disable_guards {
            crate::safe_mode::set_safe_mode(crate::safe_mode::SafeMode::Off);
        }

        // Mark TUI as active to suppress stderr status messages
        crate::ui_state::set_tui_active(true);

        self.setup_header();

        // Initial update for the status bar
        self.tui.update_status(
            self.runtime.config.model_id.clone(),
            0,
            self.runtime.config.ctx_max.unwrap_or(0),
            0, // tokens_in
            0, // tokens_out
            "⏱ 0.0s".to_string(),
        );

        let res = loop {
            let line = self.get_next_input().await?;
            let Some(line) = line else { break Ok(()); };
            
            if !self.handle_input(&line).await? {
                break Ok(());
            }
        };

        // Task 739 / Task 762: Persist left chat render at session end (final snapshot)
        {
            let lines = self.tui.visible_transcript_lines();
            let width = self.tui.terminal_width();
            crate::session_write::write_left_chat_render(&self.runtime.state.session.root, &lines, width);
        }

        // Mark TUI as inactive
        crate::ui_state::set_tui_active(false);
        self.tui.cleanup()?;
        res
    }

    fn setup_header(&mut self) {
        let session_name = self.runtime
            .state
            .session
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.runtime.state.session.root.display().to_string());
        let endpoint = self.runtime
            .config
            .chat_url
            .host_str()
            .map(|h| {
                let port = self.runtime
                    .config
                    .chat_url
                    .port()
                    .map(|p| format!(":{}", p))
                    .unwrap_or_default();
                format!("{}://{}{}", self.runtime.config.chat_url.scheme(), h, port)
            })
            .unwrap_or_else(|| self.runtime.config.chat_url.to_string());
        let ws_name = if self.runtime.workspace.ws_brief.is_empty() {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| ".".to_string())
        } else {
            self.runtime.workspace.ws_brief.clone()
        };
        self.tui.set_header_info(HeaderInfo {
            model: self.runtime.config.model_id.clone(),
            endpoint,
            route: String::new(),
            workspace: ws_name,
            session: session_name,
            workflow: String::new(),
            stage: None,
            verbose: self.runtime.tui.verbose,
        });
    }

    async fn get_next_input(&mut self) -> Result<Option<String>> {
        self.queued_inputs.extend(self.tui.take_queued_submissions());
        if let Some(queued) = self.queued_inputs.pop_front() {
            Ok(Some(queued))
        } else {
            let line_opt = self.tui.run_input_loop().await?;
            Ok(line_opt.map(|s| s.to_string()))
        }
    }

    async fn handle_input(&mut self, line: &str) -> Result<bool> {
        let line = line.trim();
        if !handle_chat_command(self.runtime, line, &mut self.tui).await? {
            return Ok(false);
        }
        if line.starts_with('/') {
            return Ok(true);
        }

        self.execute_turn(line).await?;
        Ok(true)
    }

    async fn execute_turn(&mut self, line: &str) -> Result<()> {
        // Task 107: Start effort timer for this turn
        let turn_timer = crate::ui_effort::EffortTimer::start();

        // Clear previous turn's status thread (respects min-visible window)
        self.tui.clear_status();

        self.runtime.state.turn_count += 1;

        self.tui.add_message(MessageRole::User, line.to_string());
        self.runtime
            .state
            .messages
            .push(ChatMessage::simple("user", &line.to_string()));
        let _ = save_user_prompt_display(&self.runtime.state.session, line);

        // T305: Seed goals from multi-step request on first turn
        if self.turn_number == 0 && !self.runtime.state.goal_state.has_active_goal() {
            seed_goals_if_multi_step(line, &mut self.runtime.state.goal_state);
            let _ = save_goal_state(&self.runtime.state.session.root, &self.runtime.state.goal_state);
        }

        // Phase 2 (Task 310): Apply pending turn summary from previous turn
        if let Ok(Some((turn_num, summary))) =
            crate::session_write::load_pending_turn_summary(&self.runtime.state.session.root)
        {
            let mut user_msg_count = 0;
            let mut boundary_idx = 0;
            for (i, msg) in self.runtime.state.messages.iter().enumerate() {
                if msg.role == "user" && msg.name != Some("turn_summary".to_string()) {
                    user_msg_count += 1;
                    if user_msg_count == turn_num + 1 {
                        boundary_idx = i;
                        break;
                    }
                }
            }
            for msg in self.runtime.state.messages.iter_mut().take(boundary_idx) {
                msg.mark_summarized();
            }
            crate::effective_history::inject_turn_summary(&mut self.runtime.state.messages, &summary);
            let _ = crate::session_write::mark_summary_applied(&self.runtime.state.session.root, turn_num);
        }

        // Show activity indicator while processing
        self.tui.set_activity("Thinking", "Thinking...");

        // Immediate redraw so user sees submitted message + busy state
        self.tui.pump_ui()?;
        if self.check_interrupt()? {
            return Ok(());
        }

        // Task 760: Initial shape-based complexity assessment before discovery
        let initial_gate =
            crate::complexity_gate::ComplexityGate::assess(line, None);

        let discovery_metrics = self.try_workspace_discovery(line);

        // Task 760: Scope-based reassessment after discovery.
        // If discovery found many files, upgrade complexity above shape-only estimate.
        let gate_assessment = if let Some(ref metrics) = discovery_metrics {
            let estimated_work = metrics.file_count.saturating_mul(2);
            let reassessed = crate::complexity_gate::ComplexityGate::reassess_with_scope(
                initial_gate.level,
                metrics.file_count,
                &metrics.file_type_mix,
                estimated_work,
            );
            if reassessed.level != initial_gate.level {
                let original_label = complexity_level_label(initial_gate.level);
                let new_label = complexity_level_label(reassessed.level);
                self.tui.push_meta_event(
                    "COMPLEXITY",
                    &format!(
                        "scope_upgrade {}->{} files={} work_est={}",
                        original_label, new_label, metrics.file_count, estimated_work
                    ),
                );
            }
            reassessed
        } else {
            initial_gate
        };

        // Tool discovery and execution (Task 015: Autonomous Tool Discovery)
        if self.runtime.workspace.tool_registry.needs_discovery() {
            if let Ok(registry) = tool_discovery::discover_workspace_tools(&self.runtime.workspace.repo) {
                let ws_count = registry.available_tools().len();
                self.runtime.workspace.tool_registry = registry;
                self.tui.push_meta_event("TOOLS", &format!("workspace_tools={} available={}+ tools via tool_search", ws_count, crate::tool_registry::default_tool_count()));
            }
        }

        let complexity_label = complexity_level_label(gate_assessment.level);

        // Task 380: Create continuity tracker with route alignment check
        let mut continuity_tracker = crate::continuity::ContinuityTracker::new(
            line.to_string(),
            "SHELL",
            "pending",
        );
        crate::continuity::apply_model_threshold(&mut continuity_tracker, &self.runtime.config.model_id);

        trace(
            &self.runtime.args,
            &format!("complexity_level={}", complexity_label),
        );
        trace(
            &self.runtime.args,
            &format!(
                "intent_annotation={}",
                line.replace('\n', " ")
            ),
        );

        self.tui.set_activity("Planning", "Planning...");
        self.tui.pump_ui()?;

        if self.check_interrupt()? {
            return Ok(());
        }

        let program = build_program(
            self.runtime,
            line,
            complexity_label,
            &mut self.tui,
        )
        .await;

        let step_results: Vec<StepResult> = Vec::new();

        // Redraw after planning so user sees the plan before execution
        self.tui.pump_ui()?;

        if self.check_interrupt()? {
            return Ok(());
        }

        self.tui.set_activity("Responding", "Responding...");
        self.tui.pump_ui()?;

        // Tool-calling pipeline produces a single Respond step with pre-built answer.
        let mut final_text = String::new();
        let final_usage_total: Option<u64> = None;
        if let Some(Step::Respond { instructions, .. }) = program.steps.first() {
            final_text = instructions.clone();
            trace(
                &self.runtime.args,
                &format!("tool_calling_answer_used length={}", final_text.len()),
            );
        } else {
            // Error fallback
            final_text = "I encountered an error and could not process your request.".to_string();
        }

        // Clear coordinator status after execution
        self.tui.set_coordinator_status("".to_string(), false);

        // Task 380 / Task 598: Post-execution continuity check.
        // For direct tool-calling, derive evidence from the evidence ledger.
        let has_evidence = crate::evidence_ledger::get_session_ledger()
            .map(|l| l.entries_count() > 0)
            .unwrap_or(false);
        continuity_tracker.check_final_answer(&final_text, has_evidence);
        trace(
            &self.runtime.args,
            &format!(
                "continuity_score={:.2} needs_fallback={} last_stage={}",
                continuity_tracker.alignment_score,
                continuity_tracker.needs_fallback(),
                continuity_tracker
                    .checkpoints
                    .last()
                    .map(|c| c.stage.as_str())
                    .unwrap_or("none"),
            ),
        );

        // Task 498 / Task 597: Continuity guard — if score < 0.85, re-prompt once.
        let is_direct = matches!(
            gate_assessment.level,
            crate::complexity_gate::ComplexityLevel::Direct
        );
        let already_retried = self.runtime
            .state
            .messages
            .last()
            .map(|m| m.content.contains("[continuity_retry]"))
            .unwrap_or(false);
        let mut retry_happened = false;
        if !is_direct && continuity_tracker.needs_fallback() && !already_retried {
            let gap_reason = continuity_tracker.gap();
            let evidence_count = crate::evidence_ledger::get_session_ledger()
                .map(|l| l.entries_count())
                .unwrap_or(0);
            let retry_msg = format!(
                "[continuity_retry]\n\
                The previous answer may not fully address your request.\n\n\
                Original request: {}\n\n\
                Evidence gathered: {} tool outputs recorded.\n\
                Issue detected: The answer appears to have a gap vs the request.\n\
                Specifically: {}\n\n\
                Please provide a more complete answer. Do NOT call any tools — \
                use the evidence you already gathered. Reference specific files \
                and findings.",
                line, evidence_count, gap_reason
            );
            self.runtime
                .state
                .messages
                .push(ChatMessage::simple("user", &retry_msg));

            // Lightweight text-only request with full conversation as context
            let profile = crate::llm_config::ad_hoc_profile(&self.runtime.config.model_id, "continuity_retry");
            let req = crate::llm_config::chat_request_from_profile(
                &profile,
                self.runtime.state.messages.clone(),
                crate::llm_config::ChatRequestOptions {
                    stream: Some(false),
                    ..crate::llm_config::ChatRequestOptions::default()
                },
            );
            match crate::ui::ui_chat::chat_once_with_timeout(
                &self.runtime.config.client,
                &self.runtime.config.chat_url,
                &req,
                profile.timeout_s,
            )
            .await
            {
                Ok(response) => {
                    if let Some(choice) = response.choices.get(0) {
                        if let Some(ref content) = choice.message.content {
                            let improved = crate::final_answer::process_final_answer(content);
                            // Task 609: Only accept retry text if it's valid user-facing content.
                            let trimmed = improved.trim();
                            let is_valid_answer = trimmed.len() >= 20
                                && !crate::strict_tool_parser::StrictToolParser::contains_tool_proposals(&improved);
                            if is_valid_answer {
                                final_text = improved;
                                retry_happened = true;
                                self.runtime
                                    .state
                                    .messages
                                    .push(ChatMessage::simple("assistant", &final_text));
                            } else {
                                trace(
                                    &self.runtime.args,
                                    &format!(
                                        "continuity_retry_rejected: retry response was non-text/too-short ({} chars), keeping original ({} chars)",
                                        trimmed.len(),
                                        final_text.len()
                                    ),
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    trace(
                        &self.runtime.args,
                        &format!("continuity_retry_failed_nonfatal error={}", e),
                    );
                    self.tui.push_meta_event(
                        "RECOVERY",
                        "Continuity retry failed; keeping the best answer already prepared.",
                    );
                }
            }
        }

        // Task 384: Clean-Context Finalization — strip internal framing
        let final_text = crate::final_answer::process_final_answer(&final_text);

        // Task 603: Detect and correct evidence contradictions in the final answer
        let final_text =
            crate::final_answer::correct_evidence_contradictions(&final_text, &self.runtime.state.messages);

        // Task 761: Finalization verification — block polished answers for bad
        // stop reasons with insufficient coverage.
        let stop_outcome = self.runtime.state.last_stop_outcome.as_ref();
        let (final_text, _finalization_status) = verify_finalization_logic(
            &mut self.tui,
            &final_text,
            stop_outcome,
        );

        // Task 608: Best-effort finalization fallback — if answer is still empty
        // after all processing and evidence exists, build a transparent answer.
        let final_text = if final_text.trim().is_empty() && has_evidence {
            build_best_effort_answer_logic(
                self.runtime.state.last_stop_outcome.as_ref(),
                self.runtime.state.last_evidence_summary.as_deref(),
            )
        } else {
            final_text
        };

        // Task 392: Strip markdown for terminal display
        let display_text = crate::final_answer::process_final_answer_display(&final_text);

        // Show assistant response
        if !final_text.is_empty() {
            if retry_happened {
                self.tui.replace_last_assistant_message(display_text);
            } else {
                self.tui.add_message(MessageRole::Assistant, display_text);
            }
            self.runtime
                .state
                .messages
                .push(ChatMessage::simple("assistant", &final_text));
            let _ = save_final_answer_display(&self.runtime.state.session, &final_text);
        }

        // Clear activity indicator
        self.tui.clear_activity();

        // Task 610: Clear evidence ledger at end of turn
        crate::evidence_ledger::clear_session_ledger();

        // Update status bar
        self.update_status_bar(&turn_timer, final_usage_total);

        let types_api_assessment = gate_assessment.to_types_api();
        let default_formula = FormulaSelection::default();
        let default_scope = ScopePlan::default();
        let empty_results = Vec::new();
        let fut = maybe_save_formula_memory(
            &self.runtime.args,
            &self.runtime.config.client,
            &self.runtime.config.chat_url,
            &self.runtime.config.profiles.memory_gate_cfg,
            &self.runtime.config.model_id,
            &self.runtime.config.model_cfg_dir,
            line,
            "AUTO",
            &types_api_assessment,
            &default_formula,
            &default_scope,
            &program,
            &empty_results,
            false,
        );
        await_with_busy_queue_logic(
            &mut self.tui,
            &mut self.queued_inputs,
            fut,
        )
        .await?;

        if self.has_edit_result(&step_results) {
            refresh_runtime_workspace(self.runtime)?;
        }
        let _ = save_goal_state(&self.runtime.state.session.root, &self.runtime.state.goal_state);

        // Phase 1 (Task 310): Spawn background turn summarizer
        self.spawn_summarizer(line, &final_text, &step_results);

        self.queued_inputs.extend(self.tui.take_queued_submissions());

        // Task 762: Persist left chat render after every completed turn
        self.persist_chat_render();

        self.turn_number += 1;
        Ok(())
    }

    fn update_status_bar(&mut self, turn_timer: &crate::ui_effort::EffortTimer, final_usage_total: Option<u64>) {
        let mut tokens_in: u64 = 0;
        let mut tokens_out: u64 = 0;
        for msg in &self.runtime.state.messages {
            let est = TerminalUI::estimate_tokens(&msg.content);
            if msg.role == "assistant" {
                tokens_out += est;
            } else {
                tokens_in += est;
            }
        }
        let ctx_tokens = final_usage_total.unwrap_or(tokens_in + tokens_out);
        self.tui.update_status(
            self.runtime.config.model_id.clone(),
            ctx_tokens,
            self.runtime.config.ctx_max.unwrap_or(0),
            tokens_in,
            tokens_out,
            turn_timer.format(),
        );
    }

    fn spawn_summarizer(&self, line: &str, final_text: &str, step_results: &[StepResult]) {
        let turn_number = self.runtime
            .state
            .messages
            .iter()
            .filter(|m| m.role == "user" && m.name != Some("turn_summary".to_string()))
            .count()
            .saturating_sub(1);
        let step_results_json: Vec<serde_json::Value> = step_results
            .iter()
            .map(|sr| {
                serde_json::json!({
                    "id": sr.id,
                    "kind": sr.kind,
                    "ok": sr.ok,
                    "summary": sr.summary.chars().take(200).collect::<String>(),
                })
            })
            .collect();

        let session_root = self.runtime.state.session.root.clone();
        let client = self.runtime.config.client.clone();
        let summarizer_cfg = self.runtime.config.profiles.turn_summary_cfg.clone();
        let final_text_clone = final_text.to_string();
        let user_message_clone = line.to_string();
        let model_id = self.runtime.config.model_id.clone();
        let last_evidence = self.runtime.state.last_evidence_summary.clone().unwrap_or_default();
        let session_id = session_root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let uid = format!("{}:{}", session_id, turn_number);

        tokio::spawn(async move {
            let unit = crate::intel_units::TurnSummaryUnit::new(summarizer_cfg);
            let tool_summary: String = step_results_json
                .iter()
                .filter_map(|sr| {
                    let kind = sr.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                    let ok = sr.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    let summary = sr.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                    if !summary.is_empty() {
                        Some(format!(
                            "  - {} {}: {}",
                            if ok { "✓" } else { "✗" },
                            kind,
                            summary
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            
            let context = match crate::intel_trait::IntelContext::new(
                user_message_clone,
                last_evidence,
                String::new(),
                vec![],
                client,
            )
            .with_extra("final_text", &final_text_clone)
            .and_then(|c| c.with_extra("uid", &uid))
            .and_then(|c| {
                if !tool_summary.is_empty() {
                    c.with_extra("tool_results", &tool_summary)
                } else {
                    Ok(c)
                }
            }) {
                Ok(ctx) => ctx,
                Err(e) => {
                    tracing::debug!("Failed to build context: {}", e);
                    return;
                }
            };
            match unit.execute_with_fallback(&context).await {
                Ok(output) => {
                    if let Ok(summary) =
                        serde_json::from_value::<TurnSummaryOutput>(output.data)
                    {
                        let _ = crate::session_write::save_turn_summary(
                            &session_root,
                            turn_number,
                            &summary,
                        );
                        crate::session_write::write_summary_markdown(
                            &session_root,
                            turn_number,
                            &chrono::Utc::now().to_rfc3339(),
                            &model_id,
                            &session_id,
                            &summary.summary_narrative,
                        );
                    }
                }
                Err(e) => {
                    tracing::debug!("Turn summary failed: {}", e);
                }
            }
        });
    }

    fn persist_chat_render(&mut self) {
        let lines = self.tui.visible_transcript_lines();
        let width = self.tui.terminal_width();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        crate::session_write::write_left_chat_render(&self.runtime.state.session.root, &lines, width);
        crate::session_write::append_left_chat_frame(
            &self.runtime.state.session.root,
            (self.turn_number + 1) as usize,
            width as usize,
            lines.len(),
            ts,
        );
    }

    fn try_workspace_discovery(&mut self, line: &str) -> Option<DiscoveryMetrics> {
        let path = extract_first_path_from_user_text(line)?;

        // Validate path stays within workspace to prevent directory traversal attacks.
        let canonical_path = std::fs::canonicalize(&path).ok()?;
        let workspace_root = std::fs::canonicalize(".").ok()?;
        if !canonical_path.starts_with(&workspace_root) {
            return None;
        }

        // Properly quote the path for shell interpolation to prevent injection.
        let safe_path = quote(&path);

        let cmd = format!(
            "ls -R {safe_path} | head -n 100; echo '---'; file -b {safe_path}/* 2>/dev/null | head -n 10"
        );
        let output = crate::workspace::cmd_out(&cmd, &std::path::PathBuf::from("."));
        if output.trim().is_empty() {
            return None;
        }

        let ls_part = output.split("---").next().unwrap_or(&output);
        let mut file_count = 0usize;
        let mut file_type_mix = std::collections::HashMap::new();

        for line in ls_part.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.ends_with(':')
                || trimmed.starts_with("total ")
            {
                continue;
            }
            file_count += 1;
            if let Some(ext) = std::path::Path::new(trimmed)
                .extension()
                .and_then(|e| e.to_str())
            {
                *file_type_mix.entry(ext.to_string()).or_insert(0) += 1;
            }
        }

        self.runtime.workspace.ws = format!(
            "### GROUNDED WORKSPACE DISCOVERY ({path})\n{}\n\n{}",
            output.trim(),
            self.runtime.workspace.ws
        );

        Some(DiscoveryMetrics {
            file_count,
            file_type_mix,
        })
    }

    fn has_edit_result(&self, step_results: &[StepResult]) -> bool {
        step_results.iter().any(|s| s.kind == "edit" && s.ok)
    }
}

async fn await_with_busy_queue_logic<T, F>(
    tui: &mut TerminalUI,
    queued_inputs: &mut VecDeque<String>,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result,
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                tui.process_pending_input_events();
                tui.pump_ui()?;
                if let Some(queued) = tui.poll_busy_submission()? {
                    queued_inputs.push_back(queued);
                    tui.notify("Queued 1 message (will run after current response)");
                }
            }
        }
    }
}

fn verify_finalization_logic(
    tui: &mut TerminalUI,
    final_text: &str,
    stop_outcome: Option<&StopOutcome>,
) -> (String, FinalizationStatus) {
    let ledger = crate::evidence_ledger::get_session_ledger();

    let Some(outcome) = stop_outcome else {
        return (final_text.to_string(), FinalizationStatus::Normal);
    };

    let stop_reason_str = outcome.reason.as_str();
    let coverage_count = ledger.as_ref().map(|l| l.entries_count()).unwrap_or(0);
    let has_minimal = ledger.as_ref().map(|l| l.has_minimal_coverage()).unwrap_or(false);
    let unique_files = ledger.as_ref().map(|l| l.unique_files_read()).unwrap_or(0);

    if outcome.reason.is_clean_stop() {
        return (final_text.to_string(), FinalizationStatus::Normal);
    }

    if outcome.reason.is_bad_stop() && !has_minimal {
        tui.push_meta_event("FINALIZATION", "bad_stop_no_coverage - producing partial report");
        let partial = build_partial_answer_logic(
            stop_reason_str,
            coverage_count,
            unique_files,
            ledger.as_ref(),
        );
        return (partial, FinalizationStatus::PartialNoCoverage);
    }

    if outcome.reason.is_bad_stop() && has_minimal {
        let looks_complete = final_text.len() > 200;

        if looks_complete && coverage_count < 5 && unique_files < 3 {
            tui.push_meta_event(
                "FINALIZATION",
                &format!(
                    "bad_stop_thin_coverage stop={} coverage={} files={} - labeling partial",
                    stop_reason_str, coverage_count, unique_files
                ),
            );
            let partial = wrap_partial_label_logic(
                final_text,
                stop_reason_str,
                coverage_count,
                unique_files,
                ledger.as_ref(),
            );
            return (partial, FinalizationStatus::PartialThinCoverage);
        }
    }

    if outcome.reason.is_bad_stop() {
        tui.push_meta_event(
            "FINALIZATION",
            &format!(
                "bad_stop_with_coverage stop={} entries={} files={} - allowing answer",
                stop_reason_str, coverage_count, unique_files
            ),
        );
    }
    (final_text.to_string(), FinalizationStatus::Normal)
}

fn build_best_effort_answer_logic(
    stop_outcome: Option<&StopOutcome>,
    prior_evidence: Option<&str>,
) -> String {
    let evidence_summary = crate::evidence_ledger::get_session_ledger()
        .map(|l| l.compact_summary())
        .unwrap_or_default();

    let stop_prefix = stop_outcome
        .map(|o| {
            format!(
                "[Note: Execution stopped ({}). ",
                o.reason.as_str()
            )
        })
        .unwrap_or_else(|| "[Note: I exhausted my iteration budget before completing this task. ".to_string());

    if evidence_summary.is_empty() || evidence_summary == "No evidence collected yet." {
        let extra = prior_evidence
            .map(|e| format!("\n\nPreviously gathered: {}", e))
            .unwrap_or_default();
        return format!(
            "{}Unfortunately, no usable evidence was gathered in this attempt.]{}",
            stop_prefix, extra
        );
    }

    format!(
        "{}Here's what I found so far:]\n{}",
        stop_prefix, evidence_summary
    )
}

fn build_partial_answer_logic(
    stop_reason: &str,
    coverage_count: usize,
    unique_files: usize,
    ledger: Option<&crate::evidence_ledger::EvidenceLedger>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push("[PARTIAL ANSWER — task incomplete]".to_string());
    parts.push(String::new());
    parts.push(format!("Stop reason: {}", stop_reason));
    parts.push(String::new());

    let coverage_detail = ledger
        .map(|l| l.coverage_summary())
        .unwrap_or_else(|| "No evidence collected.".to_string());
    parts.push("**What was gathered:**".to_string());
    parts.push(coverage_detail);
    parts.push(String::new());

    parts.push("**What remains incomplete:**".to_string());
    parts.push(format!(
        "- Coverage: {} evidence entries, {} unique files read",
        coverage_count, unique_files
    ));

    if coverage_count == 0 {
        parts.push("- No files were read or searched during this attempt.".to_string());
        parts.push("- No shell commands produced useful output.".to_string());
    }

    parts.push(String::new());
    parts.push("**Suggested next action:**".to_string());
    parts.push(format!(
        "- Re-run the task with a narrower scope or break it into smaller steps.",
    ));
    parts.push("- Check that file paths and tool arguments are correct.".to_string());
    parts.push(format!(
        "- If stop_reason is '{}', the model may have been stuck in a loop — \
         try rephrasing the request more directly.",
        stop_reason
    ));

    parts.join("\n")
}

fn wrap_partial_label_logic(
    answer: &str,
    stop_reason: &str,
    coverage_count: usize,
    unique_files: usize,
    ledger: Option<&crate::evidence_ledger::EvidenceLedger>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push("[PARTIAL ANSWER — task incomplete]".to_string());
    parts.push(String::new());
    parts.push(format!(
        "The following answer was produced, but coverage is limited ({} evidence entries, \
         {} unique files read) and the run was stopped for: {}.",
        coverage_count, unique_files, stop_reason
    ));
    parts.push(String::new());

    if let Some(l) = ledger {
        parts.push("**Missing coverage:** found {} total evidence entries.".to_string());
        parts.push(format!("{}", l.coverage_summary()));
        parts.push(String::new());
    }

    parts.push("---".to_string());
    parts.push(String::new());
    parts.push(answer.to_string());

    parts.join("\n")
}

/// Finalization outcome from the verifier (Task 761).
#[derive(Debug, Clone, PartialEq)]
enum FinalizationStatus {
    Normal,
    PartialNoCoverage,
    PartialThinCoverage,
}

/// Metrics from workspace discovery, used for scope-based complexity reassessment.
struct DiscoveryMetrics {
    file_count: usize,
    file_type_mix: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_state_machine_init() {
        // Simple sanity check
    }
}
