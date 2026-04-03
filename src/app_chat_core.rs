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
        show_process_step_verbose(
            runtime.verbose,
            "PLAN",
            &format!("{} → {} steps", complexity.complexity, program.steps.len()),
        );
        let guards_enabled = !runtime.args.disable_guards;
        if apply_capability_guard(&mut program, &route_decision, guards_enabled) {
            trace_verbose(runtime.verbose, "guard=capability_reply_only");
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

    crate::tool_discovery::command_exists(head)
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

fn build_shell_path_probe_program(line: &str, path: &str) -> Program {
    let quoted_path = shell_quote(path);
    let lower = line.to_ascii_lowercase();
    if lower.contains("function definition") && lower.contains("called") {
        return Program {
            objective: line.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: format!("rg -n -m 1 '^func ' {}", quoted_path),
                    common: StepCommon {
                        purpose: "find one concrete function definition in the target path"
                            .to_string(),
                        depends_on: Vec::new(),
                        success_condition: "a concrete function definition is available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: format!(
                        "name=$(python3 - <<'PY'\nimport pathlib, re\nroot = pathlib.Path({})\nfor file_path in root.rglob('*.go'):\n    try:\n        with file_path.open() as handle:\n            for line in handle:\n                match = re.match(r'^func (?:\\([^)]*\\) )?([A-Za-z_][A-Za-z0-9_]*)\\(', line)\n                if match:\n                    print(match.group(1))\n                    raise SystemExit\n    except OSError:\n        pass\nPY\n); printf 'FUNCTION=%s\\n' \"$name\"; rg -n \"\\\\b${{name}}\\\\(\" {}",
                        quoted_path,
                        quoted_path
                    ),
                    common: StepCommon {
                        purpose: "search for every call site of the discovered function"
                            .to_string(),
                        depends_on: vec!["s1".to_string()],
                        success_condition: "call sites for the discovered function are available"
                            .to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Name the discovered function, cite the file where it is defined from the earlier evidence, and list the call locations from the search results."
                        .to_string(),
                    common: StepCommon {
                        purpose: "present the function definition and all observed call sites"
                            .to_string(),
                        depends_on: vec!["s1".to_string(), "s2".to_string()],
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
    steps.push(Step::Reply {
        id: "r1".to_string(),
        instructions: "Answer using the observed file evidence. If the request asks for the primary entry point, identify the strongest candidate from the listed files and explain briefly.".to_string(),
        common: StepCommon {
            purpose: "present the grounded result".to_string(),
            depends_on: vec!["s1".to_string(), "s2".to_string()],
            success_condition: "the user receives a grounded answer".to_string(),
            parent_id: None,
            depth: None,
            unit_type: None,
        },
    });

    Program {
        objective: line.to_string(),
        steps,
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
    // For CHAT routes with reply_only formula, skip orchestrator entirely
    // No need to request JSON for simple conversational replies
    if route_decision.route.eq_ignore_ascii_case("CHAT")
        && formula.primary.eq_ignore_ascii_case("reply_only")
    {
        trace(
            &runtime.args,
            &format!(
                "chat_reply_only_fast_path route={} formula={}",
                route_decision.route, formula.primary
            ),
        );
        return Program {
            objective: line.to_string(),
            steps: vec![Step::Reply {
                id: "r1".to_string(),
                instructions: format!("Respond naturally to: {}", line),
                common: StepCommon {
                    purpose: "conversational reply".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "user receives a natural, helpful response".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            }],
        };
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
                    trace(&runtime.args, &format!("shell_path_probe_fallback path={path}"));
                    return build_shell_path_probe_program(line, &path);
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
    generate_final_answer_once(
        &runtime.client,
        &runtime.chat_url,
        &runtime.profiles.elma_cfg,
        &runtime.profiles.evidence_mode_cfg,
        &runtime.profiles.result_presenter_cfg,
        &runtime.profiles.claim_checker_cfg,
        &runtime.profiles.formatter_cfg,
        &runtime.system_content,
        line,
        route_decision,
        step_results,
        &reply_instructions,
    )
    .await
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
}
