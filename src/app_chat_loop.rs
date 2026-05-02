//! @efficiency-role: service-orchestrator
//!
//! App Chat - Main Chat Loop Orchestration

use crate::app::*;
use crate::app_chat_builders_advanced::*;
use crate::app_chat_builders_basic::*;
use crate::app_chat_fast_paths::*;
use crate::app_chat_handlers::*;
use crate::app_chat_helpers::*;
use crate::app_chat_orchestrator::*;
use crate::app_chat_patterns::*;
use crate::app_chat_trace::*;
use crate::goal_seeding::*;
use crate::session_write::save_goal_state;
use crate::ui_state::HeaderInfo;
use crate::ui_terminal::{MessageRole, TerminalUI};
use crate::*;
use shlex::quote;
use std::collections::VecDeque;
use std::future::Future;

async fn await_with_busy_queue<T, F>(
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
            _ = tokio::time::sleep(std::time::Duration::from_millis(40)) => {
                tui.pump_ui()?;
                if let Some(queued) = tui.poll_busy_submission()? {
                    queued_inputs.push_back(queued);
                    tui.notify("Queued 1 message (will run after current response)");
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn apply_policy_fallback(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    ladder: &ExecutionLadderAssessment,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _workflow_plan: &Option<WorkflowPlannerOutput>,
    program: &mut Program,
) {
    let Some(path) = extract_first_path_from_user_text(line) else {
        return;
    };
    let fallback: Option<(&str, fn(&str, &str) -> Program)> = match ladder.level {
        ExecutionLevel::Plan => {
            // Task 453 Category 1: Remove stress-test fallback policies
            // These were exercise markers, not production user features
            None
        }
        ExecutionLevel::MasterPlan => {
            // Task 453 Category 1: Remove stress-test fallback policies
            None
        }
        _ => {
            if route_decision.route.eq_ignore_ascii_case("SHELL") {
                if looks_like_natural_language_edit_request(line) {
                    Some((
                        "edit_path_probe_policy_fallback",
                        build_edit_path_probe_program,
                    ))
                } else {
                    Some((
                        "shell_path_probe_policy_fallback",
                        build_shell_path_probe_program,
                    ))
                }
            } else if route_decision.route.eq_ignore_ascii_case("DECIDE") {
                if request_looks_like_scoped_list_request(line) {
                    Some(("shell_list_policy_fallback", build_shell_path_probe_program))
                } else {
                    Some((
                        "decide_path_probe_policy_fallback",
                        build_decide_path_probe_program,
                    ))
                }
            } else {
                None
            }
        }
    };
    if let Some((tag, builder)) = fallback {
        trace(&runtime.args, &format!("{tag} path={path}"));
        *program = builder(line, &path);
    }
}

/// Returns true if the command was handled (should continue loop), false if not a command.
async fn handle_chat_command(
    runtime: &mut AppRuntime,
    line: &str,
    tui: &mut TerminalUI,
) -> Result<bool> {
    if line.is_empty() {
        return Ok(true);
    }
    macro_rules! handled {
        () => {
            Ok(true)
        };
    }
    match line {
        "/exit" | "/quit" => Ok(false),
        "/clear" => {
            runtime.messages.truncate(1);
            tui.clear_messages();
            tui.add_claude_message(crate::claude_ui::ClaudeMessage::System {
                content: "Conversation cleared".to_string(),
            });
            handled!()
        }
        "/sessions" | "/resume" => {
            open_session_picker(runtime, tui);
            handled!()
        }
        "/tasks" => {
            let lines = tui.todo_render_lines();
            if lines.is_empty() {
                tui.add_message(
                    MessageRole::Assistant,
                    "(no tasks yet — task list appears during multi-step work)".to_string(),
                );
            } else {
                tui.add_message(MessageRole::Assistant, lines.join("\n"));
            }
            handled!()
        }

        "/reset" => {
            runtime.messages.truncate(1);
            crate::permission_gate::reset_permission_cache();
            crate::command_budget::reset_budget();
            crate::shell_preflight::clear_confirmation_cache();
            tui.add_message(MessageRole::Assistant, "(history reset, permission cache cleared, command budget reset, confirmation cache cleared)".to_string());
            handled!()
        }
        "/snapshot" => {
            handle_manual_snapshot(runtime)?;
            handled!()
        }
        "/tune" => {
            handle_runtime_tune(runtime).await?;
            handled!()
        }
        "/goals" => {
            handle_show_goals(runtime)?;
            handled!()
        }
        "/reset-goals" => {
            runtime.goal_state.clear();
            tui.add_message(MessageRole::Assistant, "(goals reset)".to_string());
            handled!()
        }
        "/tools" => {
            handle_discover_tools(runtime)?;
            handled!()
        }
        "/verbose" => {
            runtime.verbose = !runtime.verbose;
            tui.add_message(
                MessageRole::Assistant,
                format!("(verbose {})", if runtime.verbose { "on" } else { "off" }).to_string(),
            );
            handled!()
        }
        "/reasoning" => {
            let new_state = crate::toggle_show_reasoning();
            tui.notify(&format!(
                "Reasoning {}",
                if new_state { "ON" } else { "OFF" }
            ));
            handled!()
        }
        "/expand-thinking" => {
            let expanded = tui.claude_transcript_expanded();
            tui.set_claude_transcript_expanded(!expanded);
            tui.notify(&format!(
                "Thinking {}",
                if !expanded { "EXPANDED" } else { "COLLAPSED" }
            ));
            handled!()
        }
        "/help" => {
            use crate::ui_state::ModalState;
            let help_content = format!(
                "GLOBAL:\n\
                 Ctrl+C     Clear input / quit\n\
                 Ctrl+L     Sessions\n\
                 Ctrl+N     New session\n\
                 Ctrl+Shift+S Toggle mouse capture (scroll vs select text)\n\n\
                 CHAT:\n\
                 Enter      Send message\n\
                Ctrl+J     New line\n\
                 Tab        Cycle autocomplete\n\
                 Page Up/Dn Scroll history\n\
                 Up/Down    History / navigate\n\n\
                 INPUT:\n\
                 Ctrl+←/→   Jump word\n\
                 Ctrl+W     Delete word\n\
                 Ctrl+U     Delete to line start\n\
                 Home/End   Start / end of line\n\n\
                 THINKING:\n\
                 Ctrl+T     Expand/collapse all thinking threads\n\
                 Ctrl+O     Toggle task list\n\n\
                 SLASH COMMANDS:\n\
                 /help      Show this help\n\
                 /models    Switch model/provider\n\
                 /provider  Configure endpoint (IP/port)\n\
                 /usage     Token and cost stats\n\
                 /expand-thinking Expand all thinking\n\
                 /approve   Tool approval policy\n\
                 /compact   Compact context\n\
                 /reset     Clear history\n\
                 /snapshot  Create snapshot\n\
                 /tune      Model tuning\n\
                 /tools     Discover tools\n\
                 /verbose   Toggle verbose\n\
                 /reasoning Toggle reasoning visibility\n\
                 /exit      Quit Elma"
            );
            tui.set_modal(ModalState::Help {
                content: help_content,
            });
            handled!()
        }
        "/settings" => {
            use crate::ui_state::ModalState;
            let settings_content = format!(
                "PROVIDER: {}\n\
                 MODEL: {}\n\
                 ENDPOINT: {}\n\
                 APPROVAL: auto\n\
                 WORKSPACE: {}",
                runtime.model_id,
                runtime.model_id,
                runtime.chat_url,
                if runtime.ws_brief.is_empty() {
                    "."
                } else {
                    &runtime.ws_brief
                },
            );
            tui.set_modal(ModalState::Settings {
                content: settings_content,
            });
            handled!()
        }
        "/provider" => {
            // Task 438: Interactive provider endpoint configuration
            crate::app_chat_handlers::handle_provider_config(runtime).await?;
            handled!()
        }
        "/usage" => {
            use crate::ui_state::ModalState;
            let usage_content = format!(
                "Model: {}\n\
                 Context: {} / {} tokens",
                runtime.model_id,
                "0",
                runtime
                    .ctx_max
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            tui.set_modal(ModalState::Usage {
                content: usage_content,
            });
            handled!()
        }
        "/approve" => {
            // Cycle approval policy: Off → Ask → On → Off
            use crate::safe_mode;
            let current = safe_mode::get_safe_mode();
            let next = match current {
                safe_mode::SafeMode::Off => safe_mode::SafeMode::Ask,
                safe_mode::SafeMode::Ask => safe_mode::SafeMode::On,
                safe_mode::SafeMode::On => safe_mode::SafeMode::Off,
            };
            safe_mode::set_safe_mode(next);
            let label = match next {
                safe_mode::SafeMode::Off => "off (yolo — approve all)",
                safe_mode::SafeMode::Ask => "ask (auto — prompt for destructive)",
                safe_mode::SafeMode::On => "on (review — ask before every tool)",
            };
            tui.add_message(
                MessageRole::Assistant,
                format!("(approval policy: {})", label),
            );
            handled!()
        }
        "/compact" => {
            tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactBoundary);
            tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactSummary {
                message_count: runtime.messages.len(),
                context_preview: Some("manual compact".to_string()),
            });
            handled!()
        }
        _ => {
            if let Some(id) = line.strip_prefix("/rollback") {
                handle_manual_rollback(runtime, id.trim())?;
                return handled!();
            }
            if let Some(a) = line.strip_prefix("/api") {
                handle_api_config(runtime, a)?;
                return handled!();
            }
            Ok(true)
        }
    }
}

/// Open the session picker modal with current session list.
fn open_session_picker(runtime: &mut AppRuntime, tui: &mut TerminalUI) {
    let sessions_root = runtime
        .session
        .root
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| runtime.session.root.clone());
    let current_id = runtime
        .session
        .root
        .file_name()
        .map(|s| s.to_string_lossy().to_string());
    let entries =
        crate::session_browser::load_session_picker_entries(&sessions_root, current_id.as_deref());
    tui.set_modal(crate::ui_state::ModalState::SessionPicker {
        entries,
        selected: 0,
        filter: String::new(),
        error: None,
    });
}

// --- Helpers extracted from run_chat_loop ---

async fn annotate_and_classify(
    runtime: &AppRuntime,
    line: &str,
) -> Result<(String, RouteDecision)> {
    let intent = annotate_user_intent(
        &runtime.client,
        &runtime.chat_url,
        &runtime.profiles.intent_helper_cfg,
        line,
        &runtime.messages,
    )
    .await
    .unwrap_or_else(|e| {
        trace_verbose(
            runtime.verbose,
            &format!("intent_helper_failed error={}", e),
        );
        "unknown intent".to_string()
    });
    let intent_only = intent.lines().last().unwrap_or(&intent).trim().to_string();
    let rephrased = format!("{}\n[intent: {}]", line, intent_only);
    trace(&runtime.args, &format!("intent_annotation={}", rephrased));
    let decision = infer_route_prior(
        &runtime.client,
        &runtime.chat_url,
        &runtime.profiles.speech_act_cfg,
        &runtime.profiles.router_cfg,
        &runtime.profiles.mode_router_cfg,
        &runtime.profiles.router_cal,
        &rephrased,
        &runtime.ws,
        &runtime.ws_brief,
        &runtime.messages,
    )
    .await?;
    show_process_step_verbose(
        runtime.verbose,
        "CLASSIFY",
        &format!(
            "speech={} route={} (entropy={:.2})",
            decision.speech_act.choice, decision.route, decision.entropy
        ),
    );
    trace_route_decision(&runtime.args, &decision);
    Ok((rephrased, decision))
}

fn try_workspace_discovery(runtime: &mut AppRuntime, line: &str) {
    let Some(path) = extract_first_path_from_user_text(line) else {
        return;
    };

    // Validate path stays within workspace to prevent directory traversal attacks.
    let canonical_path = match std::fs::canonicalize(&path) {
        Ok(p) => p,
        Err(_) => return,
    };
    let workspace_root = match std::fs::canonicalize(".") {
        Ok(p) => p,
        Err(_) => return,
    };
    if !canonical_path.starts_with(&workspace_root) {
        // Path escapes workspace — silently skip discovery.
        return;
    }

    // Properly quote the path for shell interpolation to prevent injection.
    let safe_path = quote(&path);

    let cmd = format!(
        "ls -R {safe_path} | head -n 100; echo '---'; file -b {safe_path}/* 2>/dev/null | head -n 10"
    );
    let output = crate::workspace::cmd_out(&cmd, &std::path::PathBuf::from("."));
    if !output.trim().is_empty() {
        runtime.ws = format!(
            "### GROUNDED WORKSPACE DISCOVERY ({path})\n{}\n\n{}",
            output.trim(),
            runtime.ws
        );
    }
}

/// Execute a tool by name with arguments
pub(crate) async fn execute_tool(
    runtime: &mut AppRuntime,
    tool_name: &str,
    args: &[String],
) -> Result<String> {
    use tool_discovery::ToolCapability;

    let tool = runtime
        .tool_registry
        .get_tool(tool_name)
        .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?;

    if !tool.available {
        return Err(anyhow::anyhow!("Tool not available: {}", tool_name));
    }

    // Build command from template
    let cmd_template = &tool.invocation;
    let cmd = if args.is_empty() {
        cmd_template.clone()
    } else {
        format!("{} {}", cmd_template, args.join(" "))
    };

    // Execute the command
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&runtime.repo)
        .output()
        .await
        .with_context(|| format!("Failed to execute tool: {}", tool_name))?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!("Tool failed: {}\n{}", tool_name, stderr))
    }
}

fn trace_workflow_plan(args: &Args, plan: &WorkflowPlannerOutput) {
    fn fmt(s: &str) -> &str {
        let s = s.trim();
        if s.is_empty() {
            "-"
        } else {
            s
        }
    }
    trace(
        args,
        &format!(
            "workflow_planner objective={} complexity={} risk={} reason={}",
            fmt(&plan.objective),
            fmt(&plan.complexity),
            fmt(&plan.risk),
            fmt(&plan.reason),
        ),
    );
}

fn apply_shape_fallbacks(
    runtime: &AppRuntime,
    line: &str,
    ladder: &ExecutionLadderAssessment,
    program: &mut Program,
) {
    let try_path = || extract_first_path_from_user_text(line);
    let is_plan = ladder.level == ExecutionLevel::Plan;
    let is_master = ladder.level == ExecutionLevel::MasterPlan;

    // Task 453 Category 1: Remove stress-test shape fallbacks
    // These were exercise markers, not production user features
}

fn has_edit_result(step_results: &[StepResult]) -> bool {
    step_results.iter().any(|s| s.kind == "edit" && s.ok)
}

async fn run_reflection_loop(
    runtime: &mut AppRuntime,
    program: Program,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    rephrased_objective: &str,
    tui: &mut TerminalUI,
) -> Program {
    let is_trivial = route_decision.route.eq_ignore_ascii_case("CHAT")
        && formula.primary.eq_ignore_ascii_case("reply_only");
    if is_trivial {
        return program;
    }

    let features = ClassificationFeatures::from(route_decision);
    let (mut program, mut attempts, mut temp) =
        (program, 0u32, runtime.profiles.orchestrator_cfg.temperature);
    while attempts < 3 {
        let result = reflect_on_program(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.reflection_cfg,
            &program,
            &features,
            &runtime.ws,
            rephrased_objective,
        )
        .await;
        let ok = match result {
            Ok(r) => {
                trace(
                    &runtime.args,
                    &format!(
                        "reflection_confidence={:.2} concerns={} missing={} attempt={}",
                        r.confidence_score,
                        r.concerns.len(),
                        r.missing_points.len(),
                        attempts + 1
                    ),
                );
                show_process_step_verbose(
                    runtime.verbose,
                    "REFLECT",
                    &format!(
                        "confidence={:.0}%{}",
                        r.confidence_score * 100.0,
                        if !r.is_confident { "!" } else { "" }
                    ),
                );
                if !r.is_confident || r.confidence_score < 0.6 {
                    trace_verbose(
                        runtime.verbose,
                        &format!("reflection_warnings={:?}", r.concerns),
                    );
                }
                r.confidence_score >= 0.51
            }
            Err(e) => {
                trace_verbose(runtime.verbose, &format!("reflection_failed error={}", e));
                false
            }
        };
        if ok {
            break;
        }
        attempts += 1;
        if attempts < 3 {
            temp = (temp + 0.2).min(0.8);
            trace(&runtime.args, &format!("program_regenerate orchestrator_temp={temp} reason=reflection_confidence_below_51_percent"));
            program = build_program_with_temp(
                runtime,
                line,
                route_decision,
                workflow_plan,
                complexity,
                scope,
                formula,
                temp,
                tui,
            )
            .await;
        } else {
            trace(
                &runtime.args,
                "reflection_max_attempts_reached proceeding_with_low_confidence_program",
            );
        }
    }
    program
}

pub(crate) async fn run_chat_loop(runtime: &mut AppRuntime) -> Result<()> {
    let mut tui = TerminalUI::new(Some(runtime.session.root.clone()))
        .context("Failed to initialize Terminal UI")?;

    // Initialize safe mode from CLI flag / env var
    if runtime.args.disable_guards {
        crate::safe_mode::set_safe_mode(crate::safe_mode::SafeMode::Off);
    }

    // Mark TUI as active to suppress stderr status messages
    crate::ui_state::set_tui_active(true);

    // Set header info (replaces noisy startup banner)
    let session_name = runtime
        .session
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| runtime.session.root.display().to_string());
    let endpoint = runtime
        .chat_url
        .host_str()
        .map(|h| {
            let port = runtime
                .chat_url
                .port()
                .map(|p| format!(":{}", p))
                .unwrap_or_default();
            format!("{}://{}{}", runtime.chat_url.scheme(), h, port)
        })
        .unwrap_or_else(|| runtime.chat_url.to_string());
    let ws_name = if runtime.ws_brief.is_empty() {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| ".".to_string())
    } else {
        runtime.ws_brief.clone()
    };
    tui.set_header_info(HeaderInfo {
        model: runtime.model_id.clone(),
        endpoint,
        route: String::new(),
        workspace: ws_name,
        session: session_name,
        workflow: String::new(),
        stage: None,
        verbose: runtime.verbose,
    });

    // Initial update for the status bar
    tui.update_status(
        runtime.model_id.clone(),
        0,
        runtime.ctx_max.unwrap_or(0),
        0, // tokens_in
        0, // tokens_out
        "⏱ 0.0s".to_string(),
    );

    let mut queued_inputs: VecDeque<String> = VecDeque::new();
    let mut turn_number: u64 = 0;

    let res = loop {
        queued_inputs.extend(tui.take_queued_submissions());
        let line = if let Some(queued) = queued_inputs.pop_front() {
            queued
        } else {
            let line_opt = tui.run_input_loop().await?;
            let Some(line) = line_opt else {
                break Ok(());
            };
            line.to_string()
        };
        let line = line.trim();
        if !handle_chat_command(runtime, line, &mut tui).await? {
            break Ok(());
        }
        if line.starts_with('/') {
            continue;
        }

        // Task 107: Start effort timer for this turn
        let turn_timer = crate::ui_effort::EffortTimer::start();

        // Clear previous turn's status thread (respects min-visible window)
        tui.clear_status();

        runtime.turn_count += 1;

        tui.add_message(MessageRole::User, line.to_string());
        runtime
            .messages
            .push(ChatMessage::simple("user", &line.to_string()));
        let _ = save_user_prompt_display(&runtime.session, line);

        // T305: Seed goals from multi-step request on first turn
        if turn_number == 0 && !runtime.goal_state.has_active_goal() {
            seed_goals_if_multi_step(line, &mut runtime.goal_state);
            let _ = save_goal_state(&runtime.session.root, &runtime.goal_state);
        }

        // Phase 2 (Task 310): Apply pending turn summary from previous turn
        if let Ok(Some((turn_num, summary))) =
            crate::session_write::load_pending_turn_summary(&runtime.session.root)
        {
            let mut user_msg_count = 0;
            let mut boundary_idx = 0;
            for (i, msg) in runtime.messages.iter().enumerate() {
                if msg.role == "user" && msg.name != Some("turn_summary".to_string()) {
                    user_msg_count += 1;
                    if user_msg_count == turn_num + 1 {
                        boundary_idx = i;
                        break;
                    }
                }
            }
            for msg in runtime.messages.iter_mut().take(boundary_idx) {
                msg.mark_summarized();
            }
            crate::effective_history::inject_turn_summary(&mut runtime.messages, &summary);
            let _ = crate::session_write::mark_summary_applied(&runtime.session.root, turn_num);
        }

        // Show activity indicator while processing
        tui.set_activity("Thinking", "Thinking...");

        // Immediate redraw so user sees submitted message + busy state
        tui.pump_ui()?;

        try_workspace_discovery(runtime, line);

        // Tool discovery and execution (Task 015: Autonomous Tool Discovery)
        if runtime.tool_registry.needs_discovery() {
            if let Ok(registry) = tool_discovery::discover_workspace_tools(&runtime.repo) {
                let tool_count = registry.available_tools().len();
                runtime.tool_registry = registry;
                tui.push_meta_event("TOOLS", &format!("discovered {} tool(s)", tool_count));
            }
        }

        // LLM-driven route inference replaces the old line.len() < 30 heuristic.
        // annotate_and_classify calls infer_route_prior for speech-act, workflow,
        // and mode classification. On failure, fall back to conservative defaults
        // that ALLOW tool access (safe uncertainty).
        // Route classification is no longer needed. The model has all tools
        // and decides what to call via the tool loop. Routing was only used
        // to gate tool access and set execution mode — both now unnecessary.
        let rephrased_objective = line.to_string();
        let route_decision = RouteDecision {
            route: "SHELL".to_string(),
            source: "direct_tool_calling".to_string(),
            margin: 0.0,
            entropy: 0.0,
            distribution: vec![("SHELL".to_string(), 1.0)],
            speech_act: ProbabilityDecision {
                choice: "INSTRUCT".to_string(),
                source: "direct".to_string(),
                distribution: vec![("INSTRUCT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            workflow: ProbabilityDecision {
                choice: "WORKFLOW".to_string(),
                source: "direct".to_string(),
                distribution: vec![("WORKFLOW".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            mode: ProbabilityDecision {
                choice: "EXECUTE".to_string(),
                source: "direct".to_string(),
                distribution: vec![("EXECUTE".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            evidence_required: false,
        };

        // Task 380: Create continuity tracker with route alignment check
        let mut continuity_tracker = crate::continuity::ContinuityTracker::new(
            line.to_string(),
            &route_decision.route,
            "pending",
        );
        continuity_tracker.check_route_alignment(&route_decision);
        if !continuity_tracker.last_checkpoint_is_aligned() {
            trace(
                &runtime.args,
                &format!(
                    "continuity_route_drift score={:.2} reason={:?}",
                    continuity_tracker.alignment_score,
                    continuity_tracker.checkpoints.last().map(|c| &c.reason)
                ),
            );
        }

        let needs_tools = route_decision.evidence_required
            || route_decision.speech_act.choice != "CHAT"
            || route_decision.route != "CHAT";
        let complexity = ComplexityAssessment {
            complexity: if route_decision.route == "CHAT" && !needs_tools {
                "DIRECT"
            } else {
                "INVESTIGATE"
            }
            .to_string(),
            needs_evidence: route_decision.evidence_required,
            needs_tools,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: if needs_tools {
                "inspect_reply"
            } else {
                "reply_only"
            }
            .to_string(),
        };
        let scope = ScopePlan::default();
        let formula = FormulaSelection {
            primary: if needs_tools {
                "inspect_reply"
            } else {
                "reply_only"
            }
            .to_string(),
            alternatives: Vec::new(),
            reason: format!("LLM-route: speech_act={}", route_decision.speech_act.choice),
            memory_id: String::new(),
        };
        let ladder = ExecutionLadderAssessment::new(
            if route_decision.route == "CHAT" && !needs_tools {
                ExecutionLevel::Action
            } else {
                ExecutionLevel::Task
            },
            format!("LLM-driven (source={})", route_decision.source),
            route_decision.evidence_required,
            false,
            false,
            false,
            "LOW".to_string(),
            if route_decision.route == "CHAT" && !needs_tools {
                "DIRECT"
            } else {
                "INVESTIGATE"
            }
            .to_string(),
        );
        let workflow_plan: Option<WorkflowPlannerOutput> = None;

        trace(
            &runtime.args,
            &format!("planning_source=maestro ladder_level={:?}", ladder.level),
        );
        trace(
            &runtime.args,
            &format!(
                "intent_annotation={}",
                rephrased_objective.replace('\n', " ")
            ),
        );

        let hierarchy_goal: Option<Masterplan> = None;

        tui.set_activity("Planning", "Planning...");
        tui.pump_ui()?;

        let mut program = build_program(
            runtime,
            line,
            &route_decision,
            workflow_plan.as_ref(),
            &complexity,
            &scope,
            &formula,
            &mut tui,
        )
        .await;
        if route_decision.route.eq_ignore_ascii_case("CHAT")
            && formula.primary.eq_ignore_ascii_case("reply_only")
        {
            if let Some(path) = extract_first_path_from_user_text(line) {
                trace(
                    &runtime.args,
                    &format!("path_scoped_chat_probe_fallback path={path}"),
                );
                program = build_shell_path_probe_program(line, &path);
            }
        }
        // Skip capability guard and policy validation for Maestro-generated programs
        // The Maestro + Orchestrator pipeline self-validates step generation

        apply_shape_fallbacks(runtime, line, &ladder, &mut program);
        // Skip reflection loop — Maestro + Orchestrator self-validate step generation
        // program = build_program_with_temp(...);  // disabled

        // Redraw after planning so user sees the plan before execution
        tui.pump_ui()?;

        // Tool-calling pipeline produces a single Respond step with pre-built answer.
        // Detect this and skip the legacy orchestration retry chain.
        // All other programs (including CHAT+reply_only) go through retry orchestration.
        let is_tool_calling_result = program.steps.len() == 1
            && matches!(&program.steps[0], Step::Respond { instructions, .. } if !instructions.trim().is_empty());

        let mut loop_outcome = if is_tool_calling_result {
            tui.set_activity("Responding", "Responding...");
            tui.pump_ui()?;
            AutonomousLoopOutcome {
                program: program.clone(),
                step_results: vec![],
                final_reply: None,
                reasoning_clean: true,
            }
        } else {
            tui.set_activity("Executing", "Executing...");
            tui.pump_ui()?;
            orchestrate_with_retries(
                &runtime.args,
                &runtime.client,
                &runtime.chat_url,
                &runtime.session,
                &runtime.repo,
                program,
                &route_decision,
                workflow_plan.as_ref(),
                &complexity,
                &scope,
                &formula,
                &runtime.ws,
                &runtime.ws_brief,
                &runtime.messages,
                &runtime.profiles,
                runtime.args.max_retries,
                runtime.args.retry_temp_step,
                runtime.args.max_retry_temp,
                Some(&mut tui),
            )
            .await?
        };
        let (mut program, mut step_results, mut final_reply, reasoning_clean) = (
            loop_outcome.program,
            loop_outcome.step_results,
            loop_outcome.final_reply,
            loop_outcome.reasoning_clean,
        );

        // Clear coordinator status after execution
        tui.set_coordinator_status("".to_string(), false);

        // Extract final_text and usage (thinking is stripped by isolate_reasoning_fields)
        let (final_text, final_usage_total) = if is_tool_calling_result {
            let answer = match &program.steps[0] {
                Step::Respond { instructions, .. } => instructions.clone(),
                _ => String::new(),
            };
            trace(
                &runtime.args,
                &format!("tool_calling_answer_used length={}", answer.len()),
            );
            (answer, None)
        } else {
            resolve_final_text(
                runtime,
                line,
                &route_decision,
                &step_results,
                &mut final_reply,
                Some(&mut tui),
            )
            .await?
        };

        // Task 380: Post-execution continuity check
        let has_evidence = !step_results.is_empty() && step_results.iter().any(|r| r.ok);
        continuity_tracker.check_final_answer(&final_text, has_evidence);
        trace(
            &runtime.args,
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

        // Task 498: Continuity guard — if score < 0.85, re-prompt once.
        // This is a recoverable quality pass, not a reason to close the TUI.
        let already_retried = runtime
            .messages
            .last()
            .map(|m| m.content.contains("[continuity_retry]"))
            .unwrap_or(false);
        let mut final_text = final_text;
        if continuity_tracker.alignment_score < 0.85 && !already_retried {
            let gap_reason = continuity_tracker.gap();
            let continuity_prompt = format!(
                "[continuity_retry]\nThe previous answer may not fully address your request.\nOriginal request: {}\nIssue detected: {}\n\nPlease provide a more complete answer focused on what was asked.",
                line, gap_reason
            );
            let context_hint = route_decision.route.as_str();
            match crate::orchestration_core::run_tool_calling_pipeline(
                runtime,
                &continuity_prompt,
                &mut tui,
                context_hint,
                route_decision.evidence_required,
            )
            .await
            {
                Ok((retry_text, _, _, _)) => {
                    let retry_text = crate::final_answer::process_final_answer(&retry_text);
                    if !retry_text.trim().is_empty() {
                        final_text = retry_text;
                    }
                }
                Err(e) => {
                    trace(
                        &runtime.args,
                        &format!("continuity_retry_failed_nonfatal error={}", e),
                    );
                    tui.push_meta_event(
                        "RECOVERY",
                        "Continuity retry failed; keeping the best answer already prepared.",
                    );
                }
            }
        }

        // Task 384: Clean-Context Finalization — strip internal framing
        let final_text = crate::final_answer::process_final_answer(&final_text);

        // Task 392: Strip markdown for terminal display (keep original for messages/artifacts)
        let display_text = crate::final_answer::process_final_answer_display(&final_text);

        // Show assistant response (thinking is already stripped from final_text)
        if !final_text.is_empty() {
            tui.add_message(MessageRole::Assistant, display_text);
            runtime
                .messages
                .push(ChatMessage::simple("assistant", &final_text));
            let _ = save_final_answer_display(&runtime.session, &final_text);
        }

        // Clear activity indicator now that processing is complete
        tui.clear_activity();

        // Estimate tokens from message content (~4 chars per token)
        let mut tokens_in: u64 = 0;
        let mut tokens_out: u64 = 0;
        for msg in &runtime.messages {
            let est = TerminalUI::estimate_tokens(&msg.content);
            if msg.role == "assistant" {
                tokens_out += est;
            } else {
                tokens_in += est;
            }
        }
        // Use API-reported tokens if available, otherwise use estimates
        let ctx_tokens = final_usage_total.unwrap_or(tokens_in + tokens_out);
        tui.update_status(
            runtime.model_id.clone(),
            ctx_tokens,
            runtime.ctx_max.unwrap_or(0),
            tokens_in,
            tokens_out,
            turn_timer.format(),
        );
        await_with_busy_queue(
            &mut tui,
            &mut queued_inputs,
            maybe_save_formula_memory(
                &runtime.args,
                &runtime.client,
                &runtime.chat_url,
                &runtime.profiles.memory_gate_cfg,
                &runtime.model_id,
                &runtime.model_cfg_dir,
                line,
                &route_decision,
                &complexity,
                &formula,
                &scope,
                &program,
                &step_results,
                reasoning_clean,
            ),
        )
        .await?;
        if has_edit_result(&step_results) {
            refresh_runtime_workspace(runtime)?;
        }
        let _ = save_goal_state(&runtime.session.root, &runtime.goal_state);

        // Phase 1 (Task 310): Spawn background turn summarizer (fire-and-forget)
        let mut turn_number = runtime
            .messages
            .iter()
            .filter(|m| m.role == "user" && m.name != Some("turn_summary".to_string()))
            .count()
            .saturating_sub(1);
        let tools_used: Vec<String> = step_results
            .iter()
            .filter(|sr| sr.kind == "tool_call")
            .filter_map(|sr| sr.command.clone())
            .collect();
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
        {
            let session_root = runtime.session.root.clone();
            let client = runtime.client.clone();
            let summarizer_cfg = runtime.profiles.turn_summary_cfg.clone();
            let final_text_clone = final_text.clone();
            let route_clone = route_decision.clone();
            let formula_clone = formula.clone();
            let user_message_clone = line.to_string();
            let model_id = runtime.model_id.clone();
            let session_id = runtime
                .session
                .root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            tokio::spawn(async move {
                let unit = crate::intel_units::TurnSummaryUnit::new(summarizer_cfg);
                let context = crate::intel_trait::IntelContext::new(
                    user_message_clone,
                    route_clone,
                    String::new(),
                    String::new(),
                    Vec::new(),
                    client,
                )
                .with_extra("final_text", &final_text_clone)
                .and_then(|c| c.with_extra("step_results", &step_results_json))
                .and_then(|c| c.with_extra("tools_used", &tools_used.join(",")))
                .and_then(|c| c.with_extra("formula", &formula_clone));
                match context {
                    Ok(ctx) => match unit.execute_with_fallback(&ctx).await {
                        Ok(output) => {
                            if let Ok(summary) =
                                serde_json::from_value::<TurnSummaryOutput>(output.data)
                            {
                                let _ = crate::session_write::save_turn_summary(
                                    &session_root,
                                    turn_number,
                                    &summary,
                                );
                                // Task 385: Persist as markdown artifact
                                crate::session_write::write_summary_markdown(
                                    &session_root,
                                    turn_number,
                                    &chrono::Utc::now().to_rfc3339(),
                                    &model_id,
                                    &session_id,
                                    &summary.summary_narrative,
                                    &summary.status_category,
                                    &summary.tools_used,
                                    &summary.errors,
                                );
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Turn summary failed: {}", e);
                        }
                    },
                    Err(e) => {
                        tracing::debug!("Turn summary context build failed: {}", e);
                    }
                }
            });
        }

        queued_inputs.extend(tui.take_queued_submissions());
        turn_number += 1;
    };

    // Mark TUI as inactive
    crate::ui_state::set_tui_active(false);
    tui.cleanup()?;
    res
}
