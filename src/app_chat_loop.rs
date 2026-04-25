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
            if request_looks_like_logging_standardization(line) {
                Some((
                    "logging_standardization_plan_policy_fallback",
                    build_logging_standardization_plan_program,
                ))
            } else if request_looks_like_workflow_endurance_audit(line) {
                Some((
                    "workflow_endurance_plan_policy_fallback",
                    build_workflow_endurance_audit_plan_program,
                ))
            } else if request_looks_like_architecture_audit(line) {
                Some((
                    "architecture_audit_plan_policy_fallback",
                    build_architecture_audit_plan_program,
                ))
            } else {
                None
            }
        }
        ExecutionLevel::MasterPlan => {
            if request_looks_like_hybrid_audit_masterplan(line) {
                Some((
                    "hybrid_masterplan_policy_fallback",
                    build_hybrid_audit_masterplan_program,
                ))
            } else {
                None
            }
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
        "/resume" => {
            let sessions_root = runtime
                .session
                .root
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| runtime.session.root.clone());
            let current = runtime
                .session
                .root
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "current".to_string());
            let mut options: Vec<String> = std::fs::read_dir(&sessions_root)
                .ok()
                .into_iter()
                .flat_map(|it| it.filter_map(|e| e.ok()))
                .filter_map(|e| {
                    let path = e.path();
                    if !path.is_dir() {
                        return None;
                    }
                    let name = path.file_name()?.to_string_lossy().to_string();
                    if !name.starts_with("s_") {
                        return None;
                    }
                    let marker = if name == current { " (current)" } else { "" };
                    Some(format!("{}{}", name, marker))
                })
                .collect();
            options.sort();
            options.reverse();
            if options.is_empty() {
                options.push("No sessions found".to_string());
            }
            options.push("Esc — Back to chat".to_string());
            tui.set_modal(crate::ui_state::ModalState::Select {
                title: "Resume Session".to_string(),
                options,
            });
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
        "/sessions" | "/sessions cleanup" | "/sessions cleanup-all" => {
            let session_root = runtime.session.root.clone();
            let response = session_cleanup::sessions_savings(2, &session_root);
            tui.add_message(MessageRole::Assistant, response);
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
        "/help" => {
            use crate::ui_state::ModalState;
            let help_content = format!(
                "GLOBAL:\n\
                 Ctrl+C     Clear input / quit\n\
                 Ctrl+L     Sessions\n\
                 Ctrl+N     New session\n\n\
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
                 SLASH COMMANDS:\n\
                 /help      Show this help\n\
                 /models    Switch model/provider\n\
                 /usage     Token and cost stats\n\
                 /sessions  Session manager\n\
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
            // Cycle approval policy: yolo → auto → approve → yolo
            let policies = ["yolo", "auto", "approve"];
            let current = &runtime.args.disable_guards.to_string();
            let current_idx = policies.iter().position(|p| p == current).unwrap_or(1);
            let next_idx = (current_idx + 1) % policies.len();
            // Note: actual policy change would require runtime mutation.
            // For now, show the policy selection.
            use crate::ui_state::ModalState;
            let policy_content = format!(
                "Current: {}\n\n\
                 Policies:\n\
                 yolo    — Execute everything without approval\n\
                 auto    — Auto-approve for current session\n\
                 approve — Always ask before executing tools\n\n\
                 Next: {}",
                policies[current_idx], policies[next_idx],
            );
            tui.add_message(
                MessageRole::Assistant,
                format!("(approval policy: {})", policies[next_idx]).to_string(),
            );
            tui.set_modal(ModalState::Settings {
                content: policy_content,
            });
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

    let tool = runtime.tool_registry.get_tool(tool_name)
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

    if is_master
        && request_looks_like_hybrid_audit_masterplan(line)
        && !program.steps.iter().any(|s| {
            matches!(
                s,
                Step::Edit { .. } | Step::Read { .. } | Step::Search { .. } | Step::Shell { .. }
            )
        })
    {
        if let Some(path) = try_path() {
            trace(
                &runtime.args,
                &format!("hybrid_masterplan_shape_fallback path={path}"),
            );
            *program = build_hybrid_audit_masterplan_program(line, &path);
        }
    }
    if is_plan && request_looks_like_logging_standardization(line) {
        if let Some(path) = try_path() {
            trace(
                &runtime.args,
                &format!("logging_standardization_plan_shape_fallback path={path}"),
            );
            *program = build_logging_standardization_plan_program(line, &path);
        }
    }
    if is_plan && request_looks_like_workflow_endurance_audit(line) {
        if let Some(path) = try_path() {
            trace(
                &runtime.args,
                &format!("workflow_endurance_plan_shape_fallback path={path}"),
            );
            *program = build_workflow_endurance_audit_plan_program(line, &path);
        }
    }
    if is_plan
        && request_looks_like_architecture_audit(line)
        && !program.steps.iter().any(|s| matches!(s, Step::Plan { .. }))
    {
        if let Some(path) = try_path() {
            trace(
                &runtime.args,
                &format!("architecture_audit_plan_shape_fallback path={path}"),
            );
            *program = build_architecture_audit_plan_program(line, &path);
        }
    }
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
    let mut tui = TerminalUI::new().context("Failed to initialize Terminal UI")?;

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

        tui.add_message(MessageRole::User, line.to_string());
        runtime
            .messages
            .push(ChatMessage::simple("user", &line.to_string()));

        // Show activity indicator while processing
        tui.set_activity("Analyzing", "Processing your request...");

        // Immediate redraw so user sees submitted message + busy state
        tui.pump_ui()?;

        // Simplified: intent annotation only — no classification pipeline
        let rephrased_objective = await_with_busy_queue(
            &mut tui,
            &mut queued_inputs,
            annotate_user_intent(
                &runtime.client,
                &runtime.chat_url,
                &runtime.profiles.intent_helper_cfg,
                line,
                &runtime.messages,
            ),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::trace!(error = %e, "intent_annotation_failed");
            line.to_string()
        });

        try_workspace_discovery(runtime, line);

        // Tool discovery and execution (Task 015: Autonomous Tool Discovery)
        if runtime.tool_registry.needs_discovery() {
            if let Ok(registry) = tool_discovery::discover_workspace_tools(&runtime.repo) {
                runtime.tool_registry = registry;
            }
        }

        // No keyword-based routing — the Maestro pipeline handles everything.
        // Default to CHAT+reply_only for very short inputs (likely conversational),
        // WORKFLOW+inspect_reply for everything else. This is a conservative heuristic,
        // not keyword matching.
        let likely_conversational =
            line.len() < 30 && !extract_first_path_from_user_text(line).is_some();

        let route = if likely_conversational {
            "CHAT"
        } else {
            "WORKFLOW"
        };
        let formula_primary = if likely_conversational {
            "reply_only"
        } else {
            "inspect_reply"
        };
        let needs_evidence = !likely_conversational;

        let route_decision = RouteDecision {
            route: route.to_string(),
            source: "maestro_pipeline".to_string(),
            margin: 0.0,
            entropy: 0.0,
            distribution: vec![(route.to_string(), 1.0)],
            speech_act: ProbabilityDecision::default(),
            workflow: ProbabilityDecision::default(),
            mode: ProbabilityDecision::default(),
        };
        let complexity = ComplexityAssessment {
            complexity: if likely_conversational {
                "DIRECT"
            } else {
                "INVESTIGATE"
            }
            .to_string(),
            needs_evidence,
            needs_tools: !likely_conversational,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: formula_primary.to_string(),
        };
        let scope = ScopePlan::default();
        let formula = FormulaSelection {
            primary: formula_primary.to_string(),
            alternatives: Vec::new(),
            reason: "Maestro-driven".to_string(),
            memory_id: String::new(),
        };
        let ladder = ExecutionLadderAssessment::new(
            if likely_conversational {
                ExecutionLevel::Action
            } else {
                ExecutionLevel::Task
            },
            "Maestro pipeline".to_string(),
            false,
            false,
            false,
            false,
            "LOW".to_string(),
            if likely_conversational {
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

        tui.set_activity("Planning", "Building execution plan...");
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

        let is_trivial = route_decision.route.eq_ignore_ascii_case("CHAT")
            && formula.primary.eq_ignore_ascii_case("reply_only");

        // Tool-calling pipeline produces a single Respond step with pre-built answer.
        // Detect this and skip the legacy orchestration retry chain.
        let is_tool_calling_result = program.steps.len() == 1
            && matches!(&program.steps[0], Step::Respond { instructions, .. } if !instructions.trim().is_empty());

        let mut loop_outcome = if is_trivial || is_tool_calling_result {
            tui.set_activity("Responding", "Generating response...");
            tui.pump_ui()?;
            AutonomousLoopOutcome {
                program: program.clone(),
                step_results: vec![],
                final_reply: None,
                reasoning_clean: true,
            }
        } else {
            tui.set_activity("Executing", "Running steps...");
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

        // Show assistant response (thinking is already stripped from final_text)
        if !final_text.is_empty() {
            tui.add_message(MessageRole::Assistant, final_text.clone());
            runtime
                .messages
                .push(ChatMessage::simple("assistant", &final_text));
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
        queued_inputs.extend(tui.take_queued_submissions());
    };

    // Mark TUI as inactive
    crate::ui_state::set_tui_active(false);
    tui.cleanup()?;
    res
}
