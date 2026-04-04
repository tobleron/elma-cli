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
use crate::*;

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
async fn handle_chat_command(runtime: &mut AppRuntime, line: &str) -> Result<bool> {
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
        "/reset" => {
            runtime.messages.truncate(1);
            eprintln!("(history reset)");
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
            eprintln!("(goals reset)");
            handled!()
        }
        "/tools" => {
            handle_discover_tools(runtime)?;
            handled!()
        }
        "/verbose" => {
            runtime.verbose = !runtime.verbose;
            eprintln!("(verbose {})", if runtime.verbose { "on" } else { "off" });
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
    let cmd = format!(
        "ls -R '{path}' | head -n 100; echo '---'; file -b '{path}'/* 2>/dev/null | head -n 10"
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
    runtime: &AppRuntime,
    program: Program,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    rephrased_objective: &str,
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
    loop {
        let prompt = user_prompt_label(&runtime.args);
        let Some(line) = prompt_line(&prompt)? else {
            break;
        };
        let line = line.trim();
        if !handle_chat_command(runtime, line).await? {
            break;
        }
        if line.starts_with('/') {
            continue;
        }

        runtime.messages.push(ChatMessage {
            role: "user".to_string(),
            content: line.to_string(),
        });

        let (rephrased_objective, route_decision) = annotate_and_classify(runtime, line).await?;
        try_workspace_discovery(runtime, line);

        let memories = load_recent_formula_memories(&runtime.model_cfg_dir, 8).unwrap_or_default();
        let (workflow_plan, ladder, complexity, scope, formula, planner_fallback_used) =
            derive_planning_prior_with_ladder(
                &runtime.client,
                &runtime.chat_url,
                &runtime.profiles.workflow_planner_cfg,
                &runtime.profiles.complexity_cfg,
                &runtime.profiles.evidence_need_cfg,
                &runtime.profiles.action_need_cfg,
                &runtime.profiles.scope_builder_cfg,
                &runtime.profiles.formula_cfg,
                line,
                &route_decision,
                &runtime.ws,
                &runtime.ws_brief,
                &memories,
                &runtime.messages,
            )
            .await;
        let src = if planner_fallback_used {
            "fallback_chain"
        } else {
            "ladder_assessment"
        };
        trace(
            &runtime.args,
            &format!("planning_source={src} ladder_level={:?}", ladder.level),
        );
        if let Some(plan) = &workflow_plan {
            trace_workflow_plan(&runtime.args, plan);
        }
        trace_complexity(&runtime.args, &complexity);
        trace_scope(&runtime.args, &scope);
        trace_formula(&runtime.args, &formula);
        let intent = describe_operator_intent(&route_decision, &complexity, &formula);
        operator_trace(&runtime.args, &intent);

        let hierarchy_goal = try_hierarchical_decomposition(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles,
            &runtime.session.root,
            line,
            &complexity,
            &runtime.ws,
            &runtime.ws_brief,
            &runtime.messages,
        )
        .await
        .unwrap_or(None);
        if hierarchy_goal.is_some() {
            trace_verbose(runtime.verbose, "hierarchy_decomposition=triggered");
        }

        let mut program = build_program(
            runtime,
            line,
            &route_decision,
            workflow_plan.as_ref(),
            &complexity,
            &scope,
            &formula,
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
        show_process_step_verbose(
            runtime.verbose,
            "PLAN",
            &format!("{} -> {} steps", complexity.complexity, program.steps.len()),
        );
        if apply_capability_guard(&mut program, &route_decision, !runtime.args.disable_guards) {
            trace_verbose(runtime.verbose, "guard=capability_reply_only");
        }

        let policy_err = validate_formula_level(&formula, ladder.level)
            .err()
            .or_else(|| program_matches_level(&program, ladder.level).err())
            .or_else(|| {
                validate_evidence_requirements(&program, &route_decision, &complexity, &formula)
                    .err()
            });
        if let Some(reason) = policy_err {
            trace(
                &runtime.args,
                &format!(
                    "program_policy_mismatch level={:?} reason={reason}",
                    ladder.level
                ),
            );
            apply_policy_fallback(
                runtime,
                line,
                &route_decision,
                &ladder,
                &complexity,
                &scope,
                &formula,
                &workflow_plan,
                &mut program,
            )
            .await;
            let recheck = program_matches_level(&program, ladder.level)
                .err()
                .or_else(|| {
                    validate_evidence_requirements(&program, &route_decision, &complexity, &formula)
                        .err()
                });
            if let Some(recheck_reason) = recheck {
                operator_trace(&runtime.args, "repairing the workflow plan");
                if let Ok(recovered) = recover_program_once(
                    &runtime.client,
                    &runtime.chat_url,
                    &runtime.profiles.orchestrator_cfg,
                    line,
                    &route_decision,
                    workflow_plan.as_ref(),
                    &complexity,
                    &scope,
                    &formula,
                    &runtime.ws,
                    &runtime.ws_brief,
                    &runtime.messages,
                    &format!("program_policy_mismatch: {recheck_reason}"),
                    Some(&program),
                    &[],
                )
                .await
                {
                    if program_matches_level(&recovered, ladder.level).is_ok()
                        && validate_evidence_requirements(
                            &recovered,
                            &route_decision,
                            &complexity,
                            &formula,
                        )
                        .is_ok()
                    {
                        program = recovered;
                    } else {
                        trace_verbose(
                            runtime.verbose,
                            "workflow_recovery=failed source=program_policy_mismatch",
                        );
                    }
                }
            }
        }

        apply_shape_fallbacks(runtime, line, &ladder, &mut program);
        program = run_reflection_loop(
            runtime,
            program,
            line,
            &route_decision,
            workflow_plan.as_ref(),
            &complexity,
            &scope,
            &formula,
            &rephrased_objective,
        )
        .await;

        let is_trivial = route_decision.route.eq_ignore_ascii_case("CHAT")
            && formula.primary.eq_ignore_ascii_case("reply_only");
        let mut loop_outcome = if is_trivial {
            AutonomousLoopOutcome {
                program: program.clone(),
                step_results: vec![],
                final_reply: None,
                reasoning_clean: true,
            }
        } else {
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
            )
            .await?
        };
        let (mut program, mut step_results, mut final_reply, reasoning_clean) = (
            loop_outcome.program,
            loop_outcome.step_results,
            loop_outcome.final_reply,
            loop_outcome.reasoning_clean,
        );

        let (final_text, final_usage_total) = resolve_final_text(
            runtime,
            line,
            &route_decision,
            &step_results,
            &mut final_reply,
        )
        .await?;
        print_final_output(
            &runtime.args,
            runtime.ctx_max,
            final_usage_total,
            &final_text,
        );
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
        )
        .await?;
        if !final_text.is_empty() {
            runtime.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: final_text,
            });
        }
        if has_edit_result(&step_results) {
            refresh_runtime_workspace(runtime)?;
        }
        let _ = save_goal_state(&runtime.session.root, &runtime.goal_state);
    }
    Ok(())
}
