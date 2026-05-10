use crate::*;
use crate::stop_policy::{StopOutcome, StopPolicy, StageBudget};
use crate::llm_config::{ad_hoc_profile, chat_request_from_profile, ChatRequestOptions, runtime_llm_config};
use crate::auto_compact::{CompactTracker, apply_compact, apply_compact_with_summarizer, DEFAULT_CONTEXT_WINDOW_TOKENS};
use crate::ui_trace::{trace, append_trace_log_line};
use crate::tool_result_storage::{apply_tool_result_budget, DEFAULT_MAX_RESULT_SIZE_CHARS};
use crate::tool_loop::streaming::{request_tool_loop_model_turn_streaming, ToolLoopModelTurn};
use crate::tool_loop::finalization::{
    finalize_from_evidence_or_fallback, 
    build_evidence_progress_summary, 
    build_fallback_from_recent_tool_evidence, 
    normalize_final_answer_candidate, 
    final_answer_needs_retry
};
use crate::tool_loop::coverage::{
    sync_loop_summary_coverage, 
    scope_coverage_blocks_finalization, 
    build_scope_coverage_nudge, 
    update_scope_coverage_from_tool, 
    read_call_requests_broad_scope, 
    extract_read_paths_from_args
};
use crate::tool_loop::{ToolLoopResult, ToolLoopSummary, await_with_busy_input};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Instant};

pub(crate) struct ToolStateMachine<'a> {
    pub(crate) args: &'a Args,
    pub(crate) client: &'a reqwest::Client,
    pub(crate) chat_url: &'a Url,
    pub(crate) model_id: &'a str,
    pub(crate) system_prompt: &'a str,
    pub(crate) user_message: &'a str,
    pub(crate) workdir: &'a PathBuf,
    pub(crate) sess: &'a SessionPaths,
    pub(crate) temperature: f64,
    pub(crate) max_tokens: u32,
    pub(crate) tui: &'a mut crate::ui_terminal::TerminalUI,
    pub(crate) summarizer_cfg: Option<&'a Profile>,
    pub(crate) context_hint: &'a str,
    pub(crate) evidence_required: bool,
    pub(crate) ctx_max: Option<u64>,
    pub(crate) goal_state: &'a GoalState,
    pub(crate) complexity: &'a str,
    pub(crate) raw_user_request: Option<&'a str>,
    pub(crate) work_graph_runner: crate::work_graph_runner::WorkGraphRunner,

    // Internal state
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) tracker: CompactTracker,
    pub(crate) stop_policy: StopPolicy,
    pub(crate) tool_outcomes: std::collections::HashMap<String, (bool, String)>,
    pub(crate) outcome_history: crate::tool_repair::ToolOutcomeHistory,
    pub(crate) failure_circuit: crate::tool_repair::ToolFailureCircuit,
    pub(crate) empty_read_count: usize,
    pub(crate) turn_counter: usize,
    pub(crate) continuation_count: u32,
    pub(crate) consecutive_read_duplicates: u32,
    pub(crate) consecutive_empty_read_signals: u32,
    pub(crate) read_stuck_hint_injected: bool,
    pub(crate) loop_summary_tracker: ToolLoopSummary,
    pub(crate) scope_coverage: crate::scope_coverage::ScopeCoverageLedger,
    pub(crate) read_scope_required: bool,
    pub(crate) loop_start: Instant,
    pub(crate) original_user_request: String,
}

impl<'a> ToolStateMachine<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        args: &'a Args,
        client: &'a reqwest::Client,
        chat_url: &'a Url,
        model_id: &'a str,
        system_prompt: &'a str,
        user_message: &'a str,
        workdir: &'a PathBuf,
        sess: &'a SessionPaths,
        temperature: f64,
        max_tokens: u32,
        tui: &'a mut crate::ui_terminal::TerminalUI,
        summarizer_cfg: Option<&'a Profile>,
        context_hint: &'a str,
        evidence_required: bool,
        ctx_max: Option<u64>,
        goal_state: &'a GoalState,
        complexity: &'a str,
        raw_user_request: Option<&'a str>,
        work_graph_runner: crate::work_graph_runner::WorkGraphRunner,
    ) -> Self {
        let budget = StageBudget::from_complexity(complexity);
        let mut stop_policy = StopPolicy::new(budget);
        if work_graph_runner.is_graph_driven() {
            stop_policy.budget.max_iterations = 1000;
        }

        let original_user_request = user_message.to_string();
        let messages = vec![
            ChatMessage::simple("system", system_prompt),
            ChatMessage::simple("user", user_message),
        ];

        Self {
            args,
            client,
            chat_url,
            model_id,
            system_prompt,
            user_message,
            workdir,
            sess,
            temperature,
            max_tokens,
            tui,
            summarizer_cfg,
            context_hint,
            evidence_required,
            ctx_max,
            goal_state,
            complexity,
            raw_user_request,
            work_graph_runner,

            messages,
            tracker: CompactTracker::new(),
            stop_policy,
            tool_outcomes: std::collections::HashMap::new(),
            outcome_history: crate::tool_repair::ToolOutcomeHistory::default(),
            failure_circuit: crate::tool_repair::ToolFailureCircuit::new(),
            empty_read_count: 0,
            turn_counter: 0,
            continuation_count: 0,
            consecutive_read_duplicates: 0,
            consecutive_empty_read_signals: 0,
            read_stuck_hint_injected: false,
            loop_summary_tracker: ToolLoopSummary::default(),
            scope_coverage: crate::scope_coverage::ScopeCoverageLedger::new(),
            read_scope_required: false,
            loop_start: Instant::now(),
            original_user_request,
        }
    }

    pub(crate) async fn run(mut self) -> Result<ToolLoopResult> {
        let artifact_request = self.raw_user_request.unwrap_or(self.user_message);
        crate::artifact_verifier::init_artifact_tracking();
        crate::tool_repair::reset_empty_read_validation_failures();
        let required_artifacts =
            crate::artifact_verifier::extract_required_artifacts_from_request(artifact_request);
        if !required_artifacts.is_empty() {
            crate::artifact_verifier::require_artifacts(&required_artifacts);
            trace(
                self.args,
                &format!(
                    "artifact_tracking: required={}",
                    required_artifacts.join(",")
                ),
            );
        }
        trace(
            self.args,
            &format!(
                "tool_loop: starting max_iterations={} stagnation_threshold={} timeout={}m",
                self.stop_policy.budget.max_iterations, self.stop_policy.budget.max_stagnation_cycles, 30
            ),
        );

        let session_id = self.sess
            .root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        crate::evidence_ledger::init_session_ledger(&session_id, &self.sess.root);
        crate::event_log::init_session_event_log(&session_id);

        self.update_context_estimate();

        loop {
            self.turn_counter += 1;
            let turn_id = format!("turn_{}", self.turn_counter);
            crate::event_log::set_current_turn(&turn_id);
            crate::event_log::record_lifecycle(
                crate::event_log::LifecycleEventType::TurnStarted,
                Some(&turn_id),
            );

            // T303: Iteration starts

            if let Some(outcome) = self.stop_policy.start_iteration() {
                if let Some(result) = self.handle_stop_outcome(outcome, &turn_id).await? {
                    return Ok(result);
                }
                continue;
            }

            crate::command_budget::get_budget().start_turn();

            self.handle_compaction().await?;

            let max_iter = self.stop_policy.max_iterations();
            if max_iter > 0 {
                trace(
                    self.args,
                    &format!(
                        "tool_loop: iteration {}/{}",
                        self.stop_policy.iteration(),
                        max_iter
                    ),
                );
            }
            let iter = self.stop_policy.iteration();
            if max_iter > 0 && iter == (max_iter * 2 / 3).max(1) {
                self.tui.push_budget_notice(&format!(
                    "Approaching iteration limit ({}/{})",
                    iter, max_iter
                ));
            }

            self.tui.process_pending_input_events();

            if self.work_graph_runner.is_graph_driven() {
                if self.work_graph_runner.current_progress.is_none() {
                    if self.work_graph_runner.advance_to_next_node().is_some() {
                        trace(
                            self.args,
                            "work_graph_runner: advanced to next pending graph node",
                        );
                        self.work_graph_runner.seed_coverage_from_graph();
                    }
                }
                self.work_graph_runner.record_iteration();
            }

            let turn = self.request_model_turn(&turn_id).await?;
            
            self.handle_thinking(&turn).await?;

            if !turn.tool_calls.is_empty() {
                if let Some(result) = self.handle_tool_calls(turn, &turn_id).await? {
                    return Ok(result);
                }
            } else if !turn.content.trim().is_empty() {
                if let Some(result) = self.handle_content(turn.content, &turn_id).await? {
                    return Ok(result);
                }
            }

            self.update_context_estimate();

            self.handle_goal_consistency().await?;

            if self.stop_policy.is_retry_loop_detected() {
                if let Some(hint) = self.stop_policy.strategy_shift_hint() {
                    trace(self.args, &format!("tool_loop: {}", hint.replace('\n', " | ")));
                    self.messages.push(ChatMessage::simple("user", &hint));
                }
            }

            if self.stop_policy.is_identical_error_loop() {
                self.handle_identical_error_loop();
            }

            if let Some(result) = self.handle_consecutive_shell_failures(&turn_id).await? {
                return Ok(result);
            }

            if self.stop_policy.is_struggling() {
                self.tui.push_meta_event("STRUGGLE", "Model detected as struggling (repeated failures/stagnation). Decomposition recommended.");
            }

            if let Some(outcome) = self.stop_policy.check_should_stop() {
                return self.finalize_loop(outcome, &turn_id, true).await;
            }

            if !self.stop_policy.has_real_tool_calls_this_turn() {
                if let Some(outcome) = self.stop_policy.record_respond_only_turn() {
                    trace(
                        self.args,
                        &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
                    );
                    self.messages.push(ChatMessage::simple(
                        "user",
                        "You've called 'respond' 5+ times without using any real tools (search, read, shell). \
                         Provide your final answer now based on what you know, even if incomplete.",
                    ));
                    return self.finalize_loop(outcome, &turn_id, true).await;
                }
            }
        }
    }

    async fn handle_stop_outcome(&mut self, outcome: StopOutcome, turn_id: &str) -> Result<Option<ToolLoopResult>> {
        trace(
            self.args,
            &format!("tool_loop: stopping reason={}", outcome.reason.as_str()),
        );

        let has_evidence = has_recent_tool_evidence(&self.messages);
        let is_recoverable = outcome.reason.is_budget_recoverable();
        let can_continue = is_recoverable
            && has_evidence
            && self.continuation_count < MAX_CONTINUATIONS
            && !self.complexity.eq_ignore_ascii_case("DIRECT");

        let mutation_type =
            crate::mutation_contract::detect_mutating_request(&self.original_user_request);
        let mutation_needed =
            mutation_type.is_some() && !crate::mutation_contract::mutation_performed();
        let has_required_artifact_deliverable =
            !crate::artifact_verifier::get_required_artifacts().is_empty();

        if mutation_needed
            && !has_required_artifact_deliverable
            && self.continuation_count < MAX_CONTINUATIONS
        {
            self.continuation_count += 1;
            let msg = crate::mutation_contract::build_mutation_required_message(
                mutation_type.as_deref().unwrap_or("unknown"),
            );
            self.messages.push(ChatMessage::simple("user", &msg));
            self.tui.push_stop_notice(
                "Mutation required: continuing until mutating tool call is made",
            );
            trace(self.args, "tool_loop: mutation enforced continuation");
            let new_budget = StageBudget::from_complexity(self.complexity);
            self.stop_policy = StopPolicy::new(new_budget);
            self.failure_circuit.clear();
            return Ok(None);
        } else if mutation_needed && has_required_artifact_deliverable {
            trace(
                self.args,
                "tool_loop: mutation enforcement deferred to required artifact finalization",
            );
        }

        if can_continue && !has_required_artifact_deliverable {
            self.continuation_count += 1;
            let cont_msg = if scope_coverage_blocks_finalization(
                self.read_scope_required,
                &self.scope_coverage,
            ) {
                sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
                self.tui.push_meta_event(
                    "COVERAGE",
                    &format!("continuing - {}", self.scope_coverage.render_summary()),
                );
                format!(
                    "Continue from existing evidence. Scope coverage is still incomplete, so do not finalize yet.\n\n{}",
                    build_scope_coverage_nudge(&self.scope_coverage)
                )
            } else {
                self.build_continuation_message(
                    self.continuation_count,
                    MAX_CONTINUATIONS,
                )
            };
            self.messages.push(ChatMessage::simple("user", &cont_msg));
            self.tui.push_stop_notice(&format!(
                "Budget continued ({}/{}): meaningful progress detected",
                self.continuation_count, MAX_CONTINUATIONS
            ));
            trace(
                self.args,
                &format!(
                    "tool_loop: budget continuation {}/{} after {} iterations",
                    self.continuation_count,
                    MAX_CONTINUATIONS,
                    self.stop_policy.iteration()
                ),
            );
            let new_budget = StageBudget::from_complexity(self.complexity);
            self.stop_policy = StopPolicy::new(new_budget);
            self.failure_circuit.clear();
            return Ok(None);
        } else if can_continue && has_required_artifact_deliverable {
            let all_complete = crate::artifact_verifier::are_all_artifacts_complete(self.workdir);
            let incomplete = crate::artifact_verifier::find_incomplete_artifacts(self.workdir);
            if !all_complete && !incomplete.is_empty() && self.continuation_count < MAX_CONTINUATIONS
            {
                self.continuation_count += 1;
                let cont_msg = format!(
                    "You reached the tool-call budget but the following required deliverables are still incomplete:\n{}\n\n\
                     Continue working to complete these deliverables. Focus on completing them directly.",
                    incomplete
                        .iter()
                        .map(|(name, _, state)| { format!("- `{}` ({})", name, state) })
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                self.messages.push(ChatMessage::simple("user", &cont_msg));
                self.tui.push_stop_notice(&format!(
                    "Artifact continuation ({}/{}): completing incomplete deliverables",
                    self.continuation_count, MAX_CONTINUATIONS
                ));
                trace(
                    self.args,
                    &format!(
                        "tool_loop: artifact continuation {}/{} ({} incomplete artifacts)",
                        self.continuation_count,
                        MAX_CONTINUATIONS,
                        incomplete.len()
                    ),
                );
                let new_budget = StageBudget::from_complexity(self.complexity);
                self.stop_policy = StopPolicy::new(new_budget);
                self.failure_circuit.clear();
                return Ok(None);
            }
            trace(
                self.args,
                "tool_loop: budget continuation deferred to required artifact finalization (all artifacts complete or continuations exhausted)",
            );
        }

        let graph_incomplete = self.work_graph_runner.finalization_is_premature();
        if scope_coverage_blocks_finalization(self.read_scope_required, &self.scope_coverage) || graph_incomplete {
            sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
            self.tui.push_meta_event(
                "COVERAGE",
                &format!("incomplete at stop - {}", self.scope_coverage.render_summary()),
            );
            let cont_msg = if graph_incomplete {
                format!(
                    "{}\n\n{}",
                    self.work_graph_runner.build_relaxed_continuation(),
                    build_scope_coverage_nudge(&self.scope_coverage)
                )
            } else {
                format!(
                    "You've reached the maximum number of tool calls with incomplete scope coverage. Produce a clearly partial progress report.\n\n{}",
                    build_scope_coverage_nudge(&self.scope_coverage)
                )
            };
            self.messages.push(ChatMessage::simple("user", &cont_msg));
        } else {
            self.messages.push(ChatMessage::simple(
                "user",
                "You've reached the maximum number of tool calls. Please provide your final answer.",
            ));
        }
        
        let result = self.finalize_loop(outcome, turn_id, true).await?;
        Ok(Some(result))
    }

    async fn handle_compaction(&mut self) -> Result<()> {
        self.tracker.recalculate(&self.messages);
        let (should_compact, ctx, buf) = self.tracker.should_compact(self.ctx_max.map(|v| v as usize), None);
        if should_compact {
            trace(
                self.args,
                &format!(
                    "auto_compact: firing (tokens={}, turns={}, ctx={}, buf={})",
                    self.tracker.total_tokens, self.tracker.turn_count, ctx, buf
                ),
            );
            let (new_messages, result) = if let Some(cfg) = self.summarizer_cfg {
                apply_compact_with_summarizer(&self.messages, 3, self.client, self.chat_url, cfg).await
            } else {
                apply_compact(&self.messages, 3)
            };
            if result.ok {
                let before_count = self.messages.len();
                self.messages = new_messages;
                self.tracker.record_success();
                self.update_context_estimate();
                self.tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactBoundary);
                self.tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactSummary {
                    message_count: before_count,
                    context_preview: Some("auto compact".to_string()),
                });
                self.tui.push_meta_event(
                    "COMPACTION",
                    &format!(
                        "Auto-compact triggered: {} tokens freed",
                        result.tokens_freed
                    ),
                );
                trace(
                    self.args,
                    &format!(
                        "auto_compact: succeeded (freed {} tokens)",
                        result.tokens_freed
                    ),
                );
            } else {
                self.tracker.record_failure();
                trace(self.args, "auto_compact: failed (no messages to compact)");
            }
        }
        Ok(())
    }

    async fn request_model_turn(&mut self, turn_id: &str) -> Result<ToolLoopModelTurn> {
        let profile = ad_hoc_profile(self.model_id, "tool_loop");
        let req = chat_request_from_profile(
            &profile,
            self.messages.clone(),
            ChatRequestOptions {
                temperature: Some(self.temperature),
                top_p: Some(1.0),
                stream: Some(true),
                max_tokens: Some(self.max_tokens.min(runtime_llm_config().tool_loop_max_tokens_cap)),
                repeat_penalty: Some(None),
                reasoning_format: Some(Some("auto".to_string())),
                tools: Some(crate::tool_calling::build_tool_definitions(&PathBuf::new())),
                ..ChatRequestOptions::default()
            },
        );
        crate::event_log::record_model_event(
            crate::event_log::ModelEventType::ModelRequestStarted,
            turn_id,
            None,
            None,
        );
        match request_tool_loop_model_turn_streaming(
            self.tui,
            self.client,
            self.chat_url,
            req.clone(),
            runtime_llm_config().tool_loop_timeout_s,
            self.sess,
        )
        .await
        {
            Ok(turn) => {
                crate::event_log::record_model_event(
                    crate::event_log::ModelEventType::ModelResponseReceived,
                    turn_id,
                    None,
                    None,
                );
                for tc in &turn.tool_calls {
                    crate::event_log::record_model_event(
                        crate::event_log::ModelEventType::ModelToolCallProposed,
                        turn_id,
                        Some(&tc.id),
                        None,
                    );
                }
                Ok(turn)
            }
            Err(error) => {
                append_trace_log_line(&format!("[TOOL_LOOP_STREAM_FALLBACK] {}", error));
                let mut fallback_req = req;
                fallback_req.stream = false;
                let resp_result = await_with_busy_input(
                    self.tui,
                    crate::ui_chat::chat_once_with_timeout(
                        self.client,
                        self.chat_url,
                        &fallback_req,
                        runtime_llm_config().tool_loop_timeout_s,
                    ),
                )
                .await;
                
                let resp = match resp_result {
                    Ok(r) => r,
                    Err(e) => {
                        if let Some(crate::diagnostics::ElmaDiagnostic::ModelApiContextLimitExceeded { last_error }) = e.downcast_ref::<crate::diagnostics::ElmaDiagnostic>() {
                            append_trace_log_line(&format!("[TOOL_LOOP] Context limit exceeded during fallback. Forcing compaction: {}", last_error));
                            let (new_messages, result) = if let Some(cfg) = self.summarizer_cfg {
                                apply_compact_with_summarizer(&self.messages, 3, self.client, self.chat_url, cfg).await
                            } else {
                                apply_compact(&self.messages, 3)
                            };
                            self.messages = new_messages;
                            if result.ok {
                                self.tracker.record_success();
                                self.tui.push_meta_event("COMPACTION", &format!("Emergency compact triggered: {} tokens freed", result.tokens_freed));
                            }
                            // This is a bit tricky, the original code used `continue` in the outer loop.
                            // We'll return an empty turn to let the outer loop continue.
                            return Ok(ToolLoopModelTurn::default());
                        }
                        return Err(e);
                    }
                };
                let choice = resp.choices.get(0).context("No choices in response")?;
                crate::event_log::record_model_event(
                    crate::event_log::ModelEventType::ModelResponseReceived,
                    turn_id,
                    None,
                    None,
                );
                let tool_calls = choice.message.tool_calls.clone().unwrap_or_default();
                for tc in &tool_calls {
                    crate::event_log::record_model_event(
                        crate::event_log::ModelEventType::ModelToolCallProposed,
                        turn_id,
                        Some(&tc.id),
                        None,
                    );
                }
                Ok(ToolLoopModelTurn {
                    content: choice.message.content.clone().unwrap_or_default(),
                    content_raw: choice.message.content.clone().unwrap_or_default(),
                    tool_calls,
                    reasoning_content: choice.message.reasoning_content.clone(),
                    thinking_content: String::new(),
                })
            }
        }
    }

    async fn handle_thinking(&mut self, turn: &ToolLoopModelTurn) -> Result<()> {
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
                        self.client,
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
                                    self.tui.push_thought_summary(&clean);
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    trace(self.args, "auxiliary_helper_disabled");
                }
                Err(error) => {
                    trace(
                        self.args,
                        &format!("auxiliary_llm_disabled_or_invalid error={error:#}"),
                    );
                }
            }
        }
        Ok(())
    }

    async fn handle_tool_calls(&mut self, turn: ToolLoopModelTurn, turn_id: &str) -> Result<Option<ToolLoopResult>> {
        if let Some(outcome) = self.stop_policy.record_tool_calls(&turn.tool_calls) {
            return Ok(Some(self.finalize_loop(outcome, turn_id, true).await?));
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
            if self.stop_policy.register_signal(sig) {
                new_signal_seen = true;
            }
        }
        if new_signal_seen {
            self.stop_policy.record_new_signals();
        } else if let Some(outcome) = self.stop_policy.record_stagnation() {
            trace(
                self.args,
                "tool_loop: stagnation threshold reached; forcing finalization",
            );
            return Ok(Some(self.finalize_loop(outcome, turn_id, false).await?));
        } else {
            let stagnation_info = self.stop_policy.stagnation_trace_info();
            trace(
                self.args,
                &format!("tool_loop: {} (no new tool signal)", stagnation_info),
            );
            if self.stop_policy.stagnation_runs() >= 3 {
                self.tui.push_meta_event("STAGNATION", &stagnation_info);
            }

            if self.stop_policy.stagnation_runs() >= 2
                && self.work_graph_runner.coverage.has_pending()
            {
                let pending: Vec<String> = self.work_graph_runner
                    .coverage
                    .items
                    .iter()
                    .filter(|i| i.status == crate::scope_coverage::CoverageStatus::Pending)
                    .take(5)
                    .map(|i| format!("  - `{}`", i.item))
                    .collect();
                if !pending.is_empty() {
                    let total = self.work_graph_runner.coverage.count_by_status(
                        crate::scope_coverage::CoverageStatus::Pending,
                    );
                    let hint = format!(
                        "You have {} unread files. Read one of these next:\n{}\n\
                         Pick any pending file and read it. Do NOT re-read files you already read.",
                        total,
                        pending.join("\n"),
                    );
                    self.messages.push(ChatMessage::simple("system", &hint));
                    trace(
                        self.args,
                        &format!(
                            "tool_loop: injected stagnation hint with {} pending files",
                            pending.len()
                        ),
                    );
                }
            }
        }

        trace(
            self.args,
            &format!("tool_loop: {} tool call(s)", turn.tool_calls.len()),
        );

        for tc in turn.tool_calls {
            self.execute_single_tool_call(tc, turn_id).await?;
        }

        Ok(None)
    }

    async fn execute_single_tool_call(&mut self, tc: ToolCall, turn_id: &str) -> Result<()> {
        let sig = tool_signal(&tc);
        if tc.function.name != "workspace_info"
            && tc.function.name != "tool_search"
        {
            if let Some((ok, prev)) = self.tool_outcomes.get(&sig) {
                if *ok {
                    let is_read = tc.function.name == "read";
                    if is_read {
                        self.consecutive_read_duplicates += 1;
                    }
                    self.loop_summary_tracker.duplicate_suppressions += 1;
                    let dup_path = if is_read {
                        crate::tool_repair::extract_path_from_args(&tc.function.arguments)
                    } else {
                        sig.clone()
                    };
                    trace(
                        self.args,
                        &format!(
                            "tool_loop: duplicate skipped (already succeeded) signal={} consecutive_read_dups={} dup_path={}",
                            sig, self.consecutive_read_duplicates, dup_path
                        ),
                    );
                    if is_read
                        && self.consecutive_read_duplicates >= 2
                        && !self.read_stuck_hint_injected
                    {
                        self.handle_read_stuck_hint(&tc);
                        return Ok(());
                    }
                    self.messages.push(ChatMessage::simple(
                        "system",
                        "This call is a duplicate of a successful previous turn. You already have this information in your message history. Do NOT repeat the same call; instead, use the existing evidence to move forward.",
                    ));
                    return Ok(());
                } else {
                    let is_empty_read_retry = sig == "read:"
                        && crate::tool_repair::extract_path_from_args(
                            &tc.function.arguments,
                        )
                        .is_empty();
                    if is_empty_read_retry {
                        let search_paths = self.outcome_history.get_existing_search_paths(3);
                        let trace_note = if search_paths.is_empty() {
                            "empty_read_suppressed no_candidate_exists".to_string()
                        } else {
                            format!(
                                "empty_read_repaired_from_evidence candidates=[{}]",
                                search_paths.join(", ")
                            )
                        };
                        trace(self.args, &format!("tool_loop: {} signal={}", trace_note, sig));
                        let hint = if search_paths.is_empty() {
                            "The same empty read call already failed. No valid file paths found in recent evidence. Use 'glob' or 'search' to discover files first.".to_string()
                        } else {
                            format!(
                                "The same empty read call already failed. Use 'read' with one of: {}",
                                search_paths.join(", ")
                            )
                        };
                        self.messages.push(ChatMessage::simple("system", &hint));
                        return Ok(());
                    } else {
                        trace(
                            self.args,
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
                        self.messages.push(ChatMessage::simple("system", &error_hint));
                        return Ok(());
                    }
                }
            }
        }

        if tc.function.name == "shell" {
            self.handle_shell_risk(&tc).await?;
        }

        if self.failure_circuit.is_open(&tc.function.name) {
            if self.handle_open_circuit(&tc).await? {
                return Ok(());
            }
        }

        let tc = {
            let repaired_json = crate::tool_repair::repair_tool_call_args(
                &tc.function.name,
                &tc.function.arguments,
                &self.outcome_history,
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
                None => tc,
            }
        };

        if crate::tool_repair::should_block_empty_read(
            &tc.function.name,
            &tc.function.arguments,
            self.empty_read_count,
        ) {
            self.handle_empty_read_fallback(&tc).await?;
            return Ok(());
        }

        if tc.function.name == "read"
            && crate::tool_repair::extract_path_from_args(&tc.function.arguments).is_empty()
        {
            self.empty_read_count += 1;
        }

        if tc.function.name == "read" && self.stop_policy.consecutive_read_failures() >= 2 {
            if self.handle_read_to_shell_fallback(&tc, turn_id).await? {
                return Ok(());
            }
        }

        crate::event_log::record_tool_event(
            crate::event_log::ToolEventType::ToolStarted,
            turn_id,
            &tc.id,
            &tc.function.name,
        );

        let available_tokens = self.available_tokens();

        let result = crate::tool_calling::execute_tool_call(
            self.args,
            &tc,
            self.workdir,
            self.sess,
            self.client,
            self.chat_url,
            self.user_message,
            Some(&mut *self.tui),
            available_tokens,
        )
        .await;

        let tool_event_type = if result.ok {
            crate::event_log::ToolEventType::ToolFinished
        } else {
            crate::event_log::ToolEventType::ToolFailed
        };
        crate::event_log::record_tool_event(
            tool_event_type,
            turn_id,
            &tc.id,
            &tc.function.name,
        );

        crate::session_flush::flush_tool_result(
            &self.sess.root,
            &tc.id,
            &tc.function.name,
            &result.content,
            result.ok,
        );

        self.update_coverage(&tc, &result);

        if tc.function.name != "update_todo_list"
            && tc.function.name != "tool_search"
        {
            self.record_evidence(&tc, &result, turn_id);
        }

        self.stop_policy.record_tool_result(&tc, &result);

        self.handle_tool_result_logic(&tc, &result);

        Ok(())
    }

    fn available_tokens(&self) -> Option<usize> {
        self.ctx_max.map(|max| {
            max.saturating_sub(self.tracker.total_tokens as u64) as usize
        })
    }

    fn handle_read_stuck_hint(&mut self, tc: &ToolCall) {
        self.read_stuck_hint_injected = true;
        let mut exclude_set: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let current_path = crate::tool_repair::extract_path_from_args(
            &tc.function.arguments,
        );
        if !current_path.is_empty() {
            exclude_set.insert(current_path.clone());
        }
        let mut alt_hint = String::new();
        for msg in self.messages.iter().rev().take(12) {
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
                        self.args,
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
                "You have already read `{}` and the content is in your context. {} consecutive duplicate reads detected.\n\
                 STOP reading the same files. If you are stuck, use `glob` or `search` to find NEW files, or proceed to `respond` using the facts you already gathered.",
                if current_path.is_empty() {
                    "this file"
                } else {
                    &current_path
                },
                self.consecutive_read_duplicates
            );
            trace(
                self.args,
                &format!(
                    "tool_loop: read_stuck_hint_generic dup_path={} count={}",
                    current_path, self.consecutive_read_duplicates
                ),
            );
        }
        self.messages.push(ChatMessage::simple("system", &alt_hint));
    }

    async fn handle_shell_risk(&mut self, tc: &ToolCall) -> Result<()> {
        let (is_risky, reason) =
            CompactTracker::forecast_shell_output_risk(&tc.function.arguments);
        if is_risky {
            self.tui.push_budget_notice(&format!(
                "High-risk command detected: {}. Forecast: high volume.",
                reason
            ));

            let mut ctx_limit = self.tui.get_context_max() as usize;
            if ctx_limit == 0 {
                ctx_limit = self.ctx_max
                    .map(|v| v as usize)
                    .unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS);
            }
            if self.tracker.total_tokens > (ctx_limit * 70 / 100) {
                trace(
                    self.args,
                    "auto_compact: proactive compaction for high-risk command",
                );
                let (new_messages, result) = if let Some(cfg) = self.summarizer_cfg {
                    apply_compact_with_summarizer(&self.messages, 3, self.client, self.chat_url, cfg)
                        .await
                } else {
                    apply_compact(&self.messages, 3)
                };
                if result.ok {
                    self.messages = new_messages;
                    self.tracker.record_success();
                    self.tracker.recalculate(&self.messages);
                    self.update_context_estimate();
                    self.tui.add_claude_message(
                        crate::claude_ui::ClaudeMessage::CompactBoundary,
                    );
                    self.tui.push_compaction_notice(
                        "Proactive compaction triggered to accommodate high-volume shell output.",
                    );
                }
            }
        }
        Ok(())
    }

    async fn handle_open_circuit(&mut self, tc: &ToolCall) -> Result<bool> {
        trace(
            self.args,
            &format!(
                "tool_loop: circuit open for {}, injecting strategy shift",
                tc.function.name
            ),
        );
        if tc.function.name == "shell" {
            self.messages.push(ChatMessage::simple(
                "system",
                "The shell tool circuit is open. Do not call shell again for this objective. Use non-shell tools if they can complete the work, otherwise provide a bounded failure report with the exact blocker.",
            ));
            return Ok(true);
        }
        let shift_msg = format!(
            "Tool '{}' has failed repeatedly. \
             Stop using it and switch to a completely different approach. \
             Try: shell cat/head for reading files, or a different search strategy.",
            tc.function.name
        );
        self.messages.push(ChatMessage::simple("system", &shift_msg));
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
        let available_tokens = self.available_tokens();
        let _ = crate::tool_calling::execute_tool_call(
            self.args,
            &repaired_tc,
            self.workdir,
            self.sess,
            self.client,
            self.chat_url,
            self.user_message,
            Some(&mut *self.tui),
            available_tokens,
        )
        .await;
        self.messages.push(ChatMessage::simple(
            "system",
            &format!(
                "Used shell fallback because tool '{}' circuit is open.",
                tc.function.name
            ),
        ));
        Ok(true)
    }

    async fn handle_empty_read_fallback(&mut self, tc: &ToolCall) -> Result<()> {
        let search_path = self.outcome_history.last_search_path().map(|s| s.to_string());
        let hint = crate::tool_repair::empty_read_fallback_hint(search_path.as_deref());
        trace(
            self.args,
            &format!("tool_loop: blocked empty read (count={})", self.empty_read_count),
        );
        self.messages.push(ChatMessage::simple("system", &hint));
        if let Some(candidate) = search_path
            .or_else(|| self.outcome_history.last_written_path().map(|s| s.to_string()))
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
            let available_tokens = self.available_tokens();
            let fallback_result = crate::tool_calling::execute_tool_call(
                self.args,
                &fallback_tc,
                self.workdir,
                self.sess,
                self.client,
                self.chat_url,
                self.user_message,
                Some(&mut *self.tui),
                available_tokens,
            )
            .await;
            if fallback_result.ok {
                self.messages.push(ChatMessage {
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
        self.empty_read_count += 1;
        Ok(())
    }

    async fn handle_read_to_shell_fallback(&mut self, tc: &ToolCall, turn_id: &str) -> Result<bool> {
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
                self.args,
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
            let available_tokens = self.available_tokens();
            let result = crate::tool_calling::execute_tool_call(
                self.args,
                &fallback_tc,
                self.workdir,
                self.sess,
                self.client,
                self.chat_url,
                self.user_message,
                Some(&mut *self.tui),
                available_tokens,
            )
            .await;

            crate::event_log::record_tool_event(
                crate::event_log::ToolEventType::ToolStarted,
                turn_id,
                &tc.id,
                "read",
            );
            crate::event_log::record_tool_event(
                if result.ok {
                    crate::event_log::ToolEventType::ToolFinished
                } else {
                    crate::event_log::ToolEventType::ToolFailed
                },
                turn_id,
                &tc.id,
                "read",
            );

            self.stop_policy.record_tool_result(tc, &result);
            let _ = self.tui.push_tool_finish(
                "shell",
                result.ok,
                &result.content,
                Some(result.duration_ms),
            );
            self.messages.push(ChatMessage {
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
            self.tool_outcomes.insert(sig, (result.ok, preview));

            if result.ok {
                self.messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: result.content.clone(),
                    name: Some("shell".to_string()),
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                    reasoning_content: None,
                    summarized: false,
                });
            } else {
                self.messages.push(ChatMessage::simple(
                    "system",
                    "That attempt failed. Try a different approach.",
                ));
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn update_coverage(&mut self, tc: &ToolCall, result: &crate::tools::ToolExecutionResult) {
        let read_paths_for_coverage = if tc.function.name == "read" {
            extract_read_paths_from_args(&tc.function.arguments)
        } else {
            Vec::new()
        };
        if tc.function.name == "read"
            && read_call_requests_broad_scope(&read_paths_for_coverage)
        {
            self.read_scope_required = true;
            if !result.ok {
                self.messages.push(ChatMessage::simple(
                    "system",
                    "The broad read attempt did not resolve concrete files. Discover concrete paths with ls or glob, then read the remaining files in batches.",
                ));
            }
        }
        update_scope_coverage_from_tool(
            &mut self.scope_coverage,
            &tc.function.name,
            &tc.function.arguments,
            result,
            self.workdir,
            self.read_scope_required,
        );
        if self.scope_coverage.total() > 0 {
            sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
            self.scope_coverage.persist(&self.sess.root);
            if !self.read_scope_required {
                self.read_scope_required = true;
            }
        }
        self.tui.push_meta_event("COVERAGE", &self.work_graph_runner.render_progress());

        if self.work_graph_runner.is_graph_driven() {
            self.work_graph_runner.sync_external_coverage(&self.scope_coverage);
            self.work_graph_runner.record_tool_call(result.ok);

            let is_discovery = result.ok
                && (tc.function.name == "ls" || tc.function.name == "glob");
            if is_discovery {
                let paths: Vec<String> = self.scope_coverage
                    .items
                    .iter()
                    .map(|i| i.item.clone())
                    .collect();
                let expanded =
                    self.work_graph_runner.expand_instructions_from_discovery(&paths);
                if expanded > 0 {
                    self.work_graph_runner.seed_coverage_from_graph();
                    trace(
                        self.args,
                        &format!(
                            "work_graph_runner: expanded {} instructions from discovery",
                            expanded
                        ),
                    );
                }
            }
        }
    }

    fn record_evidence(&mut self, tc: &ToolCall, result: &crate::tools::ToolExecutionResult, turn_id: &str) {
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
                turn_id,
                &entry.summary,
                source_artifact,
            );
        });
    }

    fn handle_tool_result_logic(&mut self, tc: &ToolCall, result: &crate::tools::ToolExecutionResult) {
        if !result.ok && result.content.contains("required field") {
            let fail_count = self.stop_policy.consecutive_identical_errors();
            if fail_count == 1 {
                let hint = match tc.function.name.as_str() {
                    "read" => "The 'read' tool requires a filePath argument. Use 'shell cat <path>' instead. Example: shell command='cat docs/ARCHITECTURE.md'".to_string(),
                    "exists" => "The 'exists' tool requires a 'path' argument. Example: exists path='project_tmp/GEMINI.md'. Use 'shell test -f <path>' as an alternative.".to_string(),
                    n => format!("Tool '{}' requires specific arguments. Check the schema and try again.", n),
                };
                self.messages.push(ChatMessage::simple("system", &hint));
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
            self.messages.push(ChatMessage::simple("system", &stagnation_hint));
            crate::tool_repair::reset_empty_read_validation_failures();
        }

        if tc.function.name != "update_todo_list" {
            self.stop_policy.mark_real_tool_call();
            self.stop_policy.reset_respond_counter();
        }

        let store_for_dedup = tc.function.name != "workspace_info"
            && tc.function.name != "tool_search";

        if store_for_dedup {
            let sig = tool_signal(tc);
            let preview = result.content.chars().take(200).collect::<String>();
            self.tool_outcomes.insert(sig, (result.ok, preview));
        }

        self.outcome_history.record(&tc.function.name, &tc.function.arguments, result.ok);
        if result.ok {
            self.outcome_history.record_from_result(
                &tc.function.name,
                &result.content,
                result.ok,
            );
        }
        if result.ok && crate::mutation_contract::is_mutating_tool(&tc.function.name) {
            crate::mutation_contract::mark_mutation_performed();
        }
        if result.ok {
            self.failure_circuit.record_success(&tc.function.name);
        } else {
            let error_signal = result.content.chars().take(120).collect::<String>();
            self.failure_circuit.record_failure(&tc.function.name, &error_signal);
        }

        if result.ok {
            self.loop_summary_tracker.tool_calls_made += 1;
            self.loop_summary_tracker.tool_call_ids.push(tc.id.clone());
            match tc.function.name.as_str() {
                "read" => {
                    let path = crate::tool_repair::extract_path_from_args(&tc.function.arguments);
                    self.loop_summary_tracker.successful_reads.push(path.clone());
                    self.work_graph_runner.mark_instruction_by_path(&path);
                }
                "shell" => {
                    if self.work_graph_runner.is_graph_driven() {
                        self.work_graph_runner.mark_current_node_succeeded();
                        trace(self.args, "work_graph_runner: marked current node succeeded after shell tool success");
                    }
                }
                "search" => {
                    self.loop_summary_tracker
                        .successful_searches
                        .push(tc.function.arguments.chars().take(200).collect());
                }
                _ => {}
            }
            if tc.function.name != "read" {
                self.consecutive_read_duplicates = 0;
                self.consecutive_empty_read_signals = 0;
            }
            self.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: "".to_string(),
                name: None,
                tool_calls: Some(vec![tc.clone()]),
                tool_call_id: None,
                reasoning_content: None,
                summarized: false,
            });
            let budgeted = apply_tool_result_budget(
                self.sess,
                &tc.id,
                &tc.function.name,
                &result.content,
                DEFAULT_MAX_RESULT_SIZE_CHARS,
                crate::output_truncation::TruncationPolicy::default(),
            );
            let model_content = if budgeted.content_for_model.trim().is_empty()
                && tc.function.name != "workspace_info"
            {
                "(empty result)".to_string()
            } else {
                budgeted.content_for_model
            };

            let reflection = crate::evidence_ledger::get_session_ledger()
                .and_then(|ledger| ledger.get_latest_reflection())
                .map(|r| format!("\n→ Reflection: {}", r))
                .unwrap_or_default();

            self.messages.push(ChatMessage {
                role: "tool".to_string(),
                content: format!("{}{}", model_content, reflection),
                name: Some(tc.function.name.clone()),
                tool_calls: None,
                tool_call_id: Some(tc.id.clone()),
                reasoning_content: None,
                summarized: false,
            });
        } else {
            self.messages.push(ChatMessage::simple(
                "system",
                "That attempt failed. Try a different approach.",
            ));
        }
    }

    async fn handle_goal_consistency(&mut self) -> Result<()> {
        if self.stop_policy.goal_consistency_check_needed() && self.goal_state.has_active_goal() {
            let recent_tool_summary = build_recent_tool_summary(&self.messages, 15);
            let profile = ad_hoc_profile(self.model_id, "goal_consistency");
            let steering = crate::intel_units::run_goal_consistency_check(
                self.client,
                &profile,
                self.goal_state,
                &recent_tool_summary,
            )
            .await;
            if let Some(steering_msg) = steering {
                trace(
                    self.args,
                    &format!(
                        "tool_loop: goal consistency steering injected ({} chars)",
                        steering_msg.len()
                    ),
                );
                self.messages.push(ChatMessage::simple("user", &steering_msg));
            }
        }
        Ok(())
    }

    fn handle_identical_error_loop(&mut self) {
        let last_tool = self.stop_policy.last_failed_tool_signal();
        if last_tool == "read" {
            let shift = "The 'read' tool has failed 3+ times with the same error. \
                Stop using 'read' and use 'shell cat <path>' instead to read files. \
                Example: shell command='cat docs/ARCHITECTURE.md'";
            trace(
                self.args,
                &format!("tool_loop: identical-error loop detected for read"),
            );
            self.messages.push(ChatMessage::simple("user", shift));
        } else {
            let shift = format!(
                "Tool '{}' has failed 3+ times with the same error. Stop using it and try a completely different approach.",
                last_tool
            );
            trace(
                self.args,
                &format!("tool_loop: identical-error loop detected for {}", last_tool),
            );
            self.messages.push(ChatMessage::simple("user", &shift));
        }
    }

    async fn handle_consecutive_shell_failures(&mut self, turn_id: &str) -> Result<Option<ToolLoopResult>> {
        let consecutive_failures = self.stop_policy.consecutive_shell_failures();
        if consecutive_failures >= 5 {
            trace(
                self.args,
                &format!(
                    "tool_loop: forcing finalization after {} consecutive shell failures (T304 budget preservation)",
                    consecutive_failures
                ),
            );
            self.messages.push(ChatMessage::simple(
                "user",
                "You've had 5+ consecutive shell failures. Stop trying shell commands and provide your final answer based on the evidence you already have. If you cannot answer reliably, explain what you found and what additional information would be needed."
            ));
            
            let outcome = StopOutcome {
                reason: crate::stop_policy::StopReason::RepeatedToolFailure,
                stage_index: 0,
                stage_skill: "general".to_string(),
                summary: format!("Forced finalization after {} consecutive shell failures to preserve output budget", consecutive_failures),
                next_step_hint: "Verify commands manually before retrying, or use a different approach (read/search tools instead of shell)".to_string(),
            };
            
            return Ok(Some(self.finalize_loop(outcome, turn_id, true).await?));
        }
        Ok(None)
    }

    async fn finalize_loop(&mut self, outcome: StopOutcome, turn_id: &str, push_notice: bool) -> Result<ToolLoopResult> {
        if push_notice {
            self.tui.push_stop_notice(&format!("Limit reached: {}", outcome.reason.as_str()));
        }
        self.tui.push_meta_event(
            "STOP",
            &format!(
                "Stopping: {} - {}",
                outcome.reason.as_str(),
                outcome.summary
            ),
        );

        let final_content = finalize_from_evidence_or_fallback(
            self.args,
            self.tui,
            self.client,
            self.chat_url,
            self.model_id,
            &self.original_user_request,
            &self.messages,
            self.workdir,
            self.max_tokens,
            Some(&outcome.reason),
        )
        .await;
        let final_trimmed = normalize_final_answer_candidate(&final_content);
        
        crate::event_log::record_finalization(
            crate::event_log::FinalizationEventType::FinalAnswerPrepared,
            turn_id,
            outcome.reason.as_str(),
        );
        crate::event_log::record_finalization(
            crate::event_log::FinalizationEventType::StopPolicyTriggered,
            turn_id,
            outcome.reason.as_str(),
        );
        crate::event_log::record_lifecycle(
            crate::event_log::LifecycleEventType::TurnFinished,
            Some(turn_id),
        );
        crate::event_log::clear_current_turn();
        let _ = crate::event_log::persist(&self.sess.root);
        
        let missing_after = crate::artifact_verifier::find_missing_artifacts(self.workdir);
        if !missing_after.is_empty() {
            trace(
                self.args,
                &format!(
                    "tool_loop: partial completion — {} required artifacts still missing after finalization",
                    missing_after.len()
                ),
            );
            self.tui.push_stop_notice(&format!(
                "Partial completion: {} deliverables not completed",
                missing_after.len()
            ));
        }

        sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
        
        Ok(ToolLoopResult {
            final_answer: if final_answer_needs_retry(&final_trimmed) {
                build_fallback_from_recent_tool_evidence(&self.messages, Some(&outcome.reason))
            } else {
                final_trimmed
            },
            iterations: self.stop_policy.iteration(),
            tool_calls_made: self.stop_policy.total_tool_calls(),
            stopped_by_max: outcome.reason.is_budget_stop(),
            stop_outcome: Some(outcome),
            total_elapsed_s: self.loop_start.elapsed().as_secs() as f64,
            timeout_reason: None,
            evidence_progress_summary: build_evidence_progress_summary(&self.messages),
            loop_summary: self.loop_summary_tracker.clone(),
        })
    }

    async fn handle_content(&mut self, content: String, turn_id: &str) -> Result<Option<ToolLoopResult>> {
        if self.work_graph_runner.is_graph_driven()
            && self.work_graph_runner.finalization_is_premature()
        {
            let nudge = self.work_graph_runner.build_relaxed_continuation();
            self.messages.push(ChatMessage::simple("system", &nudge));
            self.tui.push_meta_event(
                "FOCUS",
                "Graph-driven: finalization blocked — work incomplete",
            );
            trace(
                self.args,
                &format!(
                    "tool_loop: blocked bare-text finalization graph_incomplete=true"
                ),
            );
            return Ok(None);
        }

        let trimmed = content.trim();
        if is_intent_only_response(trimmed) && !has_recent_tool_evidence(&self.messages) {
            trace(
                self.args,
                "tool_loop: detected intent-only response without evidence, continuing to gather proof",
            );
            self.messages.push(ChatMessage::simple("user", "You haven't executed any tools yet. Please execute the necessary tools to answer my request accurately."));
            return Ok(None);
        }
        if has_recent_tool_evidence(&self.messages) {
            if let Some(ledger) = crate::evidence_ledger::get_session_ledger() {
                if ledger.entries_count() > 0 {
                    let verdict = crate::evidence_ledger::enforce_evidence_grounding(
                        &content,
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
                        trace(self.args, &format!("tool_loop: bare text {}", msg));
                        self.tui.push_meta_event("EVIDENCE", &msg);
                        let correction = format!(
                            "! Your previous response contains claims not supported by evidence. \
                             You must call a real tool (shell, search, read) to gather facts \
                             before making factual statements. Do not fabricate information."
                        );
                        self.messages.push(ChatMessage::simple("user", &correction));
                        return Ok(None);
                    }
                }
            }
            let coverage_incomplete =
                scope_coverage_blocks_finalization(self.read_scope_required, &self.scope_coverage);
            let graph_incomplete = self.work_graph_runner.finalization_is_premature();
            if coverage_incomplete || graph_incomplete {
                sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
                let nudge = if graph_incomplete {
                    let mut msg = self.work_graph_runner.build_relaxed_continuation();
                    if coverage_incomplete {
                        msg.push_str(&format!(
                            "\n\n{}",
                            build_scope_coverage_nudge(&self.scope_coverage)
                        ));
                    }
                    msg
                } else {
                    build_scope_coverage_nudge(&self.scope_coverage)
                };
                self.tui.push_meta_event("COVERAGE", &self.work_graph_runner.render_progress());
                if graph_incomplete {
                    self.tui.push_meta_event(
                        "FOCUS",
                        "Graph-driven: finalization blocked — work incomplete",
                    );
                }
                self.messages.push(ChatMessage::simple("system", &nudge));
                trace(
                    self.args,
                    &format!(
                        "tool_loop: blocked voluntary finalization for pending scope coverage {}",
                        self.scope_coverage.render_summary()
                    ),
                );
                return Ok(None);
            }
            trace(
                self.args,
                "tool_loop: routing voluntary stop through evidence finalizer (Task 601)",
            );
            let final_content = finalize_from_evidence_or_fallback(
                self.args,
                self.tui,
                self.client,
                self.chat_url,
                self.model_id,
                &self.original_user_request,
                &self.messages,
                self.workdir,
                self.max_tokens,
                None,
            )
            .await;
            let trimmed_final = normalize_final_answer_candidate(&final_content);
            sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
            return Ok(Some(ToolLoopResult {
                final_answer: if final_answer_needs_retry(&trimmed_final) {
                    build_fallback_from_recent_tool_evidence(&self.messages, None)
                } else {
                    trimmed_final
                },
                iterations: self.stop_policy.iteration(),
                tool_calls_made: self.stop_policy.total_tool_calls(),
                stopped_by_max: false,
                stop_outcome: None,
                total_elapsed_s: self.loop_start.elapsed().as_secs() as f64,
                timeout_reason: None,
                evidence_progress_summary: build_evidence_progress_summary(&self.messages),
                loop_summary: self.loop_summary_tracker.clone(),
            }));
        }
        sync_loop_summary_coverage(&mut self.loop_summary_tracker, &self.scope_coverage);
        Ok(Some(ToolLoopResult {
            final_answer: normalize_final_answer_candidate(&content),
            iterations: self.stop_policy.iteration(),
            tool_calls_made: self.stop_policy.total_tool_calls(),
            stopped_by_max: false,
            stop_outcome: None,
            total_elapsed_s: self.loop_start.elapsed().as_secs() as f64,
            timeout_reason: None,
            evidence_progress_summary: build_evidence_progress_summary(&self.messages),
            loop_summary: self.loop_summary_tracker.clone(),
        }))
    }

    fn update_context_estimate(&mut self) {
        let mut total = 0u64;
        for m in &self.messages {
            total += crate::ui_terminal::TerminalUI::estimate_tokens(&m.content);
        }
        self.tui.update_context_tokens(total);
    }

    fn build_continuation_message(
        &self,
        continuation_num: u32,
        max_continuations: u32,
    ) -> String {
        let has_evidence = self.messages
            .iter()
            .filter(|m| m.role == "tool")
            .any(|m| !m.content.is_empty());
        let failed_tools: Vec<String> = self.messages
            .iter()
            .filter(|m| m.role == "tool" && m.content.contains("error"))
            .map(|m| m.name.as_deref().unwrap_or("tool").to_string())
            .collect();
        let successful_tools: Vec<String> = self.messages
            .iter()
            .filter(|m| m.role == "tool" && !m.content.contains("error"))
            .map(|m| m.name.as_deref().unwrap_or("tool").to_string())
            .collect();

        let packet = crate::turn_context_packet::build_turn_context_packet(
            &self.original_user_request,
            if has_evidence {
                "Continue from existing evidence"
            } else {
                &self.original_user_request
            },
            &crate::artifact_verifier::get_required_artifacts(),
            &successful_tools,
            &failed_tools,
            "budget_exceeded",
        );

        crate::turn_context_packet::persist_turn_context_packet(&self.sess.root, &packet);

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
}

const MAX_CONTINUATIONS: u32 = 3;

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
        other => format!("{other}:{}", tc.function.arguments),
    };
    if fn_name == "shell" {
        format!(
            "{fn_name}:{}",
            crate::text_utils::normalize_shell_signal(&key)
        )
    } else {
        format!("{fn_name}:{key}")
    }
}
