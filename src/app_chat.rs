use crate::app::AppRuntime;
use crate::*;

pub(crate) async fn run_chat_loop(runtime: &mut AppRuntime) -> Result<()> {
    loop {
        let Some(line) = prompt_line("you> ")? else {
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

        let complexity = assess_complexity_once(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.complexity_cfg,
            line,
            &route_decision,
            &runtime.ws,
            &runtime.ws_brief,
            &runtime.messages,
        )
        .await
        .unwrap_or_default();
        trace_complexity(&runtime.args, &complexity);

        let scope = build_scope_once(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.scope_builder_cfg,
            line,
            &route_decision,
            &complexity,
            &runtime.ws,
            &runtime.ws_brief,
            &runtime.messages,
        )
        .await
        .unwrap_or_default();
        trace_scope(&runtime.args, &scope);

        let memories = load_recent_formula_memories(&runtime.model_cfg_dir, 8).unwrap_or_default();
        let formula = select_formula_once(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.formula_cfg,
            line,
            &route_decision,
            &complexity,
            &scope,
            &memories,
            &runtime.messages,
        )
        .await
        .unwrap_or_default();
        trace_formula(&runtime.args, &formula);
        operator_trace(
            &runtime.args,
            &describe_operator_intent(&route_decision, &complexity, &formula),
        );

        let mut program = build_program(
            runtime,
            line,
            &route_decision,
            &complexity,
            &scope,
            &formula,
        )
        .await;
        if apply_capability_guard(&mut program, &route_decision) {
            trace(&runtime.args, "guard=capability_reply_only");
        }

        let (mut step_results, mut final_reply) = execute_program(
            &runtime.args,
            &runtime.client,
            &runtime.chat_url,
            &runtime.session,
            &runtime.repo,
            &program,
            &runtime.profiles.planner_cfg,
            &runtime.profiles.planner_master_cfg,
            &runtime.profiles.decider_cfg,
            &runtime.profiles.summarizer_cfg,
            Some(&runtime.profiles.command_repair_cfg),
            Some(&runtime.profiles.evidence_compactor_cfg),
            Some(&runtime.profiles.artifact_classifier_cfg),
            &scope,
            &complexity,
            &formula,
            &program.objective,
            false,
            false,
        )
        .await?;

        let (final_text, final_usage_total) = resolve_final_text(
            runtime,
            line,
            &route_decision,
            &complexity,
            &formula,
            &scope,
            &mut program,
            &mut step_results,
            &mut final_reply,
        )
        .await?;

        print_final_output(&runtime.args, runtime.ctx_max, final_usage_total, &final_text);
        maybe_save_formula_memory(
            &runtime.args,
            &runtime.model_cfg_dir,
            line,
            &route_decision,
            &complexity,
            &formula,
            &scope,
            &program,
            &step_results,
        )?;
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
            Program {
                objective: "fallback_chat".to_string(),
                steps: vec![Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Reply to the user in plain terminal text. Do not invent workspace facts you did not inspect.".to_string(),
                    common: StepCommon::default(),
                }],
            }
        }
    }
}

async fn resolve_final_text(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    scope: &ScopePlan,
    program: &mut Program,
    step_results: &mut Vec<StepResult>,
    final_reply: &mut Option<String>,
) -> Result<(String, Option<u64>)> {
    for attempt in 0..=1u32 {
        let verdict = match run_critic_once(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.critic_cfg,
            line,
            route_decision,
            program,
            step_results,
            attempt,
        )
        .await
        {
            Ok(verdict) => verdict,
            Err(error) => {
                trace(&runtime.args, &format!("critic_parse_error={error}"));
                CriticVerdict {
                    status: "ok".to_string(),
                    reason: "critic_parse_error".to_string(),
                    program: None,
                }
            }
        };
        trace(
            &runtime.args,
            &format!("critic_status={} reason={}", verdict.status, verdict.reason),
        );

        if verdict.status.eq_ignore_ascii_case("retry") {
            if let Some(retry_program) = verdict.program {
                *program = retry_program;
                if apply_capability_guard(program, route_decision) {
                    trace(&runtime.args, "guard=capability_reply_only_retry");
                }
                let (retry_results, retry_reply) = execute_program(
                    &runtime.args,
                    &runtime.client,
                    &runtime.chat_url,
                    &runtime.session,
                    &runtime.repo,
                    program,
                    &runtime.profiles.planner_cfg,
                    &runtime.profiles.planner_master_cfg,
                    &runtime.profiles.decider_cfg,
                    &runtime.profiles.summarizer_cfg,
                    Some(&runtime.profiles.command_repair_cfg),
                    Some(&runtime.profiles.evidence_compactor_cfg),
                    Some(&runtime.profiles.artifact_classifier_cfg),
                    scope,
                    complexity,
                    formula,
                    &program.objective,
                    false,
                    false,
                )
                .await?;
                step_results.extend(retry_results);
                if retry_reply.is_some() {
                    *final_reply = retry_reply;
                }
                continue;
            }
        }

        let reply_instructions = final_reply.clone().unwrap_or_else(|| {
            "Respond to the user in plain terminal text. Use any step outputs as evidence."
                .to_string()
        });
        let (final_text, usage_total) = match generate_final_answer_once(
            &runtime.client,
            &runtime.chat_url,
            &runtime.profiles.elma_cfg,
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
        {
            Ok(value) => value,
            Err(error) => {
                trace(&runtime.args, &format!("reply_generation_error={error}"));
                (
                    "I ran into a reply-generation error after executing the workflow."
                        .to_string(),
                    None,
                )
            }
        };
        return Ok((final_text, usage_total));
    }

    Ok((String::new(), None))
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

fn maybe_save_formula_memory(
    args: &Args,
    model_cfg_dir: &PathBuf,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    scope: &ScopePlan,
    program: &Program,
    step_results: &[StepResult],
) -> Result<()> {
    if step_results.iter().all(|result| result.ok)
        && !route_decision.route.eq_ignore_ascii_case("CHAT")
        && formula.memory_id.trim().is_empty()
    {
        let now = now_unix_s()?;
        let record = FormulaMemoryRecord {
            id: format!("fm_{now}"),
            created_unix_s: now,
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
        };
        if let Ok(path) = save_formula_memory(model_cfg_dir, &record) {
            trace(args, &format!("formula_memory_saved={}", path.display()));
        }
    }
    Ok(())
}
