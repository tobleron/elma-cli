use crate::*;

fn fallback_formula_for_route(route: &str, needs_evidence: bool) -> String {
    if route.eq_ignore_ascii_case("CHAT") {
        "reply_only".to_string()
    } else if route.eq_ignore_ascii_case("PLAN") {
        "plan_reply".to_string()
    } else if route.eq_ignore_ascii_case("MASTERPLAN") {
        "masterplan_reply".to_string()
    } else if route.eq_ignore_ascii_case("DECIDE") {
        if needs_evidence {
            "inspect_decide_reply".to_string()
        } else {
            "reply_only".to_string()
        }
    } else if needs_evidence {
        "inspect_reply".to_string()
    } else {
        "execute_reply".to_string()
    }
}

fn planning_prior_from_workflow_plan(
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: &WorkflowPlannerOutput,
) -> (ComplexityAssessment, ScopePlan, FormulaSelection) {
    let complexity = ComplexityAssessment {
        complexity: if workflow_plan.complexity.trim().is_empty() {
            if route_decision.route.eq_ignore_ascii_case("CHAT") {
                "DIRECT".to_string()
            } else {
                "INVESTIGATE".to_string()
            }
        } else {
            workflow_plan.complexity.trim().to_string()
        },
        needs_evidence: workflow_plan.needs_evidence,
        needs_tools: !route_decision.route.eq_ignore_ascii_case("CHAT"),
        needs_decision: workflow_plan
            .preferred_formula
            .to_lowercase()
            .contains("decide"),
        needs_plan: route_decision.route.eq_ignore_ascii_case("PLAN")
            || route_decision.route.eq_ignore_ascii_case("MASTERPLAN"),
        risk: if workflow_plan.risk.trim().is_empty() {
            "LOW".to_string()
        } else {
            workflow_plan.risk.trim().to_string()
        },
        suggested_pattern: if workflow_plan.preferred_formula.trim().is_empty() {
            fallback_formula_for_route(&route_decision.route, workflow_plan.needs_evidence)
        } else {
            workflow_plan.preferred_formula.trim().to_string()
        },
    };

    let mut scope = workflow_plan.scope.clone();
    if scope.objective.trim().is_empty() {
        scope.objective = if workflow_plan.objective.trim().is_empty() {
            line.to_string()
        } else {
            workflow_plan.objective.trim().to_string()
        };
    }

    let formula = FormulaSelection {
        primary: complexity.suggested_pattern.clone(),
        alternatives: workflow_plan.alternatives.clone(),
        reason: workflow_plan.reason.clone(),
        memory_id: workflow_plan.memory_id.clone(),
    };

    (complexity, scope, formula)
}

pub(crate) async fn derive_planning_prior(
    client: &reqwest::Client,
    chat_url: &Url,
    workflow_planner_cfg: &Profile,
    complexity_cfg: &Profile,
    scope_builder_cfg: &Profile,
    formula_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> (Option<WorkflowPlannerOutput>, ComplexityAssessment, ScopePlan, FormulaSelection, bool) {
    if route_decision.route.eq_ignore_ascii_case("CHAT") {
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let scope = ScopePlan {
            objective: line.to_string(),
            ..ScopePlan::default()
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec!["capability_reply".to_string()],
            reason: "Direct conversational turn".to_string(),
            memory_id: String::new(),
        };
        return (None, complexity, scope, formula, false);
    }

    if let Ok(workflow_plan) = plan_workflow_once(
        client,
        chat_url,
        workflow_planner_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        memories,
        messages,
    )
    .await
    {
        let (complexity, scope, formula) =
            planning_prior_from_workflow_plan(line, route_decision, &workflow_plan);
        return (Some(workflow_plan), complexity, scope, formula, false);
    }

    let complexity = assess_complexity_once(
        client,
        chat_url,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or_default();
    let scope = build_scope_once(
        client,
        chat_url,
        scope_builder_cfg,
        line,
        route_decision,
        &complexity,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or_default();
    let formula = select_formula_once(
        client,
        chat_url,
        formula_cfg,
        line,
        route_decision,
        &complexity,
        &scope,
        memories,
        messages,
    )
    .await
    .unwrap_or_default();
    (None, complexity, scope, formula, true)
}

fn merged_program_from_history(plan: &AgentPlan) -> Program {
    let mut steps = Vec::new();
    for program in &plan.program_history {
        steps.extend(program.steps.clone());
    }
    Program {
        objective: plan.objective.clone(),
        steps,
    }
}

fn next_program_is_stale(plan: &AgentPlan, next_program: &Program) -> bool {
    program_signature(&plan.current_program) == program_signature(next_program)
}

fn program_has_shell_or_edit(program: &Program) -> bool {
    program.steps.iter().any(|step| matches!(step, Step::Shell { .. } | Step::Edit { .. }))
}

fn step_results_have_shell_or_edit(step_results: &[StepResult]) -> bool {
    step_results
        .iter()
        .any(|result| matches!(result.kind.as_str(), "shell" | "edit"))
}

#[allow(clippy::too_many_arguments)]
async fn run_staged_reviewers_once(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    logical_reviewer_cfg: &Profile,
    efficiency_reviewer_cfg: &Profile,
    risk_reviewer_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> (Option<CriticVerdict>, Option<CriticVerdict>, Option<RiskReviewVerdict>, bool) {
    let mut reasoning_clean = true;

    let logical = match run_critic_once(
        client,
        chat_url,
        logical_reviewer_cfg,
        line,
        route_decision,
        program,
        step_results,
        sufficiency,
        attempt,
    )
    .await
    {
        Ok(verdict) => {
            trace(
                args,
                &format!(
                    "logical_review={} reason={}",
                    verdict.status.trim(),
                    verdict.reason.trim()
                ),
            );
            Some(verdict)
        }
        Err(error) => {
            reasoning_clean = false;
            trace(args, &format!("logical_review_parse_error={error}"));
            None
        }
    };

    let efficiency = match run_critic_once(
        client,
        chat_url,
        efficiency_reviewer_cfg,
        line,
        route_decision,
        program,
        step_results,
        sufficiency,
        attempt,
    )
    .await
    {
        Ok(verdict) => {
            trace(
                args,
                &format!(
                    "efficiency_review={} reason={}",
                    verdict.status.trim(),
                    verdict.reason.trim()
                ),
            );
            Some(verdict)
        }
        Err(error) => {
            reasoning_clean = false;
            trace(args, &format!("efficiency_review_parse_error={error}"));
            None
        }
    };

    let risk = if program_has_shell_or_edit(program) || step_results_have_shell_or_edit(step_results) {
        match orchestration_helpers::request_risk_review(
            client,
            chat_url,
            risk_reviewer_cfg,
            line,
            route_decision,
            program,
            step_results,
            attempt,
        )
        .await
        {
            Ok(verdict) => {
                trace(
                    args,
                    &format!(
                        "risk_review={} reason={}",
                        verdict.status.trim(),
                        verdict.reason.trim()
                    ),
                );
                Some(verdict)
            }
            Err(error) => {
                reasoning_clean = false;
                trace(args, &format!("risk_review_parse_error={error}"));
                None
            }
        }
    } else {
        None
    };

    (logical, efficiency, risk, reasoning_clean)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_autonomous_loop(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    initial_program: Program,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    orchestrator_cfg: &Profile,
    planner_cfg: &Profile,
    planner_master_cfg: &Profile,
    decider_cfg: &Profile,
    selector_cfg: &Profile,
    summarizer_cfg: &Profile,
    command_repair_cfg: &Profile,
    command_preflight_cfg: &Profile,
    task_semantics_guard_cfg: &Profile,
    evidence_compactor_cfg: &Profile,
    artifact_classifier_cfg: &Profile,
    outcome_verifier_cfg: &Profile,
    execution_sufficiency_cfg: &Profile,
    critic_cfg: &Profile,
    logical_reviewer_cfg: &Profile,
    efficiency_reviewer_cfg: &Profile,
    risk_reviewer_cfg: &Profile,
) -> Result<AutonomousLoopOutcome> {
    let user_message = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let mut plan = AgentPlan {
        objective: initial_program.objective.clone(),
        current_program: initial_program.clone(),
        program_history: vec![initial_program],
        attempts: 0,
        executed_steps: 0,
        max_steps: 8,
        recovery_failures: 0,
    };
    let mut step_results = Vec::new();
    let mut final_reply = None;
    let mut reasoning_clean = true;
    let strict_post_judgment = !route_decision.route.eq_ignore_ascii_case("CHAT");

    loop {
        if plan.executed_steps >= plan.max_steps {
            final_reply = Some(
                "Tell the user plainly that Elma stopped because the workflow hit the maximum step budget before reaching a reliable conclusion. Suggest one narrower next step."
                    .to_string(),
            );
            break;
        }

        let (mut batch_results, batch_reply) = execute_program(
            args,
            client,
            chat_url,
            session,
            workdir,
            &plan.current_program,
            planner_cfg,
            planner_master_cfg,
            decider_cfg,
            selector_cfg,
            summarizer_cfg,
            Some(command_repair_cfg),
            Some(command_preflight_cfg),
            Some(task_semantics_guard_cfg),
            Some(evidence_compactor_cfg),
            Some(artifact_classifier_cfg),
            scope,
            complexity,
            formula,
            &plan.current_program.objective,
            false,
            false,
        )
        .await?;
        if batch_reply.is_some() {
            final_reply = batch_reply;
        }

        reasoning_clean &= verify_nontrivial_step_outcomes(
            args,
            client,
            chat_url,
            outcome_verifier_cfg,
            &user_message,
            route_decision,
            &plan.current_program,
            &mut batch_results,
        )
        .await;

        plan.executed_steps += batch_results
            .iter()
            .filter(|result| !result.kind.eq_ignore_ascii_case("reply"))
            .count();
        step_results.extend(batch_results);

        if route_decision.route.eq_ignore_ascii_case("CHAT") {
            return Ok(AutonomousLoopOutcome {
                program: merged_program_from_history(&plan),
                step_results,
                final_reply,
                reasoning_clean,
            });
        }

        if step_results.iter().any(|result| {
            result.summary.starts_with("preflight_rejected:")
                || result.summary == "preflight_requires_clarification"
        }) {
            break;
        }

        let merged_program = merged_program_from_history(&plan);
        let sufficiency = match check_execution_sufficiency_once(
            client,
            chat_url,
            execution_sufficiency_cfg,
            &user_message,
            route_decision,
            &merged_program,
            &step_results,
        )
        .await
        {
            Ok(verdict) => verdict,
            Err(error) => {
                reasoning_clean = false;
                trace(args, &format!("sufficiency_parse_error={error}"));
                if strict_post_judgment {
                    ExecutionSufficiencyVerdict {
                        status: "retry".to_string(),
                        reason: "sufficiency_parse_error".to_string(),
                        program: None,
                    }
                } else {
                    ExecutionSufficiencyVerdict {
                        status: "ok".to_string(),
                        reason: "sufficiency_parse_error".to_string(),
                        program: None,
                    }
                }
            }
        };
        trace(
            args,
            &format!(
                "sufficiency_status={} reason={}",
                sufficiency.status, sufficiency.reason
            ),
        );
        if sufficiency.status.eq_ignore_ascii_case("retry") {
            let (logical_review, efficiency_review, risk_review, reviewers_clean) =
                run_staged_reviewers_once(
                    args,
                    client,
                    chat_url,
                    logical_reviewer_cfg,
                    efficiency_reviewer_cfg,
                    risk_reviewer_cfg,
                    &user_message,
                    route_decision,
                    &merged_program,
                    &step_results,
                    Some(&sufficiency),
                    plan.attempts,
                )
                .await;
            reasoning_clean &= reviewers_clean;
            let review_reason = [
                logical_review
                    .as_ref()
                    .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                    .map(|review| format!("logical: {}", review.reason.trim())),
                efficiency_review
                    .as_ref()
                    .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                    .map(|review| format!("efficiency: {}", review.reason.trim())),
                risk_review
                    .as_ref()
                    .filter(|review| review.status.eq_ignore_ascii_case("caution"))
                    .map(|review| format!("risk: {}", review.reason.trim())),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" | ");

            let mut next_program = sufficiency
                .program
                .clone()
                .or_else(|| {
                    logical_review
                        .as_ref()
                        .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                        .and_then(|review| review.program.clone())
                })
                .or_else(|| {
                    efficiency_review
                        .as_ref()
                        .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                        .and_then(|review| review.program.clone())
                });
            if next_program.is_none() {
                operator_trace(args, "repairing the workflow plan");
                next_program = recover_program_once(
                    client,
                    chat_url,
                    orchestrator_cfg,
                    &user_message,
                    route_decision,
                    workflow_plan,
                    complexity,
                    scope,
                    formula,
                    ws,
                    ws_brief,
                    messages,
                    &format!(
                        "execution_sufficiency_retry_without_program: {}{}{}",
                        sufficiency.reason,
                        if review_reason.is_empty() { "" } else { " | " },
                        review_reason
                    ),
                    Some(&merged_program),
                    &step_results,
                )
                .await
                .ok();
            }

            let Some(next_program) = next_program else {
                plan.recovery_failures += 1;
                if plan.recovery_failures >= 2 {
                    final_reply = Some(
                        format!(
                            "Tell the user plainly that Elma could not repair the workflow after failure. Reason: {}. Ask one concise clarifying question or suggest a narrower next step.",
                            sufficiency.reason
                        ),
                    );
                    break;
                }
                continue;
            };

            if next_program_is_stale(&plan, &next_program) {
                plan.recovery_failures += 1;
                if plan.recovery_failures >= 2 {
                    final_reply = Some(
                        "Tell the user plainly that Elma stopped because recovery kept producing the same stale workflow. Ask for a narrower scope or a more specific next step."
                            .to_string(),
                    );
                    break;
                }
            } else {
                plan.recovery_failures = 0;
            }
            plan.attempts += 1;
            plan.current_program = next_program.clone();
            plan.program_history.push(next_program);
            continue;
        }

        let critic = match run_critic_once(
            client,
            chat_url,
            critic_cfg,
            &user_message,
            route_decision,
            &merged_program,
            &step_results,
            Some(&sufficiency),
            plan.attempts,
        )
        .await
        {
            Ok(verdict) => verdict,
            Err(error) => {
                reasoning_clean = false;
                trace(args, &format!("critic_parse_error={error}"));
                if strict_post_judgment {
                    CriticVerdict {
                        status: "retry".to_string(),
                        reason: "critic_parse_error".to_string(),
                        program: None,
                    }
                } else {
                    CriticVerdict {
                        status: "ok".to_string(),
                        reason: "critic_parse_error".to_string(),
                        program: None,
                    }
                }
            }
        };
        trace(
            args,
            &format!("critic_status={} reason={}", critic.status, critic.reason),
        );
        if critic.status.eq_ignore_ascii_case("retry") {
            let (logical_review, efficiency_review, risk_review, reviewers_clean) =
                run_staged_reviewers_once(
                    args,
                    client,
                    chat_url,
                    logical_reviewer_cfg,
                    efficiency_reviewer_cfg,
                    risk_reviewer_cfg,
                    &user_message,
                    route_decision,
                    &merged_program,
                    &step_results,
                    Some(&sufficiency),
                    plan.attempts,
                )
                .await;
            reasoning_clean &= reviewers_clean;
            let review_reason = [
                logical_review
                    .as_ref()
                    .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                    .map(|review| format!("logical: {}", review.reason.trim())),
                efficiency_review
                    .as_ref()
                    .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                    .map(|review| format!("efficiency: {}", review.reason.trim())),
                risk_review
                    .as_ref()
                    .filter(|review| review.status.eq_ignore_ascii_case("caution"))
                    .map(|review| format!("risk: {}", review.reason.trim())),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" | ");

            let mut next_program = critic
                .program
                .clone()
                .or_else(|| {
                    logical_review
                        .as_ref()
                        .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                        .and_then(|review| review.program.clone())
                })
                .or_else(|| {
                    efficiency_review
                        .as_ref()
                        .filter(|review| review.status.eq_ignore_ascii_case("retry"))
                        .and_then(|review| review.program.clone())
                });
            if next_program.is_none() {
                operator_trace(args, "repairing the workflow plan");
                next_program = recover_program_once(
                    client,
                    chat_url,
                    orchestrator_cfg,
                    &user_message,
                    route_decision,
                    workflow_plan,
                    complexity,
                    scope,
                    formula,
                    ws,
                    ws_brief,
                    messages,
                    &format!(
                        "critic_retry_without_program: {}{}{}",
                        critic.reason,
                        if review_reason.is_empty() { "" } else { " | " },
                        review_reason
                    ),
                    Some(&merged_program),
                    &step_results,
                )
                .await
                .ok();
            }

            let Some(next_program) = next_program else {
                plan.recovery_failures += 1;
                if plan.recovery_failures >= 2 {
                    final_reply = Some(
                        format!(
                            "Tell the user plainly that Elma could not validate the workflow result after repeated recovery failure. Reason: {}. Ask one concise clarifying question or suggest a narrower next step.",
                            critic.reason
                        ),
                    );
                    break;
                }
                continue;
            };

            if next_program_is_stale(&plan, &next_program) {
                plan.recovery_failures += 1;
                if plan.recovery_failures >= 2 {
                    final_reply = Some(
                        "Tell the user plainly that Elma stopped because critic recovery kept reproducing the same stale workflow. Ask for a narrower scope or a more specific next step."
                            .to_string(),
                    );
                    break;
                }
            } else {
                plan.recovery_failures = 0;
            }
            plan.attempts += 1;
            plan.current_program = next_program.clone();
            plan.program_history.push(next_program);
            continue;
        }

        return Ok(AutonomousLoopOutcome {
            program: merged_program,
            step_results,
            final_reply,
            reasoning_clean,
        });
    }

    Ok(AutonomousLoopOutcome {
        program: merged_program_from_history(&plan),
        step_results,
        final_reply,
        reasoning_clean,
    })
}

pub(crate) async fn orchestrate_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<(Program, String)> {
    let prompt = build_orchestrator_user_content(
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
    );
    orchestration_helpers::request_program_or_repair(client, chat_url, orchestrator_cfg, &prompt)
        .await
}

pub(crate) async fn recover_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    failure_reason: &str,
    current_program: Option<&Program>,
    step_results: &[StepResult],
) -> Result<Program> {
    let prompt = build_recovery_user_content(
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        failure_reason,
        current_program,
        step_results,
    );
    orchestration_helpers::request_recovery_program(client, chat_url, orchestrator_cfg, &prompt)
        .await
}

pub(crate) async fn run_critic_once(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> Result<CriticVerdict> {
    orchestration_helpers::request_critic_verdict(
        client,
        chat_url,
        critic_cfg,
        line,
        route_decision,
        program,
        step_results,
        sufficiency,
        attempt,
    )
    .await
}

pub(crate) async fn generate_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    evidence_mode_cfg: &Profile,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    formatter_cfg: &Profile,
    system_content: &str,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<(String, Option<u64>)> {
    let evidence_mode = decide_evidence_mode_once(
        client,
        chat_url,
        evidence_mode_cfg,
        line,
        route_decision,
        reply_instructions,
        step_results,
    )
    .await
    .unwrap_or_else(|_| EvidenceModeDecision {
        mode: "COMPACT".to_string(),
        reason: "fallback".to_string(),
    });

    let (mut final_text, mut usage_total) = if route_decision.route.eq_ignore_ascii_case("CHAT") {
        orchestration_helpers::request_chat_final_text(
            client,
            chat_url,
            elma_cfg,
            system_content,
            line,
            step_results,
            reply_instructions,
        )
        .await?
    } else {
        (
            present_result_once(
                client,
                chat_url,
                presenter_cfg,
                line,
                route_decision,
                &evidence_mode,
                step_results,
                reply_instructions,
            )
            .await
            .unwrap_or_default(),
            None,
        )
    };

    if !route_decision.route.eq_ignore_ascii_case("CHAT") && !final_text.trim().is_empty() {
        final_text = orchestration_helpers::maybe_revise_presented_result(
            client,
            chat_url,
            presenter_cfg,
            claim_checker_cfg,
            line,
            route_decision,
            &evidence_mode,
            step_results,
            reply_instructions,
            final_text,
        )
        .await;
    }

    let (formatted_text, formatted_usage) = orchestration_helpers::maybe_format_final_text(
        client,
        chat_url,
        formatter_cfg,
        line,
        final_text,
        usage_total,
    )
    .await;
    usage_total = formatted_usage;
    Ok((formatted_text, usage_total))
}

pub(crate) async fn judge_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    judge_cfg: &Profile,
    scenario: &CalibrationScenario,
    user_message: &str,
    step_results: &[StepResult],
    final_text: &str,
) -> Result<CalibrationJudgeVerdict> {
    orchestration_helpers::request_judge_verdict(
        client,
        chat_url,
        judge_cfg,
        scenario,
        user_message,
        step_results,
        final_text,
    )
    .await
}
