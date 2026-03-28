use crate::app::AppRuntime;
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

        runtime.messages.push(ChatMessage {
            role: "user".to_string(),
            content: line.to_string(),
        });

        let route_decision = infer_route_prior(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.speech_act_cfg,
            &runtime.profiles.router_cfg,
            &runtime.profiles.mode_router_cfg,
            &runtime.profiles.router_cal,
            line,
            &runtime.ws,
            &runtime.ws_brief,
            &runtime.messages,
        )
        .await?;
        trace_route_decision(&runtime.args, &route_decision);

        let memories = load_recent_formula_memories(&runtime.model_cfg_dir, 8).unwrap_or_default();
        let (workflow_plan, complexity, scope, formula, planner_fallback_used) = derive_planning_prior(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.workflow_planner_cfg,
            &runtime.profiles.complexity_cfg,
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
                "planning_source={}",
                if planner_fallback_used {
                    "fallback_chain"
                } else if workflow_plan.is_some() {
                    "workflow_planner"
                } else {
                    "chat_fast_path"
                }
            ),
        );
        if let Some(plan) = workflow_plan.as_ref() {
            trace(
                &runtime.args,
                &format!(
                    "workflow_planner objective={} formula={} memory={} reason={}",
                    if plan.objective.trim().is_empty() { "-" } else { plan.objective.trim() },
                    if plan.preferred_formula.trim().is_empty() {
                        "-"
                    } else {
                        plan.preferred_formula.trim()
                    },
                    if plan.memory_id.trim().is_empty() { "-" } else { plan.memory_id.trim() },
                    plan.reason.trim()
                ),
            );
        }
        trace_complexity(&runtime.args, &complexity);
        trace_scope(&runtime.args, &scope);
        trace_formula(&runtime.args, &formula);
        operator_trace(
            &runtime.args,
            &describe_operator_intent(&route_decision, &complexity, &formula),
        );

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
        if apply_capability_guard(&mut program, &route_decision) {
            trace(&runtime.args, "guard=capability_reply_only");
        }

        let mut loop_outcome = run_autonomous_loop(
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
            &runtime.profiles.orchestrator_cfg,
            &runtime.profiles.planner_cfg,
            &runtime.profiles.planner_master_cfg,
            &runtime.profiles.decider_cfg,
            &runtime.profiles.selector_cfg,
            &runtime.profiles.summarizer_cfg,
            &runtime.profiles.command_repair_cfg,
            &runtime.profiles.command_preflight_cfg,
            &runtime.profiles.task_semantics_guard_cfg,
            &runtime.profiles.evidence_compactor_cfg,
            &runtime.profiles.artifact_classifier_cfg,
            &runtime.profiles.outcome_verifier_cfg,
            &runtime.profiles.execution_sufficiency_cfg,
            &runtime.profiles.critic_cfg,
        )
        .await?;
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

        print_final_output(&runtime.args, runtime.ctx_max, final_usage_total, &final_text);
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
    }

    Ok(())
}

fn trace_route_decision(args: &Args, route_decision: &RouteDecision) {
    trace(
        args,
        &format!(
            "speech_act_dist={}",
            format_route_distribution(&route_decision.speech_act.distribution)
        ),
    );
    trace(
        args,
        &format!(
            "speech_act={} p={:.2} margin={:.2} entropy={:.2} source={}",
            route_decision.speech_act.choice,
            route_decision
                .speech_act
                .distribution
                .first()
                .map(|(_, p)| *p)
                .unwrap_or(0.0),
            route_decision.speech_act.margin,
            route_decision.speech_act.entropy,
            route_decision.speech_act.source
        ),
    );
    trace(
        args,
        &format!(
            "workflow_dist={}",
            format_route_distribution(&route_decision.workflow.distribution)
        ),
    );
    trace(
        args,
        &format!(
            "workflow={} p={:.2} margin={:.2} entropy={:.2} source={}",
            route_decision.workflow.choice,
            route_decision
                .workflow
                .distribution
                .first()
                .map(|(_, p)| *p)
                .unwrap_or(0.0),
            route_decision.workflow.margin,
            route_decision.workflow.entropy,
            route_decision.workflow.source
        ),
    );
    trace(
        args,
        &format!(
            "mode_dist={}",
            format_route_distribution(&route_decision.mode.distribution)
        ),
    );
    trace(
        args,
        &format!(
            "mode={} p={:.2} margin={:.2} entropy={:.2} source={}",
            route_decision.mode.choice,
            route_decision
                .mode
                .distribution
                .first()
                .map(|(_, p)| *p)
                .unwrap_or(0.0),
            route_decision.mode.margin,
            route_decision.mode.entropy,
            route_decision.mode.source
        ),
    );
    trace(
        args,
        &format!(
            "route_dist={}",
            format_route_distribution(&route_decision.distribution)
        ),
    );
    let route_p = route_decision
        .distribution
        .first()
        .map(|(_, p)| *p)
        .unwrap_or(0.0);
    trace(
        args,
        &format!(
            "route={} p={route_p:.2} margin={:.2} entropy={:.2} source={}",
            route_decision.route,
            route_decision.margin,
            route_decision.entropy,
            route_decision.source
        ),
    );
}

fn trace_complexity(args: &Args, complexity: &ComplexityAssessment) {
    trace(
        args,
        &format!(
            "complexity={} pattern={} risk={}",
            if complexity.complexity.is_empty() {
                "UNKNOWN"
            } else {
                &complexity.complexity
            },
            if complexity.suggested_pattern.is_empty() {
                "unknown"
            } else {
                &complexity.suggested_pattern
            },
            if complexity.risk.is_empty() {
                "UNKNOWN"
            } else {
                &complexity.risk
            }
        ),
    );
}

fn trace_scope(args: &Args, scope: &ScopePlan) {
    let trivial_root_only = !scope.focus_paths.is_empty()
        && scope.focus_paths.iter().all(|path| {
            let path = path.trim();
            path.is_empty() || path == "." || path == "./"
        });
    if !scope.focus_paths.is_empty() && !trivial_root_only {
        operator_trace(
            args,
            &format!(
                "narrowing the scope{}",
                if scope.focus_paths.is_empty() {
                    String::new()
                } else {
                    format!(" to {}", scope.focus_paths.join(", "))
                }
            ),
        );
    }
    trace(
        args,
        &format!(
            "scope focus={} include={} exclude={} query={} reason={}",
            if scope.focus_paths.is_empty() {
                "-".to_string()
            } else {
                scope.focus_paths.join(",")
            },
            if scope.include_globs.is_empty() {
                "-".to_string()
            } else {
                scope.include_globs.join(",")
            },
            if scope.exclude_globs.is_empty() {
                "-".to_string()
            } else {
                scope.exclude_globs.join(",")
            },
            if scope.query_terms.is_empty() {
                "-".to_string()
            } else {
                scope.query_terms.join(",")
            },
            scope.reason
        ),
    );
}

fn trace_formula(args: &Args, formula: &FormulaSelection) {
    trace(
        args,
        &format!(
            "formula={} alt={} reason={}",
            if formula.primary.is_empty() {
                "unknown"
            } else {
                &formula.primary
            },
            if formula.alternatives.is_empty() {
                "-".to_string()
            } else {
                formula.alternatives.join(",")
            },
            if formula.memory_id.trim().is_empty() {
                formula.reason.clone()
            } else {
                format!("{} memory={}", formula.reason, formula.memory_id)
            }
        ),
    );
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
    match orchestrate_program_once(
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
    )
    .await
    {
        Ok((program, _)) => program,
        Err(error) => {
            trace(
                &runtime.args,
                &format!("orchestrator_repair_parse_error={error}"),
            );
            if !route_decision.route.eq_ignore_ascii_case("CHAT") {
                operator_trace(&runtime.args, "repairing the workflow plan");
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
                    trace(&runtime.args, "workflow_recovery=ok source=orchestrator_parse_error");
                    return program;
                }
                trace(
                    &runtime.args,
                    "workflow_recovery=failed source=orchestrator_parse_error",
                );
            }
            Program {
                objective: "fallback_clarification".to_string(),
                steps: vec![Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Tell the user plainly that Elma could not form a safe valid workflow for this request yet. Ask one concise clarifying question or ask the user to narrow the scope. Do not invent outputs or workspace facts.".to_string(),
                    common: StepCommon {
                        purpose: "ask for clarification after workflow recovery failure".to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the user receives one concise honest clarification request".to_string(),
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
        "Respond to the user in plain terminal text. Use any step outputs as evidence."
            .to_string()
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

fn print_final_output(
    args: &Args,
    ctx_max: Option<u64>,
    final_usage_total: Option<u64>,
    final_text: &str,
) {
    println!(
        "{}",
        if args.no_color {
            format!("Elma: {final_text}")
        } else {
            ansi_orange(&format!("Elma: {final_text}"))
        }
    );

    if let Some(ctx) = ctx_max {
        if let Some(total) = final_usage_total {
            let pct = (total as f64 / ctx as f64) * 100.0;
            let used_k = {
                let k = ((total as f64) / 1000.0).round() as u64;
                if total > 0 {
                    k.max(1)
                } else {
                    0
                }
            };
            let ctx_k = ((ctx as f64) / 1000.0).round() as u64;
            let line = format!("ctx: {used_k}k/{ctx_k}k [{pct:.1}%]");
            println!(
                "{}",
                if args.no_color {
                    line
                } else {
                    ansi_grey(&line)
                }
            );
        }
    }
    println!();
}

async fn maybe_save_formula_memory(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    memory_gate_cfg: &Profile,
    model_id: &str,
    model_cfg_dir: &PathBuf,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    scope: &ScopePlan,
    program: &Program,
    step_results: &[StepResult],
    reasoning_clean: bool,
) -> Result<()> {
    if !formula.memory_id.trim().is_empty() {
        let reuse_success = reasoning_clean && step_results.iter().all(|result| result.ok);
        let artifact_mode_capable = step_results
            .iter()
            .any(|result| result.artifact_path.is_some());
        if let Ok(Some(record)) = record_formula_memory_reuse(
            model_cfg_dir,
            formula.memory_id.trim(),
            reuse_success,
            artifact_mode_capable,
        ) {
            trace(
                args,
                &format!(
                    "formula_memory_reuse id={} status={} success_count={} failure_count={} disabled={}",
                    record.id,
                    if reuse_success { "success" } else { "failure" },
                    record.success_count,
                    record.failure_count,
                    record.disabled
                ),
            );
        }
        return Ok(());
    }

    if !reasoning_clean {
        trace(args, "memory_gate_status=skip reason=unclean_reasoning_fallback");
        return Ok(());
    }
    if step_results.iter().all(|result| result.ok)
        && !route_decision.route.eq_ignore_ascii_case("CHAT")
    {
        let gate = gate_formula_memory_once(
            client,
            chat_url,
            memory_gate_cfg,
            line,
            route_decision,
            complexity,
            formula,
            scope,
            program,
            step_results,
        )
        .await
        .unwrap_or_else(|_| MemoryGateVerdict {
            status: "skip".to_string(),
            reason: "memory_gate_error".to_string(),
        });
        trace(
            args,
            &format!("memory_gate_status={} reason={}", gate.status, gate.reason),
        );
        if !gate.status.eq_ignore_ascii_case("save") {
            return Ok(());
        }
        let now = now_unix_s()?;
        let active_run_id = load_active_manifest(&model_active_manifest_path(model_cfg_dir))
            .ok()
            .and_then(|m| m.active_run_id)
            .unwrap_or_default();
        let record = FormulaMemoryRecord {
            id: format!("fm_{now}"),
            created_unix_s: now,
            model_id: model_id.to_string(),
            active_run_id,
            user_message: line.to_string(),
            route: route_decision.route.clone(),
            complexity: complexity.complexity.clone(),
            formula: if formula.primary.trim().is_empty() {
                complexity.suggested_pattern.clone()
            } else {
                formula.primary.clone()
            },
            objective: program.objective.clone(),
            title: if !scope.objective.trim().is_empty() {
                scope.objective.clone()
            } else {
                line.to_string()
            },
            program_signature: program_signature(program),
            last_success_unix_s: now,
            last_failure_unix_s: 0,
            success_count: 1,
            failure_count: 0,
            disabled: false,
            artifact_mode_capable: step_results
                .iter()
                .any(|result| result.artifact_path.is_some()),
        };
        if let Ok(path) = save_formula_memory(model_cfg_dir, &record) {
            trace(args, &format!("formula_memory_saved={}", path.display()));
        }
    }
    Ok(())
}
