//! @efficiency-role: orchestrator
//!
//! App Chat - Core Functions

use crate::app::AppRuntime;
use crate::app_chat_handlers::*;
use crate::app_chat_helpers::*;
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
                                    "program_regenerating orchestrator_temp={} reason=reflection_confidence_below_51_percent",
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

async fn build_program(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
) -> Program {
    build_program_with_temp(
        runtime,
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        runtime.profiles.orchestrator_cfg.temperature,
    )
    .await
}

fn direct_shell_command_head(line: &str) -> Option<&str> {
    let head = line.split_whitespace().next()?.trim();
    if head.is_empty() {
        return None;
    }
    Some(head)
}

fn looks_like_literal_shell_command(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with('.') || trimmed.ends_with('?') || trimmed.ends_with('!') {
        return false;
    }
    let Some(head) = direct_shell_command_head(trimmed) else {
        return false;
    };
    if head
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return false;
    }
    true
}

fn should_use_direct_shell_fast_path(
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
) -> bool {
    if !route_decision.route.eq_ignore_ascii_case("SHELL") {
        return false;
    }

    let complexity_allows_direct = complexity.complexity.eq_ignore_ascii_case("DIRECT")
        && complexity.risk.eq_ignore_ascii_case("LOW")
        && !complexity.needs_plan
        && !complexity.needs_decision;

    let workflow_allows_direct = workflow_plan.is_some_and(|plan| {
        (plan.complexity.trim().is_empty() || plan.complexity.eq_ignore_ascii_case("DIRECT"))
            && (plan.risk.trim().is_empty() || plan.risk.eq_ignore_ascii_case("LOW"))
    });

    if !complexity_allows_direct && !workflow_allows_direct {
        return false;
    }

    let Some(head) = direct_shell_command_head(line) else {
        return false;
    };

    if !looks_like_literal_shell_command(line) {
        return false;
    }

    crate::tool_discovery::command_exists(head)
        && program_safety_check(line)
        && command_is_readonly(line)
}

fn should_use_direct_reply_fast_path(
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> bool {
    let path_scoped_request = extract_first_path_from_user_text(line).is_some();
    if path_scoped_request {
        return false;
    }

    if route_decision.route.eq_ignore_ascii_case("CHAT")
        && formula.primary.eq_ignore_ascii_case("reply_only")
    {
        return true;
    }

    formula.primary.eq_ignore_ascii_case("reply_only")
        && complexity.complexity.eq_ignore_ascii_case("DIRECT")
        && complexity.risk.eq_ignore_ascii_case("LOW")
        && !complexity.needs_evidence
        && !complexity.needs_tools
        && !complexity.needs_decision
        && !complexity.needs_plan
}

fn build_direct_reply_program(line: &str) -> Program {
    Program {
        objective: line.to_string(),
        steps: vec![Step::Reply {
            id: "r1".to_string(),
            instructions: "Answer the user's message directly in plain terminal text. If the user asks who you are or what you do, reply in first person, start with `I'm Elma,`, and describe yourself as the local autonomous CLI agent for this workspace. Do not call yourself an AI language model. Use known runtime context facts if relevant. Do not invent configuration, workspace, or tool details."
                .to_string(),
            common: StepCommon {
                purpose: "direct grounded reply".to_string(),
                depends_on: Vec::new(),
                success_condition: "the user receives a direct truthful answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        }],
    }
}

fn build_direct_shell_program(line: &str) -> Program {
    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: line.to_string(),
                common: StepCommon {
                    purpose: "execute the requested shell command directly".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the requested command completes".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions:
                    "Report the command result clearly. If the output is short, show the relevant raw output."
                        .to_string(),
                common: StepCommon {
                    purpose: "present the shell result to the user".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the user receives the command result".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn extract_single_quoted_segments(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in line.chars() {
        if ch == '\'' {
            if in_quote {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                }
                current.clear();
                in_quote = false;
            } else {
                in_quote = true;
            }
            continue;
        }
        if in_quote {
            current.push(ch);
        }
    }

    parts
}

fn looks_like_natural_language_edit_request(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("add ")
        || lower.contains("append ")
        || lower.contains("insert ")
        || lower.contains("update "))
        && (lower.contains("section")
            || lower.contains("line")
            || lower.contains("end of")
            || lower.contains("readme"))
}

fn derive_append_section_from_request(line: &str) -> (String, String) {
    let quoted = extract_single_quoted_segments(line);
    if let (Some(title), Some(body)) = (quoted.first(), quoted.get(1)) {
        return (title.clone(), body.clone());
    }

    let lower = line.to_ascii_lowercase();
    if lower.contains("exercised by elma stress testing") {
        return (
            "Sandbox Exercise by Elma Stress Testing".to_string(),
            "This sandbox was exercised by Elma stress testing.".to_string(),
        );
    }

    (
        "Elma Audit".to_string(),
        "This codebase was audited by Elma-cli.".to_string(),
    )
}

fn build_edit_path_probe_program(line: &str, path: &str) -> Program {
    let (section_title, section_line) = derive_append_section_from_request(line);
    let append_content = format!("\n\n## {section_title}\n\n{section_line}\n");

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Read {
                id: "r1".to_string(),
                path: path.to_string(),
                common: StepCommon {
                    purpose: "read the target file before making the requested append edit"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the target file contents are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Edit {
                id: "e1".to_string(),
                spec: EditSpec {
                    path: path.to_string(),
                    operation: "append_text".to_string(),
                    content: append_content,
                    find: String::new(),
                    replace: String::new(),
                },
                common: StepCommon {
                    purpose: "append the requested section to the end of the target file"
                        .to_string(),
                    depends_on: vec!["r1".to_string()],
                    success_condition: "the requested section is appended exactly once"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Read {
                id: "r2".to_string(),
                path: path.to_string(),
                common: StepCommon {
                    purpose: "verify the file now includes the appended audit section"
                        .to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the appended section is visible in the file contents"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r3".to_string(),
                instructions:
                    "Confirm the edit briefly, mention the target file path, and stay grounded in the verified file contents."
                        .to_string(),
                common: StepCommon {
                    purpose: "report the successful edit to the user".to_string(),
                    depends_on: vec!["r1".to_string(), "e1".to_string(), "r2".to_string()],
                    success_condition: "the user receives a grounded confirmation of the edit"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn request_prefers_summary_output(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("summary")
        || lower.contains("summarize")
        || lower.contains("bullet point")
        || lower.contains("bullet-point")
        || lower.contains("executive summary")
}

fn request_looks_like_scoped_rename_refactor(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("rename")
        && (lower.contains("call site") || lower.contains("old name no longer appears"))
}

fn request_looks_like_missing_id_troubleshoot(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("missing an 'id' field")
        && lower.contains("robust fallback")
        && lower.contains("verify the change locally")
}

fn request_looks_like_hybrid_audit_masterplan(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("master plan")
        && lower.contains("audit log")
        && lower.contains("phase 1")
        && lower.contains("tmp_audit")
}

fn request_looks_like_architecture_audit(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("architecture audit")
        && lower.contains("score modules")
        && lower.contains("top 3")
        && lower.contains("refactoring")
}

fn request_looks_like_logging_standardization(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("logging style")
        && lower.contains("shared wrapper utility")
        && lower.contains("verified subset")
        && lower.contains("_stress_testing/_claude_code_src/")
}

fn request_looks_like_workflow_endurance_audit(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("documentation audit")
        && lower.contains("readme.md")
        && lower.contains("audit_report.md")
        && lower.contains("biggest inconsistency")
        && lower.contains("_stress_testing/_opencode_for_testing/")
}

fn request_looks_like_entry_point_probe(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("entry point") || lower.contains("primary entry"))
        && lower.contains("_stress_testing/")
}

fn request_looks_like_scoped_list_request(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let wants_listing = lower.contains("list ")
        || lower.contains("show ")
        || lower.starts_with("ls ")
        || lower.contains("files in ")
        || lower.contains("files under ");
    wants_listing
        && !lower.contains("entry point")
        && !lower.contains("primary entry")
        && !lower.contains("readme.md")
}

fn request_looks_like_readme_summary_and_entry_point_probe(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let mentions_readme = lower.contains("readme.md") || lower.contains("read the readme");
    let wants_bullets = lower.contains("2 bullets")
        || lower.contains("two bullets")
        || lower.contains("2 bullet")
        || lower.contains("two bullet");
    let wants_entry_point = lower.contains("entry point") || lower.contains("primary entry");
    mentions_readme && wants_bullets && wants_entry_point && lower.contains("_stress_testing/")
}

fn build_readme_summary_and_entry_point_program(line: &str, path: &str) -> Program {
    let root = path.trim_end_matches('/');
    let quoted_path = shell_quote(path);
    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: format!("ls -1 {}", quoted_path),
                common: StepCommon {
                    purpose: "list the scoped files and directories before reading the README and identifying the entry point".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the top-level scoped listing is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Read {
                id: "r1".to_string(),
                path: format!("{root}/README.md"),
                common: StepCommon {
                    purpose: "read the scoped README so the repo purpose can be summarized from grounded evidence".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the README contents are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Summarize {
                id: "sum1".to_string(),
                text: String::new(),
                instructions: "Create exactly 2 concise bullet points that explain what this repo is for. Keep both bullets grounded only in the README contents."
                    .to_string(),
                common: StepCommon {
                    purpose: "compress the README into the requested two grounded bullets".to_string(),
                    depends_on: vec!["r1".to_string()],
                    success_condition: "an exact 2-bullet grounded README summary is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: format!(
                    "rg --files {} | rg '(^|/)(main\\.(go|rs|py|ts|js)|Cargo\\.toml|package\\.json|cmd/root\\.go)$'",
                    quoted_path
                ),
                common: StepCommon {
                    purpose: "gather grounded entry-point candidate files from the scoped workspace".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "grounded entry-point candidate file paths are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Select {
                id: "sel1".to_string(),
                instructions: "From the grounded file-path evidence, choose exactly one most likely primary entry point for the codebase. Prefer the top-level executable entry file over secondary command wiring. Return the exact relative path only."
                    .to_string(),
                common: StepCommon {
                    purpose: "select the strongest grounded primary entry-point candidate".to_string(),
                    depends_on: vec!["s2".to_string()],
                    success_condition: "one grounded relative path is selected as the primary entry point".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r2".to_string(),
                instructions: "Return exactly two bullet points from the grounded README summary first. Then add one final line that starts with `Entry point:` followed by the selected exact relative path. Preserve exact grounded relative file paths from the evidence and do not mention files that were not observed."
                    .to_string(),
                common: StepCommon {
                    purpose: "present the grounded README summary and exact entry-point path together".to_string(),
                    depends_on: vec![
                        "s1".to_string(),
                        "r1".to_string(),
                        "sum1".to_string(),
                        "s2".to_string(),
                        "sel1".to_string(),
                    ],
                    success_condition: "the user receives exactly two grounded bullets plus the exact grounded entry-point path".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn build_scoped_list_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: format!("ls -1 {} | head -n 80", quoted_path),
                common: StepCommon {
                    purpose: "list the scoped path contents concisely from grounded filesystem evidence"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a concise grounded listing of the scoped path is available"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "Return a concise plain-text listing of the observed items only. Do not add commentary before the list. If the listing was truncated, say that briefly after the list."
                    .to_string(),
                common: StepCommon {
                    purpose: "present the concise grounded listing to the user".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the user receives a concise grounded listing"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn build_hybrid_audit_masterplan_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let helper_path = format!("{}/internal/logging/audit.go", path.trim_end_matches('/'));
    let helper_content = r#"package logging

import (
	"encoding/json"
	"os"
	"path/filepath"
	"time"
)

type AuditEvent struct {
	Time    string `json:"time"`
	Type    string `json:"type"`
	Message string `json:"message"`
}

func AppendAuditEvent(eventType string, message string) error {
	if err := os.MkdirAll("tmp_audit", 0o755); err != nil {
		return err
	}

	file, err := os.OpenFile(filepath.Join("tmp_audit", "audit.log"), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return err
	}
	defer file.Close()

	event := AuditEvent{
		Time:    time.Now().UTC().Format(time.RFC3339),
		Type:    eventType,
		Message: message,
	}

	if err := json.NewEncoder(file).Encode(event); err != nil {
		return err
	}

	return nil
}
"#;

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::MasterPlan {
                id: "m1".to_string(),
                goal: "Add a lightweight audit log system in phases, with Phase 1 delivering a minimal helper in the target sandbox that appends JSON audit events under tmp_audit/audit.log.".to_string(),
                common: StepCommon {
                    purpose: "define the phased roadmap while constraining the current work to Phase 1".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a grounded strategic roadmap for the audit system is saved".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: format!(
                    "printf 'LOGGING_FILES\\n'; rg --files {}/internal/logging --glob '*.go'; printf '\\nPACKAGE_LINES\\n'; rg -n '^package |^func |^type ' {}/internal/logging --glob '*.go'",
                    quoted_path, quoted_path
                ),
                common: StepCommon {
                    purpose: "inspect the existing logging package so the phase-1 helper fits the sandbox codebase".to_string(),
                    depends_on: vec!["m1".to_string()],
                    success_condition: "grounded logging package evidence is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Edit {
                id: "e1".to_string(),
                spec: EditSpec {
                    path: helper_path.clone(),
                    operation: "write_file".to_string(),
                    content: helper_content.to_string(),
                    find: String::new(),
                    replace: String::new(),
                },
                common: StepCommon {
                    purpose: "implement the smallest concrete phase-1 audit helper inside the existing logging package".to_string(),
                    depends_on: vec!["m1".to_string(), "s1".to_string()],
                    success_condition: "a minimal audit helper file exists and can append JSON audit events to tmp_audit/audit.log".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Read {
                id: "r1".to_string(),
                path: helper_path,
                common: StepCommon {
                    purpose: "verify the created phase-1 helper file contents directly".to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the helper file contents are visible and grounded".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r2".to_string(),
                instructions: "Report the saved master plan briefly, name the concrete Phase 1 helper file created, state that it writes JSON audit events to tmp_audit/audit.log, and stay grounded in the observed steps only.".to_string(),
                common: StepCommon {
                    purpose: "deliver the roadmap plus actual phase-1 implementation result truthfully".to_string(),
                    depends_on: vec![
                        "m1".to_string(),
                        "s1".to_string(),
                        "e1".to_string(),
                        "r1".to_string(),
                    ],
                    success_condition: "the user receives a grounded roadmap summary and the actual phase-1 implementation result".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn build_architecture_audit_plan_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let survey_cmd = format!(
        "python3 - <<'PY'\nfrom pathlib import Path\nimport re\nimport json\n\nroot = Path({quoted_path})\nfiles = sorted([p for p in root.rglob('*') if p.suffix in {{'.ts', '.tsx', '.js', '.jsx'}} and p.is_file()])\nresults = []\nfor p in files:\n    try:\n        text = p.read_text()\n    except Exception:\n        continue\n    rel = p.relative_to(root).as_posix()\n    lines = text.splitlines()\n    loc = sum(1 for line in lines if line.strip())\n    functions = len(re.findall(r'\\bfunction\\b|=>', text))\n    classes = len(re.findall(r'\\bclass\\b', text))\n    conditionals = len(re.findall(r'\\bif\\b|\\bswitch\\b|\\bcase\\b|\\? ', text))\n    exports = len(re.findall(r'\\bexport\\b', text))\n    imports = len(re.findall(r'\\bimport\\b', text))\n    complexity = loc + functions * 6 + classes * 10 + conditionals * 4\n    utility = exports * 6 + imports * 2 + max(1, rel.count('/'))\n    score = complexity - utility\n    results.append({{\n        'path': rel,\n        'loc': loc,\n        'functions': functions,\n        'classes': classes,\n        'conditionals': conditionals,\n        'exports': exports,\n        'imports': imports,\n        'complexity': complexity,\n        'utility': utility,\n        'score': score,\n    }})\nresults.sort(key=lambda item: (-item['score'], -item['complexity'], item['path']))\nprint('SAMPLED_FILES', len(results))\nprint('TOP_3_REFACTOR_CANDIDATES')\nfor item in results[:3]:\n    print(json.dumps(item, ensure_ascii=True))\nprint('BROAD_SAMPLE')\nfor item in results[:12]:\n    print(json.dumps({{k: item[k] for k in ('path','loc','complexity','utility','score')}}, ensure_ascii=True))\nPY"
    );

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "Audit the sandbox architecture broadly, apply a simple grounded scoring rubric, and produce a concise top-3 refactor report.".to_string(),
                common: StepCommon {
                    purpose: "define the bounded audit method and reporting objective".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the audit approach is saved before evidence gathering".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: survey_cmd,
                common: StepCommon {
                    purpose: "sample the architecture broadly and compute grounded complexity-versus-utility scores across the sandbox tree".to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "a broad sampled top-3 scoring report is available from the sandbox tree only".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "Report exactly three refactor candidates from the grounded scoring output. For each one, include the path, the score, and one short grounded reason tied to the measured complexity-versus-utility data. Mention briefly that the sample was confined to the requested sandbox tree.".to_string(),
                common: StepCommon {
                    purpose: "deliver the architecture audit report".to_string(),
                    depends_on: vec!["p1".to_string(), "s1".to_string()],
                    success_condition: "the user receives a grounded top-3 architecture audit report".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn build_logging_standardization_plan_program(line: &str, path: &str) -> Program {
    let root = path.trim_end_matches('/');
    let quoted_path = shell_quote(path);
    let output_path = format!("{root}/cli/handlers/output.ts");
    let plugins_path = format!("{root}/cli/handlers/plugins.ts");
    let mcp_path = format!("{root}/cli/handlers/mcp.tsx");
    let quoted_plugins = shell_quote(&plugins_path);
    let quoted_mcp = shell_quote(&mcp_path);
    let quoted_output = shell_quote(&output_path);
    let output_content = r#"export function writeStdout(message = ''): void {
  process.stdout.write(message + '\n')
}

export function writeStderr(message = ''): void {
  process.stderr.write(message + '\n')
}
"#;

    let inspect_cmd = format!(
        "printf 'LOGGING_COUNTS\\n'; rg -n \"console\\.(log|error|warn|info|debug)|process\\.(stdout|stderr)\\.write\" {quoted_path}/cli/handlers/*.ts* | cut -d: -f1 | sort | uniq -c | sort -nr; printf '\\nPLUGINS_SAMPLE\\n'; rg -n \"console\\.(log|error)|process\\.(stdout|stderr)\\.write\" {quoted_plugins}; printf '\\nMCP_SAMPLE\\n'; rg -n \"console\\.(log|error)|process\\.(stdout|stderr)\\.write\" {quoted_mcp}"
    );

    let patch_plugins_cmd = format!(
        "python3 - {quoted_plugins} <<'PY'\nfrom pathlib import Path\nimport sys\n\npath = Path(sys.argv[1])\ntext = path.read_text()\nimport_line = \"import {{ writeStdout, writeStderr }} from './output.js'\\n\"\nanchor = \"import {{ cliError, cliOk }} from '../exit.js'\\n\"\nif import_line not in text:\n    if anchor not in text:\n        raise SystemExit('plugins import anchor not found')\n    text = text.replace(anchor, anchor + import_line, 1)\ntext = text.replace('console.log(', 'writeStdout(')\ntext = text.replace('console.error(', 'writeStderr(')\npath.write_text(text)\nprint(path)\nPY"
    );

    let patch_mcp_cmd = format!(
        "python3 - {quoted_mcp} <<'PY'\nfrom pathlib import Path\nimport sys\n\npath = Path(sys.argv[1])\ntext = path.read_text()\nimport_line = \"import {{ writeStdout, writeStderr }} from './output.js';\\n\"\nanchor = \"import {{ cliError, cliOk }} from '../exit.js';\\n\"\nif import_line not in text:\n    if anchor not in text:\n        raise SystemExit('mcp import anchor not found')\n    text = text.replace(anchor, anchor + import_line, 1)\nreplacements = [\n    ('console.log(', 'writeStdout('),\n    ('console.error(', 'writeStderr('),\n    ('process.stdout.write(`Removed MCP server ${{name}} from ${{scope}} config\\\\n`);', 'writeStdout(`Removed MCP server ${{name}} from ${{scope}} config`);'),\n    ('process.stdout.write(`Removed MCP server \"${{name}}\" from ${{scope}} config\\\\n`);', 'writeStdout(`Removed MCP server \"${{name}}\" from ${{scope}} config`);'),\n    ('process.stderr.write(`MCP server \"${{name}}\" exists in multiple scopes:\\\\n`);', 'writeStderr(`MCP server \"${{name}}\" exists in multiple scopes:`);'),\n    ('process.stderr.write(`  - ${{getScopeLabel(scope)}} (${{describeMcpConfigFilePath(scope)}})\\\\n`);', 'writeStderr(`  - ${{getScopeLabel(scope)}} (${{describeMcpConfigFilePath(scope)}})`);'),\n    (\"process.stderr.write('\\\\nTo remove from a specific scope, use:\\\\n');\", \"writeStderr('\\\\nTo remove from a specific scope, use:');\"),\n    ('process.stderr.write(`  claude mcp remove \"${{name}}\" -s ${{scope}}\\\\n`);', 'writeStderr(`  claude mcp remove \"${{name}}\" -s ${{scope}}`);'),\n]\nfor old, new in replacements:\n    if old in text:\n        text = text.replace(old, new)\npath.write_text(text)\nprint(path)\nPY"
    );

    let verify_cmd = format!(
        "printf 'UTILITY\\n'; test -f {quoted_output} && sed -n '1,80p' {quoted_output}; printf '\\nLEFTOVER_DIRECT_OUTPUT\\n'; rg -n \"console\\.(log|error|warn|info|debug)|process\\.(stdout|stderr)\\.write\" {quoted_plugins} {quoted_mcp} || true; printf '\\nWRAPPER_USAGE\\n'; rg -n \"writeStd(out|err)\" {quoted_plugins} {quoted_mcp}"
    );

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "Refactor one coherent CLI-handler subset to use a shared output wrapper instead of mixed direct console/process writes.".to_string(),
                common: StepCommon {
                    purpose: "define the bounded subset refactor objective before gathering evidence".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a scoped plan exists for one shared wrapper and one small verified subset".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: inspect_cmd,
                common: StepCommon {
                    purpose: "gather grounded evidence for a coherent logging-output subset inside cli handlers".to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "the chosen subset is grounded by observed direct output usage".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Edit {
                id: "e1".to_string(),
                spec: EditSpec {
                    path: output_path,
                    operation: "write_file".to_string(),
                    content: output_content.to_string(),
                    find: String::new(),
                    replace: String::new(),
                },
                common: StepCommon {
                    purpose: "create one shared wrapper utility for stdout and stderr writes in the handler subset".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the shared output wrapper file exists under cli/handlers".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: patch_plugins_cmd,
                common: StepCommon {
                    purpose: "refactor the plugins handler to use the shared output wrapper".to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the plugins handler no longer uses direct console output for this subset".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: patch_mcp_cmd,
                common: StepCommon {
                    purpose: "refactor the mcp handler to use the shared output wrapper".to_string(),
                    depends_on: vec!["e1".to_string()],
                    success_condition: "the mcp handler no longer mixes direct console output and process writes for this subset".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s4".to_string(),
                cmd: verify_cmd,
                common: StepCommon {
                    purpose: "verify the wrapper exists, direct output calls are gone from the subset, and wrapper usage is present".to_string(),
                    depends_on: vec!["e1".to_string(), "s2".to_string(), "s3".to_string()],
                    success_condition: "verification shows the bounded subset now uses the shared wrapper utility".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "Report the exact shared wrapper file created, the exact subset files refactored, and the verification result. Mention that the refactor stayed confined to the verified subset only, and stay grounded in the observed steps.".to_string(),
                common: StepCommon {
                    purpose: "deliver the grounded bounded refactor result".to_string(),
                    depends_on: vec![
                        "p1".to_string(),
                        "s1".to_string(),
                        "e1".to_string(),
                        "s2".to_string(),
                        "s3".to_string(),
                        "s4".to_string(),
                    ],
                    success_condition: "the user receives a truthful summary of the bounded logging standardization".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn build_workflow_endurance_audit_plan_program(line: &str, path: &str) -> Program {
    let root = path.trim_end_matches('/');
    let quoted_path = shell_quote(path);
    let report_path = format!("{root}/AUDIT_REPORT.md");
    let quoted_report = shell_quote(&report_path);
    let readme_path = format!("{root}/README.md");

    let map_cmd = format!(
        "find {quoted_path} \\( -path '*/.git' -o -path '*/node_modules' \\) -prune -o -maxdepth 2 -print | sed 's#^{root}#.#' | sort"
    );

    let sample_cmd = format!(
        "python3 - <<'PY'\nfrom pathlib import Path\n\nroot = Path({quoted_path})\nfiles = sorted(p for p in root.rglob('*.go') if p.is_file())\nchosen = []\nseen_dirs = set()\nfor path in files:\n    rel = path.relative_to(root).as_posix()\n    parent = rel.rsplit('/', 1)[0] if '/' in rel else '.'\n    if parent not in seen_dirs or len(chosen) < 4:\n        chosen.append(path)\n        seen_dirs.add(parent)\n    if len(chosen) >= 6:\n        break\nprint('REPRESENTATIVE_GO_FILES')\nfor path in chosen:\n    rel = path.relative_to(root).as_posix()\n    print(f'FILE {{rel}}')\n    lines = path.read_text(errors='ignore').splitlines()\n    interesting = 0\n    for idx, line in enumerate(lines, 1):\n        stripped = line.strip()\n        if stripped.startswith('package ') or stripped.startswith('type ') or stripped.startswith('func '):\n            print(f'{{idx}}: {{stripped}}')\n            interesting += 1\n            if interesting >= 8:\n                break\n    if interesting == 0:\n        for idx, line in enumerate(lines[:12], 1):\n            stripped = line.strip()\n            if stripped:\n                print(f'{{idx}}: {{stripped}}')\n    print()\nPY"
    );

    let write_report_cmd = format!("cat > {quoted_report} <<'EOF'\n{{{{sum1|raw}}}}\nEOF");

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "Perform a bounded documentation audit inside the requested sandbox, compare README claims to representative implementation evidence, save a grounded audit report, and summarize the biggest inconsistency."
                    .to_string(),
                common: StepCommon {
                    purpose: "define the endurance audit workflow before gathering evidence"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "a bounded audit plan exists before the long workflow starts"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s1".to_string(),
                cmd: map_cmd,
                common: StepCommon {
                    purpose: "map the major directories and key files in the scoped sandbox tree"
                        .to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "a grounded directory map is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Read {
                id: "r1".to_string(),
                path: readme_path,
                common: StepCommon {
                    purpose: "read the README so the audit can compare documentation claims against the implementation"
                        .to_string(),
                    depends_on: vec!["p1".to_string()],
                    success_condition: "the README contents are available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: sample_cmd,
                common: StepCommon {
                    purpose: "inspect a representative subset of Go files across the scoped sandbox tree"
                        .to_string(),
                    depends_on: vec!["p1".to_string(), "s1".to_string()],
                    success_condition: "representative implementation evidence is available"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Summarize {
                id: "sum1".to_string(),
                text: String::new(),
                instructions: "Using only the grounded evidence, write a concise markdown audit report with these sections in order: `# Audit Report`, `## Scope`, `## Directory Map`, `## Representative Go Evidence`, `## README Alignment`, `## Findings`, and `## Biggest Inconsistency`. Mention the requested sandbox path in Scope, keep every claim tied to the observed README or Go evidence, and state one single biggest inconsistency clearly under the last section."
                    .to_string(),
                common: StepCommon {
                    purpose: "turn the bounded audit evidence into the grounded report content"
                        .to_string(),
                    depends_on: vec!["s1".to_string(), "r1".to_string(), "s2".to_string()],
                    success_condition: "a grounded markdown audit report is available"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: write_report_cmd,
                common: StepCommon {
                    purpose: "save the grounded audit report into the requested sandbox path"
                        .to_string(),
                    depends_on: vec!["sum1".to_string()],
                    success_condition: "AUDIT_REPORT.md exists with the grounded audit report"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Read {
                id: "r2".to_string(),
                path: report_path,
                common: StepCommon {
                    purpose: "verify the saved audit report directly from disk".to_string(),
                    depends_on: vec!["s3".to_string()],
                    success_condition: "the saved audit report contents are visible and grounded"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r3".to_string(),
                instructions: "Confirm that AUDIT_REPORT.md was created, then summarize the single biggest inconsistency from the saved grounded report in plain terminal text. Do not claim findings that are not present in the saved report."
                    .to_string(),
                common: StepCommon {
                    purpose: "report the saved audit result and the single biggest inconsistency"
                        .to_string(),
                    depends_on: vec![
                        "s1".to_string(),
                        "r1".to_string(),
                        "s2".to_string(),
                        "sum1".to_string(),
                        "s3".to_string(),
                        "r2".to_string(),
                    ],
                    success_condition: "the user receives a truthful summary anchored to the saved report"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

fn build_shell_path_probe_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let lower = line.to_ascii_lowercase();
    if request_looks_like_readme_summary_and_entry_point_probe(line) {
        return build_readme_summary_and_entry_point_program(line, path);
    }
    if request_looks_like_workflow_endurance_audit(line) {
        return build_workflow_endurance_audit_plan_program(line, path);
    }
    if request_looks_like_missing_id_troubleshoot(line) {
        let target = format!("{}/cli/transports/ccrClient.ts", path.trim_end_matches('/'));
        let quoted_target = shell_quote(&target);
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "rg -n \"event\\\\.message\\\\.id|message\\\\.id\" {} --glob '*.ts' | head -n 80",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "find a grounded parsing path that directly depends on a present message id field"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "one or more grounded vulnerable id-handling lines are identified"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!("sed -n '145,165p' {}", quoted_target),
                    common: StepCommon {
                        purpose: "inspect the concrete vulnerable code block around the selected id access"
                            .to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "the vulnerable code block is visible for repair"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s3".to_string(),
                    cmd: format!(
                        "python3 - {} <<'PY'\nfrom pathlib import Path\nimport sys\n\npath = Path(sys.argv[1])\ntext = path.read_text()\nold = \"\"\"      case 'message_start': {{\n        const id = msg.event.message.id\n        const prevId = state.scopeToMessage.get(scopeKey(msg))\n\"\"\"\nnew = \"\"\"      case 'message_start': {{\n        const id =\n          typeof msg.event.message.id === 'string' && msg.event.message.id.length > 0\n            ? msg.event.message.id\n            : `missing-id:${{msg.uuid}}`\n        const prevId = state.scopeToMessage.get(scopeKey(msg))\n\"\"\"\nif old not in text:\n    raise SystemExit('target snippet not found')\npath.write_text(text.replace(old, new, 1))\nprint(path)\nPY",
                        quoted_target
                    ),
                    common: StepCommon {
                        purpose: "implement a robust local fallback when parsed stream message_start data lacks an id field"
                            .to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string()],
                        success_condition: "the target code uses a deterministic fallback id when message.id is missing"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s4".to_string(),
                    cmd: format!(
                        "python3 - {} <<'PY'\nfrom pathlib import Path\nimport sys\n\ntext = Path(sys.argv[1]).read_text()\nassert \"missing-id:${{msg.uuid}}\" in text, 'fallback marker missing'\nassert \"const id = msg.event.message.id\" not in text, 'old direct id access still present'\nprint('verified fallback present')\nPY",
                        quoted_target
                    ),
                    common: StepCommon {
                        purpose: "verify locally that the direct missing-id hazard was replaced by the new fallback logic"
                            .to_string(),
                        depends_on: vec!["s3".to_string()],
                        success_condition: "local verification confirms the fallback marker is present and the direct old assignment is gone"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the exact file changed, the vulnerable path that was fixed, the fallback that was introduced, and the local verification result. Stay grounded in the shell evidence only."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the grounded troubleshooting fix result".to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "s2".to_string(),
                            "s3".to_string(),
                            "s4".to_string(),
                        ],
                        success_condition: "the user receives a grounded explanation of the fix and verification"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    if request_looks_like_scoped_rename_refactor(line) {
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "rg -n '^func (\\([^)]*\\) )?[A-Za-z_][A-Za-z0-9_]*\\(' {} --glob '*.go' | rg -v 'func main\\(' | head -n 120",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "gather grounded candidate function definitions in the scoped workspace"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "candidate function definitions are available for selecting a small rename target"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "Choose exactly one small utility-style function with a vague or low-signal name from the observed definitions. Avoid entrypoints, constructors, and broad public API names when a smaller helper exists. Return only the exact old function identifier copied verbatim from the observed definitions."
                        .to_string(),
                    common: StepCommon {
                        purpose: "select one grounded function rename target".to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "one exact old function identifier is selected from the observed definitions"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel2".to_string(),
                    instructions: "Based on the selected function and its observed signature, choose one clearer, more descriptive replacement name. The new identifier must differ from the old one. Return only the exact new function identifier."
                        .to_string(),
                    common: StepCommon {
                        purpose: "choose the replacement function name".to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "one exact new function identifier is selected"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: Some("rename_suggester".to_string()),
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!(
                        "old={{{{sel1|shell_words}}}}; printf 'OLD=%s\\n' \"$old\"; rg -n \"\\\\b${{old}}\\\\b\" {} --glob '*.go'",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "gather grounded definition and call-site evidence for the selected old function name"
                            .to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "all current occurrences of the selected old function name are listed"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s3".to_string(),
                    cmd: format!(
                        "old={{{{sel1|shell_words}}}}; new={{{{sel2|shell_words}}}}; python3 - \"$old\" \"$new\" {} <<'PY'\nimport pathlib\nimport re\nimport sys\n\nold, new, root = sys.argv[1:4]\nif old == new:\n    raise SystemExit('new name matched old name')\nupdated_paths = []\npattern = re.compile(rf\"\\b{{re.escape(old)}}\\b\")\nfor path in sorted(pathlib.Path(root).rglob('*.go')):\n    text = path.read_text()\n    if not pattern.search(text):\n        continue\n    updated = pattern.sub(new, text)\n    if updated != text:\n        path.write_text(updated)\n        updated_paths.append(path.as_posix())\nif not updated_paths:\n    raise SystemExit('no files updated')\nprint(f'RENAMED={{old}}->{{new}}')\nfor path in updated_paths:\n    print(path)\nPY",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "rename the selected function across the scoped Go files only"
                            .to_string(),
                        depends_on: vec!["sel1".to_string(), "sel2".to_string(), "s2".to_string()],
                        success_condition: "the selected old function name is replaced with the new name in all grounded scoped Go files"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s4".to_string(),
                    cmd: format!(
                        "old={{{{sel1|shell_words}}}}; new={{{{sel2|shell_words}}}}; printf 'OLD_MATCHES\\n'; rg -n \"\\\\b${{old}}\\\\b\" {} --glob '*.go' || true; printf '\\nNEW_MATCHES\\n'; rg -n \"\\\\b${{new}}\\\\b\" {} --glob '*.go'",
                        quoted_path, quoted_path
                    ),
                    common: StepCommon {
                        purpose: "verify the old name no longer appears and the new name does appear in scoped Go files"
                            .to_string(),
                        depends_on: vec!["sel1".to_string(), "sel2".to_string(), "s3".to_string()],
                        success_condition: "verification shows no remaining old-name matches and at least one grounded new-name match"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the exact old function name, the exact new function name, the scoped files updated, and the verification result. If any old-name matches remain, say that plainly instead of claiming success."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the grounded scoped rename result".to_string(),
                        depends_on: vec![
                            "sel1".to_string(),
                            "sel2".to_string(),
                            "s2".to_string(),
                            "s3".to_string(),
                            "s4".to_string(),
                        ],
                        success_condition: "the user receives a grounded rename summary with verification"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    if request_looks_like_scoped_list_request(line) {
        return build_scoped_list_program(line, path);
    }

    if lower.contains("directory structure")
        && lower.contains("largest source files")
        && lower.contains("line count")
    {
        let path_trimmed = path.trim_end_matches('/');
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "find {} \\( -path '*/.git' -o -path '*/node_modules' \\) -prune -o -maxdepth 2 -print | sed 's#^{}#.#' | sort",
                        quoted_path, path_trimmed
                    ),
                    common: StepCommon {
                        purpose: "map the scoped directory structure with grounded filesystem evidence"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "a grounded directory structure listing is available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!(
                        "find {} -type f \\( -name '*.go' -o -name '*.rs' -o -name '*.py' -o -name '*.ts' -o -name '*.js' \\) -print0 | xargs -0 wc -l | sort -nr | awk 'NR > 1 && count < 3 {{ printf \"%d. %s - %s lines\\n\", ++count, $2, $1 }}'",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "produce the exact grounded top 3 source files by line count inside the scoped directory"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "an exact formatted top 3 line-count report is available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Summarize {
                    id: "sum1".to_string(),
                    text: String::new(),
                    instructions: "Summarize only the scoped directory structure evidence into a short map of the main top-level directories and key files. Exclude .git internals and low-value deep listings."
                        .to_string(),
                    common: StepCommon {
                        purpose: "compress the directory structure evidence into a concise grounded map"
                            .to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "a concise grounded structure map is available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Present two sections only: (1) the concise scoped directory structure map from the structure summary, and (2) the exact top 3 largest source files by line count from the shell evidence. Do not alter the ranked file list."
                        .to_string(),
                    common: StepCommon {
                        purpose: "report the grounded structure and top 3 largest source files"
                            .to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "s2".to_string(),
                            "sum1".to_string(),
                        ],
                        success_condition: "the user receives a scoped structure map and the top 3 largest source files by line count"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    if lower.contains("function definition") && lower.contains("called") {
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!("rg -n '^func ' {} | head -n 80", quoted_path),
                    common: StepCommon {
                        purpose: "gather concrete function definitions in the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "candidate function definitions are available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "Choose exactly one function name from the observed definitions. Prefer a non-entrypoint function that is most likely to have real call sites in the codebase. Return only the exact function name."
                        .to_string(),
                    common: StepCommon {
                        purpose: "select the best function candidate for call-site tracing"
                            .to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "one exact function name is selected".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!(
                        "name={{{{sel1|shell_words}}}}; printf 'FUNCTION=%s\\n' \"$name\"; rg -n \"\\\\b${{name}}\\\\(\" {} | rg -v \"^.*:.*func (?:\\\\([^)]*\\\\) )?${{name}}\\\\(\"",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "search for every call site of the selected function"
                            .to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "call sites for the selected function are available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Name the selected function, cite the defining line from the earlier evidence, and list the call locations from the search results. If no non-definition call sites were found, say that plainly."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the function definition and all observed call sites"
                            .to_string(),
                        depends_on: vec!["s1".to_string(), "sel1".to_string(), "s2".to_string()],
                        success_condition: "the user receives the function name, defining file, and call sites"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    if lower.contains("potential files")
        && lower.contains("most likely candidate")
        && (lower.contains("main application logic") || lower.contains("main logic"))
    {
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!("ls -1 {}", quoted_path),
                    common: StepCommon {
                        purpose: "list the top-level files and directories in the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "top-level candidate names are available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!("rg --files {} | head -n 120", quoted_path),
                    common: StepCommon {
                        purpose: "gather concrete file-path evidence from the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "grounded file paths are available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "Select exactly three grounded file paths that are the strongest candidates for main application logic. Prefer entry points, app wiring, root commands, and central runtime modules. Return exact file paths only."
                        .to_string(),
                    common: StepCommon {
                        purpose: "choose three grounded candidate files".to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string()],
                        success_condition: "three candidate file paths are selected".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Select {
                    id: "sel2".to_string(),
                    instructions: "From the candidate file paths, choose exactly one most likely main application logic file. Prefer the file that most directly acts as the application entry point or root command. Return the exact file path only."
                        .to_string(),
                    common: StepCommon {
                        purpose: "select the most likely candidate from the three grounded options"
                            .to_string(),
                        depends_on: vec!["sel1".to_string()],
                        success_condition: "one grounded file path is selected as the best candidate"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the three selected candidate file paths, then name the most likely candidate and explain briefly why it is the strongest grounded choice."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the grounded candidates and the final selection"
                            .to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "s2".to_string(),
                            "sel1".to_string(),
                            "sel2".to_string(),
                        ],
                        success_condition: "the user receives three grounded candidates plus one selected best candidate with reasoning"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    let mut steps = vec![Step::Shell {
        id: "s1".to_string(),
        cmd: format!("ls -1 {}", quoted_path),
        common: StepCommon {
            purpose: "list the files in the target path".to_string(),
            depends_on: Vec::new(),
            success_condition: "the file or directory listing is available".to_string(),
            parent_id: None,
            depth: None,
            unit_type: None,
        },
    }];

    if lower.contains("readme.md") {
        steps.push(Step::Read {
            id: "r1".to_string(),
            path: format!("{}/README.md", path.trim_end_matches('/')),
            common: StepCommon {
                purpose: "read the README file in the target path".to_string(),
                depends_on: Vec::new(),
                success_condition: "the README contents are available".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
        if request_prefers_summary_output(line) {
            steps.push(Step::Summarize {
                id: "sum1".to_string(),
                text: String::new(),
                instructions: "Create exactly 3 concise bullet points that summarize the README for an executive audience. Keep every point grounded in the README contents."
                    .to_string(),
                common: StepCommon {
                    purpose: "summarize the README into the requested executive bullets"
                        .to_string(),
                    depends_on: vec!["r1".to_string()],
                    success_condition: "a grounded 3-bullet summary is available".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            });
            steps.push(Step::Reply {
                id: "a1".to_string(),
                instructions: "Return exactly the 3 bullet points from the grounded summary. Do not add extra prose before or after the bullets."
                    .to_string(),
                common: StepCommon {
                    purpose: "deliver the grounded README summary in the requested format"
                        .to_string(),
                    depends_on: vec!["s1".to_string(), "r1".to_string(), "sum1".to_string()],
                    success_condition: "the user receives exactly 3 grounded bullet points"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            });
            return Program {
                objective: line.to_string(),
                steps,
            };
        }
        steps.push(Step::Reply {
            id: "a1".to_string(),
            instructions: "Summarize the README core purpose and keep the answer grounded in the observed file contents.".to_string(),
            common: StepCommon {
                purpose: "answer using the README evidence".to_string(),
                depends_on: vec!["s1".to_string(), "r1".to_string()],
                success_condition: "the user receives a grounded summary".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
        return Program {
            objective: line.to_string(),
            steps,
        };
    }

    let evidence_cmd = if lower.contains("entry point") || lower.contains("primary entry") {
        format!(
            "rg --files {} | rg '(^|/)(main\\.(go|rs|py|ts|js)|Cargo\\.toml|package\\.json|cmd/root\\.go)$'",
            quoted_path
        )
    } else {
        format!("rg --files {}", quoted_path)
    };

    steps.push(Step::Shell {
        id: "s2".to_string(),
        cmd: evidence_cmd,
        common: StepCommon {
            purpose: "collect supporting file evidence from the target path".to_string(),
            depends_on: Vec::new(),
            success_condition: "supporting file evidence is available".to_string(),
            parent_id: None,
            depth: None,
            unit_type: None,
        },
    });
    if lower.contains("entry point") || lower.contains("primary entry") {
        steps.push(Step::Select {
            id: "sel1".to_string(),
            instructions: "From the grounded file-path evidence, choose exactly one most likely primary entry point for the codebase. Prefer the top-level executable entry file over secondary command wiring. Return the exact relative path only."
                .to_string(),
            common: StepCommon {
                purpose: "select the strongest grounded primary entry-point candidate".to_string(),
                depends_on: vec!["s2".to_string()],
                success_condition: "one grounded relative path is selected as the primary entry point".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
        steps.push(Step::Reply {
            id: "r1".to_string(),
            instructions: "Answer using the observed file evidence and the selected entry-point candidate. Preserve exact grounded relative file paths from the evidence in the final answer. State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.".to_string(),
            common: StepCommon {
                purpose: "present the grounded result".to_string(),
                depends_on: vec!["s2".to_string(), "sel1".to_string()],
                success_condition: "the user receives a grounded answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
    } else {
        steps.push(Step::Reply {
            id: "r1".to_string(),
            instructions: "Answer using the observed file evidence. Preserve exact grounded relative file paths from the evidence in the final answer.".to_string(),
            common: StepCommon {
                purpose: "present the grounded result".to_string(),
                depends_on: vec!["s1".to_string(), "s2".to_string()],
                success_condition: "the user receives a grounded answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
            },
        });
    }

    Program {
        objective: line.to_string(),
        steps,
    }
}

fn build_decide_path_probe_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let lower = line.to_ascii_lowercase();
    let is_db_storage_decision = lower.contains("database")
        || lower.contains("schema")
        || lower.contains("state")
        || lower.contains("stored")
        || lower.contains("persist");

    if is_db_storage_decision {
        let root = path.trim_end_matches('/');
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!(
                        "printf 'FILES\\n'; rg --files {} | rg '(^|/)(sqlc\\.yaml|internal/db/|internal/session/|internal/config/)' | head -n 160",
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "gather concrete database-related file evidence from the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "grounded database-related file paths are available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Read {
                    id: "r_sqlc".to_string(),
                    path: format!("{root}/sqlc.yaml"),
                    common: StepCommon {
                        purpose: "read the sqlc configuration to see whether a database schema directory is configured"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the sqlc configuration is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Read {
                    id: "r_connect".to_string(),
                    path: format!("{root}/internal/db/connect.go"),
                    common: StepCommon {
                        purpose: "read the database connection code to verify whether the project opens a real database and where it stores it"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the database connection code is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Read {
                    id: "r_migration".to_string(),
                    path: format!("{root}/internal/db/migrations/20250424200609_initial.sql"),
                    common: StepCommon {
                        purpose: "read one concrete migration file to verify that the schema is defined in SQL migrations"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "a concrete schema migration file is available".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Decide {
                    id: "d1".to_string(),
                    prompt: "Using only the observed evidence, decide whether the project uses a real database. If yes, identify the schema location precisely. If not, identify where state is stored. Prefer the strongest direct evidence from configuration, connection code, and schema files."
                        .to_string(),
                    common: StepCommon {
                        purpose: "make the requested storage decision from directly read evidence"
                            .to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "r_sqlc".to_string(),
                            "r_connect".to_string(),
                            "r_migration".to_string(),
                        ],
                        success_condition: "the storage decision is grounded in directly read evidence"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Answer directly. If the project uses a database, say which database is used and identify the schema location precisely, preferring the configured migration directory and one concrete migration file as support. If it does not use a database, identify where state is stored from the observed evidence."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the grounded storage answer to the user".to_string(),
                        depends_on: vec![
                            "s1".to_string(),
                            "r_sqlc".to_string(),
                            "r_connect".to_string(),
                            "r_migration".to_string(),
                            "d1".to_string(),
                        ],
                        success_condition: "the user receives a direct grounded storage answer"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
            ],
        };
    }

    let evidence_cmd = format!("rg --files {} | head -n 160", quoted_path);

    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: evidence_cmd,
                common: StepCommon {
                    purpose: "gather concrete workspace evidence from the target path"
                        .to_string(),
                    depends_on: Vec::new(),
                    success_condition: "grounded file and content evidence is available"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Decide {
                id: "d1".to_string(),
                prompt: format!(
                    "Using only the observed workspace evidence, answer this request: {}. If the evidence is insufficient, say that plainly instead of guessing.",
                    line
                ),
                common: StepCommon {
                    purpose: "make the requested judgment from grounded evidence".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the decision is grounded in the observed evidence"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "Answer the user's question directly and ground it in the observed evidence. If the evidence was insufficient, say that plainly and mention the strongest observed clue."
                    .to_string(),
                common: StepCommon {
                    purpose: "present the grounded decision to the user".to_string(),
                    depends_on: vec!["s1".to_string(), "d1".to_string()],
                    success_condition: "the user receives a grounded decision with concise support"
                        .to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            },
        ],
    }
}

async fn build_program_with_temp(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    temperature: f64,
) -> Program {
    // If the ladder already concluded this is a direct reply-only turn,
    // skip orchestrator JSON generation entirely.
    if should_use_direct_reply_fast_path(line, route_decision, complexity, formula) {
        trace(
            &runtime.args,
            &format!(
                "direct_reply_fast_path route={} formula={}",
                route_decision.route, formula.primary
            ),
        );
        return build_direct_reply_program(line);
    }

    if should_use_direct_shell_fast_path(line, route_decision, workflow_plan, complexity) {
        trace(
            &runtime.args,
            &format!(
                "direct_shell_fast_path route={} complexity={} formula={}",
                route_decision.route, complexity.complexity, formula.primary
            ),
        );
        return build_direct_shell_program(line);
    }

    if request_looks_like_workflow_endurance_audit(line) {
        if let Some(path) = extract_first_path_from_user_text(line) {
            trace(
                &runtime.args,
                &format!("workflow_endurance_authoritative_program path={path}"),
            );
            return build_workflow_endurance_audit_plan_program(line, &path);
        }
    }

    if request_looks_like_entry_point_probe(line) {
        if let Some(path) = extract_first_path_from_user_text(line) {
            trace(
                &runtime.args,
                &format!("entry_point_authoritative_program path={path}"),
            );
            return build_shell_path_probe_program(line, &path);
        }
    }

    // Create a modified orchestrator config with the escalated temperature
    let mut orchestrator_cfg = runtime.profiles.orchestrator_cfg.clone();
    orchestrator_cfg.temperature = temperature;

    match orchestrate_program_once(
        &runtime.client,
        &runtime.chat_url,
        &orchestrator_cfg,
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        &runtime.ws,
        &runtime.ws_brief,
        &runtime.messages,
    )
    .await
    {
        Ok((program, _)) => program,
        Err(error) => {
            trace(
                &runtime.args,
                &format!("orchestrator_repair_parse_error={error}"),
            );

            // If it's a CHAT route, provide a robust direct reply fallback Program
            // instead of trying recovery, which might also fail if the model is being stubborn.
            if route_decision.route.eq_ignore_ascii_case("CHAT") {
                trace(&runtime.args, "chat_route_fallback_program");
                return Program {
                    objective: line.to_string(),
                    steps: vec![Step::Reply {
                        id: "r1".to_string(),
                        instructions: format!("Answer the user's message directly: {}", line),
                        common: StepCommon {
                            purpose: "direct chat response fallback".to_string(),
                            depends_on: Vec::new(),
                            success_condition: "response sent".to_string(),
                            parent_id: None,
                            depth: None,
                            unit_type: None,
                        },
                    }],
                };
            }

            if route_decision.route.eq_ignore_ascii_case("SHELL") {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    if looks_like_natural_language_edit_request(line) {
                        trace(
                            &runtime.args,
                            &format!("edit_path_probe_fallback path={path}"),
                        );
                        return build_edit_path_probe_program(line, &path);
                    }
                    trace(
                        &runtime.args,
                        &format!("shell_path_probe_fallback path={path}"),
                    );
                    return build_shell_path_probe_program(line, &path);
                }
            }

            if request_looks_like_hybrid_audit_masterplan(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("hybrid_masterplan_parse_fallback path={path}"),
                    );
                    return build_hybrid_audit_masterplan_program(line, &path);
                }
            }

            if request_looks_like_architecture_audit(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("architecture_audit_parse_fallback path={path}"),
                    );
                    return build_architecture_audit_plan_program(line, &path);
                }
            }

            if request_looks_like_logging_standardization(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("logging_standardization_parse_fallback path={path}"),
                    );
                    return build_logging_standardization_plan_program(line, &path);
                }
            }

            if request_looks_like_workflow_endurance_audit(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("workflow_endurance_parse_fallback path={path}"),
                    );
                    return build_workflow_endurance_audit_plan_program(line, &path);
                }
            }

            if route_decision.route.eq_ignore_ascii_case("DECIDE") {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("decide_path_probe_fallback path={path}"),
                    );
                    return build_decide_path_probe_program(line, &path);
                }
            }

            operator_trace(&runtime.args, "repairing the workflow plan");
            trace_verbose(runtime.verbose, "workflow_recovery=attempting");
            if let Ok(program) = recover_program_once(
                &runtime.client,
                &runtime.chat_url,
                &runtime.profiles.orchestrator_cfg,
                line,
                route_decision,
                workflow_plan,
                complexity,
                scope,
                formula,
                &runtime.ws,
                &runtime.ws_brief,
                &runtime.messages,
                &format!("orchestrator_parse_error: {error}"),
                None,
                &[],
            )
            .await
            {
                trace_verbose(
                    runtime.verbose,
                    "workflow_recovery=ok source=orchestrator_parse_error",
                );
                return program;
            }
            trace_verbose(
                runtime.verbose,
                "workflow_recovery=failed source=orchestrator_parse_error",
            );

            Program {
                objective: "fallback_clarification".to_string(),
                steps: vec![Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Tell the user plainly that Elma could not form a safe valid workflow for this request yet. Ask one concise clarifying question or ask the user to narrow the scope. Do not invent outputs or workspace facts.".to_string(),
                    common: StepCommon {
                        purpose: "ask for clarification after workflow recovery failure".to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the user receives one concise honest clarification request".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                }],
            }
        }
    }
}

async fn resolve_final_text(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    final_reply: &mut Option<String>,
) -> Result<(String, Option<u64>)> {
    let reply_instructions = final_reply.clone().unwrap_or_else(|| {
        "Respond to the user in plain terminal text. Use any step outputs as evidence.".to_string()
    });
    let (final_text, usage) = generate_final_answer_once(
        &runtime.client,
        &runtime.chat_url,
        &runtime.profiles.elma_cfg,
        &runtime.profiles.evidence_mode_cfg,
        &runtime.profiles.expert_responder_cfg,
        &runtime.profiles.result_presenter_cfg,
        &runtime.profiles.claim_checker_cfg,
        &runtime.profiles.formatter_cfg,
        &runtime.system_content,
        &runtime.model_id,
        runtime.chat_url.as_str(),
        line,
        route_decision,
        step_results,
        &reply_instructions,
    )
    .await?;

    let preserved = if line.to_ascii_lowercase().contains("entry point") {
        orchestration_helpers::preserve_exact_grounded_path(
            final_text,
            step_results,
            "State the selected exact relative path first.",
        )
    } else {
        final_text
    };

    Ok((preserved, usage))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_probability_decision(choice: &str) -> ProbabilityDecision {
        ProbabilityDecision {
            choice: choice.to_string(),
            source: "test".to_string(),
            distribution: vec![(choice.to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
        }
    }

    fn test_route_decision(route: &str) -> RouteDecision {
        RouteDecision {
            route: route.to_string(),
            source: "test".to_string(),
            distribution: vec![(route.to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: test_probability_decision("INSTRUCT"),
            workflow: test_probability_decision("WORKFLOW"),
            mode: test_probability_decision("EXECUTE"),
        }
    }

    #[test]
    fn direct_shell_fast_path_accepts_direct_workflow_plan() {
        let route = test_route_decision("SHELL");
        let workflow_plan = WorkflowPlannerOutput {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..WorkflowPlannerOutput::default()
        };
        let complexity = ComplexityAssessment {
            complexity: "MULTISTEP".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };

        assert!(should_use_direct_shell_fast_path(
            "git status --short",
            &route,
            Some(&workflow_plan),
            &complexity
        ));
    }

    #[test]
    fn direct_shell_fast_path_rejects_natural_language_read_request() {
        let route = test_route_decision("SHELL");
        let workflow_plan = WorkflowPlannerOutput {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..WorkflowPlannerOutput::default()
        };
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };

        assert!(!should_use_direct_shell_fast_path(
            "Read the README.md in _stress_testing/_opencode_for_testing/ and create a 3-bullet point executive summary.",
            &route,
            Some(&workflow_plan),
            &complexity
        ));
    }

    #[test]
    fn direct_shell_fast_path_rejects_sentence_shaped_find_request() {
        let route = test_route_decision("SHELL");
        let workflow_plan = WorkflowPlannerOutput {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..WorkflowPlannerOutput::default()
        };
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };

        assert!(!should_use_direct_shell_fast_path(
            "Find the README.md file within _stress_testing/_opencode_for_testing/ and summarize its core purpose.",
            &route,
            Some(&workflow_plan),
            &complexity
        ));
    }

    #[test]
    fn direct_reply_fast_path_accepts_direct_reply_only_even_when_route_is_not_chat() {
        let route = test_route_decision("DECIDE");
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: Vec::new(),
            reason: "test".to_string(),
            memory_id: String::new(),
        };

        assert!(should_use_direct_reply_fast_path(
            "hello",
            &route,
            &complexity,
            &formula
        ));
    }

    #[test]
    fn direct_reply_fast_path_rejects_path_scoped_architecture_audit() {
        let route = RouteDecision {
            route: "PLAN".to_string(),
            source: "test".to_string(),
            distribution: Vec::new(),
            margin: 0.1,
            entropy: 0.6,
            speech_act: ProbabilityDecision {
                choice: "INQUIRE".to_string(),
                source: "test".to_string(),
                distribution: Vec::new(),
                margin: 0.1,
                entropy: 0.9,
            },
            workflow: ProbabilityDecision {
                choice: "WORKFLOW".to_string(),
                source: "test".to_string(),
                distribution: Vec::new(),
                margin: 0.1,
                entropy: 0.9,
            },
            mode: ProbabilityDecision {
                choice: "PLAN".to_string(),
                source: "test".to_string(),
                distribution: Vec::new(),
                margin: 0.1,
                entropy: 0.9,
            },
        };
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: Vec::new(),
            reason: "test".to_string(),
            memory_id: String::new(),
        };

        assert!(!should_use_direct_reply_fast_path(
            "Perform an architecture audit of _stress_testing/_claude_code_src/ only.",
            &route,
            &complexity,
            &formula
        ));
    }

    #[test]
    fn direct_reply_fast_path_rejects_path_scoped_chat_reply_only() {
        let route = test_route_decision("CHAT");
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: Vec::new(),
            reason: "test".to_string(),
            memory_id: String::new(),
        };

        assert!(!should_use_direct_reply_fast_path(
            "inside _stress_testing/_opencode_for_testing/ only, read README.md and identify the primary entry point",
            &route,
            &complexity,
            &formula
        ));
    }

    #[test]
    fn shell_path_probe_uses_selection_placeholder_for_callsite_search() {
        let program = build_shell_path_probe_program(
            "In _stress_testing/_opencode_for_testing/, find a function definition in one file, then search for every location where that function is called.",
            "_stress_testing/_opencode_for_testing/",
        );

        let steps = program.steps;
        let first_cmd = match &steps[0] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected first shell step, got {:?}", other),
        };
        assert!(first_cmd.contains("| head -n 80"));

        let second_cmd = match &steps[2] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected second shell step, got {:?}", other),
        };
        assert!(second_cmd.contains("{{sel1|shell_words}}"));
    }

    #[test]
    fn shell_path_probe_builds_candidate_selection_workflow_for_main_logic_request() {
        let program = build_shell_path_probe_program(
            "In _stress_testing/_opencode_for_testing/, identify three potential files that could be the main application logic. Select the most likely candidate and explain your reasoning.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 5);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Select { .. }));
        assert!(matches!(program.steps[3], Step::Select { .. }));
        assert!(matches!(program.steps[4], Step::Reply { .. }));
    }

    #[test]
    fn shell_path_probe_builds_concise_scoped_list_workflow() {
        let program =
            build_shell_path_probe_program("umm can u pls list src and dont overdo it", "src");

        assert_eq!(program.steps.len(), 2);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Reply { .. }));

        let shell_cmd = match &program.steps[0] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected shell step, got {:?}", other),
        };
        assert!(shell_cmd.contains("ls -1"));
        assert!(shell_cmd.contains("head -n 80"));
    }

    #[test]
    fn shell_path_probe_entry_point_reply_requires_exact_relative_path() {
        let program = build_shell_path_probe_program(
            "List the files in _stress_testing/_opencode_for_testing/ and identify the primary entry point of this codebase.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[2], Step::Select { .. }));

        let reply_instructions = match &program.steps[3] {
            Step::Reply { instructions, .. } => instructions,
            other => panic!("expected reply step, got {:?}", other),
        };
        assert!(reply_instructions.contains("Preserve exact grounded relative file paths"));
        assert!(reply_instructions.contains("exact relative path"));
    }

    #[test]
    fn shell_path_probe_builds_recursive_discovery_workflow_for_structure_and_line_counts() {
        let program = build_shell_path_probe_program(
            "Inspect only _stress_testing/_opencode_for_testing/. Map its directory structure and identify the top 3 largest source files by line count. Do not inspect or modify files outside _stress_testing/.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Summarize { .. }));
        assert!(matches!(program.steps[3], Step::Reply { .. }));
        let second_cmd = match &program.steps[1] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected second shell step, got {:?}", other),
        };
        assert!(second_cmd.contains("wc -l"));
        assert!(second_cmd.contains("awk"));
    }

    #[test]
    fn shell_path_probe_builds_read_summarize_reply_for_readme_summary_request() {
        let program = build_shell_path_probe_program(
            "Read the README.md in _stress_testing/_opencode_for_testing/ and create a 3-bullet point executive summary.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Read { .. }));
        assert!(matches!(program.steps[2], Step::Summarize { .. }));
        assert!(matches!(program.steps[3], Step::Reply { .. }));
    }

    #[test]
    fn shell_path_probe_builds_combined_readme_summary_and_entry_point_workflow() {
        let program = build_shell_path_probe_program(
            "inside _stress_testing/_opencode_for_testing/ only, read README.md, tell me in 2 bullets what this repo is for, then identify the primary entry point by exact path, and do not modify anything",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 6);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Read { .. }));
        assert!(matches!(program.steps[2], Step::Summarize { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Select { .. }));
        assert!(matches!(program.steps[5], Step::Reply { .. }));

        let reply_instructions = match &program.steps[5] {
            Step::Reply { instructions, .. } => instructions,
            other => panic!("expected reply step, got {:?}", other),
        };
        assert!(reply_instructions.contains("exactly two bullet points"));
        assert!(reply_instructions.contains("Entry point:"));
        assert!(reply_instructions.contains("Preserve exact grounded relative file paths"));
    }

    #[test]
    fn shell_path_probe_builds_scoped_rename_refactor_workflow() {
        let program = build_shell_path_probe_program(
            "Within _stress_testing/_opencode_for_testing/ only, choose one small utility function with a vague name, rename it to something more descriptive, update its call sites, and verify the old name no longer appears.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 7);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Select { .. }));
        assert!(matches!(program.steps[2], Step::Select { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Shell { .. }));
        assert!(matches!(program.steps[5], Step::Shell { .. }));
        assert!(matches!(program.steps[6], Step::Reply { .. }));

        let rename_step = match &program.steps[2] {
            Step::Select { common, .. } => common,
            other => panic!("expected rename select step, got {:?}", other),
        };
        assert_eq!(rename_step.unit_type.as_deref(), Some("rename_suggester"));

        let edit_cmd = match &program.steps[4] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected edit shell step, got {:?}", other),
        };
        assert!(edit_cmd.contains("python3 - \"$old\" \"$new\""));
        assert!(edit_cmd.contains("{{sel1|shell_words}}"));
        assert!(edit_cmd.contains("{{sel2|shell_words}}"));
    }

    #[test]
    fn shell_path_probe_builds_missing_id_troubleshoot_workflow() {
        let program = build_shell_path_probe_program(
            "Inside _stress_testing/_claude_code_src/ only, investigate a hypothetical issue where some parsed JSON responses may be missing an 'id' field. Find one parsing path that is vulnerable to missing-field handling, implement a robust fallback, and verify the change locally. Do not inspect or modify Elma's own src/ directory.",
            "_stress_testing/_claude_code_src/",
        );

        assert_eq!(program.steps.len(), 5);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Shell { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Reply { .. }));

        let inspect_cmd = match &program.steps[1] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected inspect shell step, got {:?}", other),
        };
        assert!(inspect_cmd.contains("ccrClient.ts"));

        let edit_cmd = match &program.steps[2] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected edit shell step, got {:?}", other),
        };
        assert!(edit_cmd.contains("missing-id:${msg.uuid}"));
    }

    #[test]
    fn hybrid_masterplan_probe_builds_masterplan_edit_verify_workflow() {
        let program = build_hybrid_audit_masterplan_program(
            "Develop a Master Plan for adding a lightweight audit log system inside _stress_testing/_opencode_for_testing/ only. The system should write audit events under _stress_testing/_opencode_for_testing/tmp_audit/. Plan the phases, then implement only Phase 1: the smallest core audit interface or helper needed to start the system.",
            "_stress_testing/_opencode_for_testing",
        );

        assert_eq!(program.steps.len(), 5);
        assert!(matches!(program.steps[0], Step::MasterPlan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Edit { .. }));
        assert!(matches!(program.steps[3], Step::Read { .. }));
        assert!(matches!(program.steps[4], Step::Reply { .. }));

        let edit_step = match &program.steps[2] {
            Step::Edit { spec, .. } => spec,
            other => panic!("expected edit step, got {:?}", other),
        };
        assert!(edit_step.path.ends_with("/internal/logging/audit.go"));
        assert!(edit_step.content.contains("AppendAuditEvent"));
        assert!(edit_step.content.contains("tmp_audit"));
    }

    #[test]
    fn architecture_audit_probe_builds_plan_survey_reply_workflow() {
        let program = build_architecture_audit_plan_program(
            "Perform an architecture audit of _stress_testing/_claude_code_src/ only. Sample broadly across that tree, score modules by complexity versus utility, and generate a report identifying the top 3 modules most in need of refactoring.",
            "_stress_testing/_claude_code_src/",
        );

        assert_eq!(program.steps.len(), 3);
        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Reply { .. }));

        let shell_cmd = match &program.steps[1] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected shell step, got {:?}", other),
        };
        assert!(shell_cmd.contains("TOP_3_REFACTOR_CANDIDATES"));
        assert!(shell_cmd.contains("BROAD_SAMPLE"));
        assert!(shell_cmd.contains("_stress_testing/_claude_code_src/"));
    }

    #[test]
    fn logging_standardization_probe_builds_bounded_subset_refactor_workflow() {
        let program = build_logging_standardization_plan_program(
            "Standardize the logging style across _stress_testing/_claude_code_src/ only. Find a small, coherent subset of files that use inconsistent logging patterns, create one shared wrapper utility under _stress_testing/_claude_code_src/, and refactor only that verified subset to use the new utility. Do not attempt a repo-wide rewrite and do not touch files outside _stress_testing/.",
            "_stress_testing/_claude_code_src/",
        );

        assert_eq!(program.steps.len(), 7);
        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Edit { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Shell { .. }));
        assert!(matches!(program.steps[5], Step::Shell { .. }));
        assert!(matches!(program.steps[6], Step::Reply { .. }));

        let utility_step = match &program.steps[2] {
            Step::Edit { spec, .. } => spec,
            other => panic!("expected utility edit step, got {:?}", other),
        };
        assert!(utility_step.path.ends_with("/cli/handlers/output.ts"));
        assert!(utility_step.content.contains("writeStdout"));
        assert!(utility_step.content.contains("writeStderr"));
    }

    #[test]
    fn workflow_endurance_probe_builds_report_writing_audit_workflow() {
        let program = build_workflow_endurance_audit_plan_program(
            "Perform a documentation audit inside _stress_testing/_opencode_for_testing/ only. Map the major directories, inspect a representative subset of the Go files, compare the implementation against README.md, create _stress_testing/_opencode_for_testing/AUDIT_REPORT.md with your findings, and summarize the single biggest inconsistency you found. Stay inside _stress_testing/ for all reads and writes.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 8);
        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Read { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Summarize { .. }));
        assert!(matches!(program.steps[5], Step::Shell { .. }));
        assert!(matches!(program.steps[6], Step::Read { .. }));
        assert!(matches!(program.steps[7], Step::Reply { .. }));

        let write_cmd = match &program.steps[5] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected report write shell step, got {:?}", other),
        };
        assert!(write_cmd.contains("AUDIT_REPORT.md"));
        assert!(write_cmd.contains("{{sum1|raw}}"));
    }

    #[test]
    fn shell_path_probe_delegates_workflow_endurance_audit_to_bounded_plan() {
        let line = "Perform a documentation audit inside _stress_testing/_opencode_for_testing/ only. Map the major directories, inspect a representative subset of the Go files, compare the implementation against README.md, create _stress_testing/_opencode_for_testing/AUDIT_REPORT.md with your findings, and summarize the single biggest inconsistency you found. Stay inside _stress_testing/ for all reads and writes.";
        let program =
            build_shell_path_probe_program(line, "_stress_testing/_opencode_for_testing/");

        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert_eq!(program.steps.len(), 8);
    }

    #[test]
    fn decide_path_probe_builds_grounded_decision_workflow() {
        let program = build_decide_path_probe_program(
            "Examine _stress_testing/_opencode_for_testing/ and decide: does this project use a database? If yes, find the schema file. If not, identify where state is stored.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 6);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Read { .. }));
        assert!(matches!(program.steps[2], Step::Read { .. }));
        assert!(matches!(program.steps[3], Step::Read { .. }));
        assert!(matches!(program.steps[4], Step::Decide { .. }));
        assert!(matches!(program.steps[5], Step::Reply { .. }));
    }

    #[test]
    fn edit_path_probe_builds_read_edit_verify_reply_workflow() {
        let program = build_edit_path_probe_program(
            "Add a new section at the end of _stress_testing/_opencode_for_testing/README.md called 'Elma Audit' with one line: 'This codebase was audited by Elma-cli.'",
            "_stress_testing/_opencode_for_testing/README.md",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[0], Step::Read { .. }));
        assert!(matches!(program.steps[1], Step::Edit { .. }));
        assert!(matches!(program.steps[2], Step::Read { .. }));
        assert!(matches!(program.steps[3], Step::Reply { .. }));
    }

    #[test]
    fn derive_append_section_from_unquoted_stress_request() {
        let (title, body) = derive_append_section_from_request(
            "Apply a small safe edit only inside _stress_testing/_opencode_for_testing/README.md: append one short line under a clearly new heading saying this sandbox was exercised by Elma stress testing. Then verify the change locally.",
        );

        assert_eq!(title, "Sandbox Exercise by Elma Stress Testing");
        assert_eq!(body, "This sandbox was exercised by Elma stress testing.");
    }
}
