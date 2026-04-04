//! @efficiency-role: service-orchestrator
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

pub(crate) async fn run_chat_loop(runtime: &mut AppRuntime) -> Result<()> {
    loop {
        let prompt = user_prompt_label(&runtime.args);
        let Some(line) = prompt_line(&prompt)? else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/exit" || line == "/quit" {
            break;
        }
        if line == "/reset" {
            runtime.messages.truncate(1);
            eprintln!("(history reset)");
            continue;
        }
        if line == "/snapshot" {
            handle_manual_snapshot(runtime)?;
            continue;
        }
        if let Some(snapshot_id) = line.strip_prefix("/rollback") {
            handle_manual_rollback(runtime, snapshot_id.trim())?;
            continue;
        }
        if line == "/tune" {
            handle_runtime_tune(runtime).await?;
            continue;
        }
        if line == "/goals" {
            handle_show_goals(runtime)?;
            continue;
        }
        if line == "/reset-goals" {
            runtime.goal_state.clear();
            eprintln!("(goals reset)");
            continue;
        }
        if line == "/tools" {
            handle_discover_tools(runtime)?;
            continue;
        }
        if let Some(api_args) = line.strip_prefix("/api") {
            handle_api_config(runtime, api_args)?;
            continue;
        }
        if line == "/verbose" {
            runtime.verbose = !runtime.verbose;
            eprintln!("(verbose {})", if runtime.verbose { "on" } else { "off" });
            continue;
        }

        runtime.messages.push(ChatMessage {
            role: "user".to_string(),
            content: line.to_string(),
        });

        // Step 1: Annotate user intention with helper (considering conversation context)
        let intent_annotation = annotate_user_intent(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.intent_helper_cfg,
            line,
            &runtime.messages, // Pass full conversation history
        )
        .await
        .unwrap_or_else(|e| {
            trace_verbose(
                runtime.verbose,
                &format!("intent_helper_failed error={}", e),
            );
            "unknown intent".to_string() // Fallback
        });

        // Extract just the intent sentence (model may include original message)
        // Take the last line which should be the intent
        let intent_only = intent_annotation
            .lines()
            .last()
            .unwrap_or(&intent_annotation)
            .trim()
            .to_string();

        // Format: user message + intent annotation (programmatic, not from model)
        let rephrased_objective = format!("{}\n[intent: {}]", line, intent_only);

        trace(
            &runtime.args,
            &format!("intent_annotation={}", rephrased_objective),
        );

        // Step 2: Classify with intent-annotated message
        // The intent annotation helps classifier decide route
        let route_decision = infer_route_prior(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.speech_act_cfg,
            &runtime.profiles.router_cfg,
            &runtime.profiles.mode_router_cfg,
            &runtime.profiles.router_cal,
            &rephrased_objective, // Use intent-annotated message directly
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
                route_decision.speech_act.choice, route_decision.route, route_decision.entropy
            ),
        );
        trace_route_decision(&runtime.args, &route_decision);

        // Task: Grounded workspace discovery for path-scoped requests
        if let Some(path) = extract_first_path_from_user_text(line) {
            // Include language identification in discovery
            let discovery_cmd = format!(
                "ls -R '{}' | head -n 100; echo '---'; file -b '{}'/* 2>/dev/null | head -n 10",
                path, path
            );
            let output = crate::workspace::cmd_out(&discovery_cmd, &std::path::PathBuf::from("."));
            if !output.trim().is_empty() {
                runtime.ws = format!(
                    "### GROUNDED WORKSPACE DISCOVERY ({})\n{}\n\n{}",
                    path,
                    output.trim(),
                    runtime.ws
                );
            }
        }

        let memories = load_recent_formula_memories(&runtime.model_cfg_dir, 8).unwrap_or_default();
        // Task 014: Use new function with confidence fallback
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
        trace(
            &runtime.args,
            &format!(
                "planning_source={} ladder_level={:?}",
                if planner_fallback_used {
                    "fallback_chain"
                } else {
                    "ladder_assessment"
                },
                ladder.level
            ),
        );
        if let Some(plan) = workflow_plan.as_ref() {
            trace(
                &runtime.args,
                &format!(
                    "workflow_planner objective={} complexity={} risk={} reason={}",
                    if plan.objective.trim().is_empty() {
                        "-"
                    } else {
                        plan.objective.trim()
                    },
                    if plan.complexity.trim().is_empty() {
                        "-"
                    } else {
                        plan.complexity.trim()
                    },
                    if plan.risk.trim().is_empty() {
                        "-"
                    } else {
                        plan.risk.trim()
                    },
                    plan.reason.trim()
                ),
            );
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
            &format!("{} → {} steps", complexity.complexity, program.steps.len()),
        );
        let guards_enabled = !runtime.args.disable_guards;
        if apply_capability_guard(&mut program, &route_decision, guards_enabled) {
            trace_verbose(runtime.verbose, "guard=capability_reply_only");
        }

        let formula_level_error = validate_formula_level(&formula, ladder.level).err();
        let program_level_error = program_matches_level(&program, ladder.level).err();
        let evidence_level_error =
            validate_evidence_requirements(&program, &route_decision, &complexity, &formula).err();
        if let Some(policy_reason) = formula_level_error
            .or(program_level_error)
            .or(evidence_level_error)
        {
            trace(
                &runtime.args,
                &format!(
                    "program_policy_mismatch level={:?} reason={policy_reason}",
                    ladder.level
                ),
            );

            if ladder.level == ExecutionLevel::Plan {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    if request_looks_like_logging_standardization(line) {
                        trace(
                            &runtime.args,
                            &format!("logging_standardization_plan_policy_fallback path={path}"),
                        );
                        program = build_logging_standardization_plan_program(line, &path);
                    } else if request_looks_like_workflow_endurance_audit(line) {
                        trace(
                            &runtime.args,
                            &format!("workflow_endurance_plan_policy_fallback path={path}"),
                        );
                        program = build_workflow_endurance_audit_plan_program(line, &path);
                    } else if request_looks_like_architecture_audit(line) {
                        trace(
                            &runtime.args,
                            &format!("architecture_audit_plan_policy_fallback path={path}"),
                        );
                        program = build_architecture_audit_plan_program(line, &path);
                    }
                }
            } else if route_decision.route.eq_ignore_ascii_case("SHELL") {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    if looks_like_natural_language_edit_request(line) {
                        trace(
                            &runtime.args,
                            &format!("edit_path_probe_policy_fallback path={path}"),
                        );
                        program = build_edit_path_probe_program(line, &path);
                    } else {
                        trace(
                            &runtime.args,
                            &format!("shell_path_probe_policy_fallback path={path}"),
                        );
                        program = build_shell_path_probe_program(line, &path);
                    }
                }
            } else if ladder.level == ExecutionLevel::MasterPlan {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    if request_looks_like_hybrid_audit_masterplan(line) {
                        trace(
                            &runtime.args,
                            &format!("hybrid_masterplan_policy_fallback path={path}"),
                        );
                        program = build_hybrid_audit_masterplan_program(line, &path);
                    }
                }
            } else if route_decision.route.eq_ignore_ascii_case("DECIDE") {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    if request_looks_like_scoped_list_request(line) {
                        trace(
                            &runtime.args,
                            &format!("shell_list_policy_fallback path={path}"),
                        );
                        program = build_shell_path_probe_program(line, &path);
                    } else {
                        trace(
                            &runtime.args,
                            &format!("decide_path_probe_policy_fallback path={path}"),
                        );
                        program = build_decide_path_probe_program(line, &path);
                    }
                }
            }

            if let Some(recheck_reason) = program_matches_level(&program, ladder.level)
                .err()
                .or_else(|| {
                    validate_evidence_requirements(&program, &route_decision, &complexity, &formula)
                        .err()
                })
            {
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

        if ladder.level == ExecutionLevel::MasterPlan
            && request_looks_like_hybrid_audit_masterplan(line)
            && !program.steps.iter().any(|step| {
                matches!(
                    step,
                    Step::Edit { .. }
                        | Step::Read { .. }
                        | Step::Search { .. }
                        | Step::Shell { .. }
                )
            })
        {
            if let Some(path) = extract_first_path_from_user_text(line) {
                trace(
                    &runtime.args,
                    &format!("hybrid_masterplan_shape_fallback path={path}"),
                );
                program = build_hybrid_audit_masterplan_program(line, &path);
            }
        }
        if ladder.level == ExecutionLevel::Plan && request_looks_like_logging_standardization(line)
        {
            if let Some(path) = extract_first_path_from_user_text(line) {
                trace(
                    &runtime.args,
                    &format!("logging_standardization_plan_shape_fallback path={path}"),
                );
                program = build_logging_standardization_plan_program(line, &path);
            }
        }
        if ladder.level == ExecutionLevel::Plan && request_looks_like_workflow_endurance_audit(line)
        {
            if let Some(path) = extract_first_path_from_user_text(line) {
                trace(
                    &runtime.args,
                    &format!("workflow_endurance_plan_shape_fallback path={path}"),
                );
                program = build_workflow_endurance_audit_plan_program(line, &path);
            }
        }
        if ladder.level == ExecutionLevel::Plan
            && request_looks_like_architecture_audit(line)
            && !program
                .steps
                .iter()
                .any(|step| matches!(step, Step::Plan { .. }))
        {
            if let Some(path) = extract_first_path_from_user_text(line) {
                trace(
                    &runtime.args,
                    &format!("architecture_audit_plan_shape_fallback path={path}"),
                );
                program = build_architecture_audit_plan_program(line, &path);
            }
        }

        let features = ClassificationFeatures::from(&route_decision);

        if !(route_decision.route.eq_ignore_ascii_case("CHAT")
            && formula.primary.eq_ignore_ascii_case("reply_only"))
        {
            // Reflection remains for operational paths; trivial chat replies skip it.
            let mut orchestrator_temp = runtime.profiles.orchestrator_cfg.temperature;
            let max_program_attempts = 3;
            let mut program_attempts = 0;

            while program_attempts < max_program_attempts {
                match reflect_on_program(
                    &runtime.client,
                    &runtime.chat_url,
                    &runtime.profiles.reflection_cfg,
                    &program,
                    &features,
                    &runtime.ws,
                    &rephrased_objective,
                )
                .await
                {
                    Ok(reflection) => {
                        trace(
                            &runtime.args,
                            &format!(
                                "reflection_confidence={:.2} concerns={} missing={} attempt={}",
                                reflection.confidence_score,
                                reflection.concerns.len(),
                                reflection.missing_points.len(),
                                program_attempts + 1
                            ),
                        );
                        show_process_step_verbose(
                            runtime.verbose,
                            "REFLECT",
                            &format!(
                                "confidence={:.0}%{}",
                                reflection.confidence_score * 100.0,
                                if !reflection.is_confident {
                                    " ⚠️"
                                } else {
                                    ""
                                }
                            ),
                        );
                        if !reflection.is_confident || reflection.confidence_score < 0.6 {
                            trace_verbose(
                                runtime.verbose,
                                &format!("reflection_warnings={:?}", reflection.concerns),
                            );
                        }

                        if reflection.confidence_score >= 0.51 {
                            break;
                        }

                        program_attempts += 1;
                        if program_attempts < max_program_attempts {
                            orchestrator_temp = (orchestrator_temp + 0.2).min(0.8);
                            trace(
                                &runtime.args,
                                &format!(
                                    "program_regenerate orchestrator_temp={} reason=reflection_confidence_below_51_percent",
                                    orchestrator_temp
                                ),
                            );

                            program = build_program_with_temp(
                                runtime,
                                line,
                                &route_decision,
                                workflow_plan.as_ref(),
                                &complexity,
                                &scope,
                                &formula,
                                orchestrator_temp,
                            )
                            .await;
                        } else {
                            trace(
                                &runtime.args,
                                "reflection_max_attempts_reached proceeding_with_low_confidence_program",
                            );
                        }
                    }
                    Err(error) => {
                        trace_verbose(
                            runtime.verbose,
                            &format!("reflection_failed error={}", error),
                        );
                        program_attempts += 1;
                        if program_attempts < max_program_attempts {
                            orchestrator_temp = (orchestrator_temp + 0.2).min(0.8);
                            program = build_program_with_temp(
                                runtime,
                                line,
                                &route_decision,
                                workflow_plan.as_ref(),
                                &complexity,
                                &scope,
                                &formula,
                                orchestrator_temp,
                            )
                            .await;
                        }
                    }
                }
            }
        }

        // For CHAT+reply_only, skip retry loop entirely - just execute the simple reply program
        let mut loop_outcome = if route_decision.route.eq_ignore_ascii_case("CHAT")
            && formula.primary.eq_ignore_ascii_case("reply_only")
        {
            // Direct execution without retries for simple chat replies
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
        let mut program = loop_outcome.program;
        let mut step_results = loop_outcome.step_results;
        let mut final_reply = loop_outcome.final_reply;
        let reasoning_clean = loop_outcome.reasoning_clean;

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
        if step_results
            .iter()
            .any(|step| step.kind == "edit" && step.ok)
        {
            refresh_runtime_workspace(runtime)?;
        }

        let _ = save_goal_state(&runtime.session.root, &runtime.goal_state);
    }

    Ok(())
}
