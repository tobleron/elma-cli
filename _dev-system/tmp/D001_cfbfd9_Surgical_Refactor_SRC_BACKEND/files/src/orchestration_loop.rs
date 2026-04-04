//! @efficiency-role: service-orchestrator
//!
//! Orchestration Loop Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - orchestration_loop_helpers: Helper functions
//! - orchestration_loop_reviewers: Reviewer coordination

use crate::orchestration_loop_helpers::*;
use crate::orchestration_loop_reviewers::*;
use crate::*;

fn has_trusted_exact_selection_result(step_results: &[StepResult]) -> bool {
    step_results.iter().any(|r| {
        r.kind == "select"
            && r.ok
            && r.raw_output
                .as_ref()
                .is_some_and(|raw| raw.lines().any(|l| l.trim().contains('/')))
    })
}

fn should_accept_exact_selection_workflow(
    step_results: &[StepResult],
    missing_required_evidence: bool,
    sufficiency: &ExecutionSufficiencyVerdict,
) -> bool {
    !missing_required_evidence
        && sufficiency.status.eq_ignore_ascii_case("retry")
        && has_trusted_exact_selection_result(step_results)
        && step_results.iter().all(|r| r.ok)
}

fn collect_review_reasons(
    logical: Option<&CriticVerdict>,
    efficiency: Option<&CriticVerdict>,
    risk: Option<&RiskReviewVerdict>,
) -> String {
    [
        logical
            .filter(|r| r.status.eq_ignore_ascii_case("retry"))
            .map(|r| format!("logical: {}", r.reason.trim())),
        efficiency
            .filter(|r| r.status.eq_ignore_ascii_case("retry"))
            .map(|r| format!("efficiency: {}", r.reason.trim())),
        risk.filter(|r| r.status.eq_ignore_ascii_case("caution"))
            .map(|r| format!("risk: {}", r.reason.trim())),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" | ")
}

fn pick_program_from_reviews(
    sufficiency: &ExecutionSufficiencyVerdict,
    missing_evidence: bool,
    logical: Option<&CriticVerdict>,
    efficiency: Option<&CriticVerdict>,
) -> Option<Program> {
    if missing_evidence {
        None
    } else {
        sufficiency.program.clone()
    }
    .or_else(|| {
        logical
            .filter(|r| r.status.eq_ignore_ascii_case("retry"))
            .and_then(|r| r.program.clone())
    })
    .or_else(|| {
        efficiency
            .filter(|r| r.status.eq_ignore_ascii_case("retry"))
            .and_then(|r| r.program.clone())
    })
}

fn handle_verdict_parse_error<T>(
    args: &Args,
    error: &str,
    strict: bool,
    results_good: bool,
    ok_verdict: impl FnOnce() -> T,
    retry_verdict: impl FnOnce() -> T,
) -> T {
    trace(args, &format!("{error}"));
    if strict && !results_good {
        retry_verdict()
    } else {
        if strict {
            trace(
                args,
                &format!("{error}: assuming ok due to successful outcome verification"),
            );
        }
        ok_verdict()
    }
}
fn handle_sufficiency_parse_error(
    args: &Args,
    error: &str,
    strict: bool,
    results_good: bool,
) -> ExecutionSufficiencyVerdict {
    handle_verdict_parse_error(
        args,
        error,
        strict,
        results_good,
        || ExecutionSufficiencyVerdict {
            status: "ok".to_string(),
            reason: "sufficiency_parse_error_assumed_ok".to_string(),
            program: None,
        },
        || ExecutionSufficiencyVerdict {
            status: "retry".to_string(),
            reason: "sufficiency_parse_error".to_string(),
            program: None,
        },
    )
}
fn handle_critic_parse_error(
    args: &Args,
    error: &str,
    strict: bool,
    results_good: bool,
) -> CriticVerdict {
    handle_verdict_parse_error(
        args,
        error,
        strict,
        results_good,
        || CriticVerdict {
            status: "ok".to_string(),
            reason: "critic_parse_error_assumed_ok".to_string(),
            program: None,
        },
        || CriticVerdict {
            status: "retry".to_string(),
            reason: "critic_parse_error".to_string(),
            program: None,
        },
    )
}

fn handle_recovery_failure(plan: &mut AgentPlan, reason: &str) -> Option<String> {
    plan.recovery_failures += 1;
    if plan.recovery_failures >= 2 {
        Some(format!("Tell the user plainly that Elma could not repair the workflow after failure. Reason: {}. Ask one concise clarifying question or suggest a narrower next step.", reason))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
async fn attempt_recovery(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    reason: &str,
    current_program: &Program,
    step_results: &[StepResult],
) -> Option<Program> {
    recover_program_once(
        client,
        chat_url,
        orchestrator_cfg,
        user_message,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        reason,
        Some(current_program),
        step_results,
    )
    .await
    .ok()
}

fn has_preflight_failure(step_results: &[StepResult]) -> bool {
    step_results.iter().any(|r| {
        r.summary.starts_with("preflight_rejected:")
            || r.summary.starts_with("preflight_requires_clarification")
            || r.summary.starts_with("command_unavailable:")
    })
}
fn results_are_good(rs: &[StepResult]) -> bool {
    !rs.is_empty()
        && rs.iter().all(|r| {
            r.ok && r
                .outcome_status
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case("ok"))
                .unwrap_or(true)
        })
}
fn count_executed_steps(rs: &[StepResult]) -> usize {
    rs.iter()
        .filter(|r| !r.kind.eq_ignore_ascii_case("reply"))
        .count()
}
fn handle_drift(args: &Args, v: &DriftVerdict) -> bool {
    if !v.drift_detected {
        return false;
    }
    trace(
        args,
        &format!(
            "guardrail_drift_detected confidence={:.2} reason={}",
            v.confidence,
            v.reason.as_deref().unwrap_or("unknown")
        ),
    );
    operator_trace(args, "context drift detected - running refinement phase");
    true
}

#[allow(clippy::too_many_arguments)]
async fn run_reviewers_and_recover(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    logical_reviewer_cfg: &Profile,
    efficiency_reviewer_cfg: &Profile,
    risk_reviewer_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    merged_program: &Program,
    step_results: &[StepResult],
    sufficiency: &ExecutionSufficiencyVerdict,
    missing_required_evidence: bool,
    orchestrator_cfg: &Profile,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    plan: &mut AgentPlan,
    reasoning_clean: &mut bool,
) -> Result<Option<Program>> {
    let (logical_review, efficiency_review, risk_review, reviewers_clean) =
        run_staged_reviewers_once(
            args,
            client,
            chat_url,
            logical_reviewer_cfg,
            efficiency_reviewer_cfg,
            risk_reviewer_cfg,
            user_message,
            route_decision,
            merged_program,
            step_results,
            Some(sufficiency),
            plan.attempts,
        )
        .await;
    *reasoning_clean &= reviewers_clean;
    let review_reason = collect_review_reasons(
        logical_review.as_ref(),
        efficiency_review.as_ref(),
        risk_review.as_ref(),
    );

    let mut next_program = pick_program_from_reviews(
        sufficiency,
        missing_required_evidence,
        logical_review.as_ref(),
        efficiency_review.as_ref(),
    );
    if next_program.is_none() {
        operator_trace(args, "repairing the workflow plan");
        next_program = attempt_recovery(
            client,
            chat_url,
            orchestrator_cfg,
            user_message,
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
                if missing_required_evidence {
                    "missing_required_workspace_evidence"
                } else {
                    sufficiency.reason.as_str()
                },
                if review_reason.is_empty() { "" } else { " | " },
                review_reason
            ),
            merged_program,
            step_results,
        )
        .await;
    }
    Ok(next_program)
}

fn check_should_retry(
    sufficiency: &ExecutionSufficiencyVerdict,
    missing_required_evidence: bool,
    accept_exact_selection: bool,
) -> bool {
    missing_required_evidence
        || (sufficiency.status.eq_ignore_ascii_case("retry") && !accept_exact_selection)
}

#[allow(clippy::too_many_arguments)]
async fn handle_policy_mismatch(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    plan: &mut AgentPlan,
    step_results: &[StepResult],
    policy_reason: &str,
) -> Result<Option<String>> {
    trace(
        args,
        &format!(
            "program_policy_mismatch level={:?} reason={policy_reason}",
            complexity.complexity
        ),
    );
    operator_trace(args, "repairing the workflow plan");
    let recovered = recover_program_once(
        client,
        chat_url,
        orchestrator_cfg,
        user_message,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &format!("program_policy_mismatch: {policy_reason}"),
        Some(&plan.current_program),
        step_results,
    )
    .await
    .ok();
    let Some(next_program) = recovered else {
        return Ok(Some("Tell the user plainly that Elma could not build an evidence-grounded workflow for this request. Ask one concise clarifying question or suggest a narrower next step.".to_string()));
    };
    if next_program_is_stale(plan, &next_program) {
        return Ok(Some("Tell the user plainly that Elma stopped because recovery kept reproducing a workflow without the needed evidence steps. Ask for a narrower scope or a more specific next step.".to_string()));
    }
    plan.attempts += 1;
    plan.current_program = next_program.clone();
    plan.program_history.push(next_program);
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
async fn handle_sufficiency_retry(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    logical_reviewer_cfg: &Profile,
    efficiency_reviewer_cfg: &Profile,
    risk_reviewer_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    merged_program: &Program,
    step_results: &[StepResult],
    sufficiency: &ExecutionSufficiencyVerdict,
    missing_required_evidence: bool,
    orchestrator_cfg: &Profile,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    plan: &mut AgentPlan,
    reasoning_clean: &mut bool,
) -> Result<Option<String>> {
    let next_program = run_reviewers_and_recover(
        args,
        client,
        chat_url,
        logical_reviewer_cfg,
        efficiency_reviewer_cfg,
        risk_reviewer_cfg,
        user_message,
        route_decision,
        merged_program,
        step_results,
        sufficiency,
        missing_required_evidence,
        orchestrator_cfg,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        plan,
        reasoning_clean,
    )
    .await?;
    let Some(next_program) = next_program else {
        if let Some(msg) = handle_recovery_failure(plan, &sufficiency.reason) {
            return Ok(Some(msg));
        }
        return Ok(None);
    };
    if next_program_is_stale(plan, &next_program) {
        plan.recovery_failures += 1;
        if plan.recovery_failures >= 2 {
            return Ok(Some("Tell the user plainly that Elma stopped because recovery kept producing the same stale workflow. Ask for a narrower scope or a more specific next step.".to_string()));
        }
    } else {
        plan.recovery_failures = 0;
    }
    plan.attempts += 1;
    plan.current_program = next_program.clone();
    plan.program_history.push(next_program);
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
async fn handle_critic_retry(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    critic: &CriticVerdict,
    logical_reviewer_cfg: &Profile,
    efficiency_reviewer_cfg: &Profile,
    risk_reviewer_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    merged_program: &Program,
    step_results: &[StepResult],
    sufficiency: &ExecutionSufficiencyVerdict,
    orchestrator_cfg: &Profile,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    plan: &mut AgentPlan,
    reasoning_clean: &mut bool,
) -> Result<Option<String>> {
    let (logical_review, efficiency_review, risk_review, reviewers_clean) =
        run_staged_reviewers_once(
            args,
            client,
            chat_url,
            logical_reviewer_cfg,
            efficiency_reviewer_cfg,
            risk_reviewer_cfg,
            user_message,
            route_decision,
            merged_program,
            step_results,
            Some(sufficiency),
            plan.attempts,
        )
        .await;
    *reasoning_clean &= reviewers_clean;
    let review_reason = collect_review_reasons(
        logical_review.as_ref(),
        efficiency_review.as_ref(),
        risk_review.as_ref(),
    );
    let mut next_program = critic
        .program
        .clone()
        .or_else(|| {
            logical_review
                .as_ref()
                .filter(|r| r.status.eq_ignore_ascii_case("retry"))
                .and_then(|r| r.program.clone())
        })
        .or_else(|| {
            efficiency_review
                .as_ref()
                .filter(|r| r.status.eq_ignore_ascii_case("retry"))
                .and_then(|r| r.program.clone())
        });
    if next_program.is_none() {
        operator_trace(args, "repairing the workflow plan");
        next_program = attempt_recovery(
            client,
            chat_url,
            orchestrator_cfg,
            user_message,
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
            merged_program,
            step_results,
        )
        .await;
    }
    let Some(next_program) = next_program else {
        if let Some(msg) = handle_recovery_failure(plan, &critic.reason) {
            return Ok(Some(msg));
        }
        return Ok(None);
    };
    if next_program_is_stale(plan, &next_program) {
        plan.recovery_failures += 1;
        if plan.recovery_failures >= 2 {
            return Ok(Some("Tell the user plainly that Elma stopped because critic recovery kept reproducing the same stale workflow. Ask for a narrower scope or a more specific next step.".to_string()));
        }
    } else {
        plan.recovery_failures = 0;
    }
    plan.attempts += 1;
    plan.program_history.push(next_program);
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
async fn try_refinement_if_needed(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    merged_program: &Program,
    step_results: &mut Vec<StepResult>,
    final_reply: &mut Option<String>,
    plan: &mut AgentPlan,
    status_message_cfg: &Profile,
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
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    refinement_cfg: &Profile,
) -> Result<()> {
    let achievement = check_objective_achievement(&merged_program.objective, step_results);
    if achievement.is_achieved || step_results.is_empty() {
        return Ok(());
    }
    trace(
        args,
        &format!(
            "refinement_needed iteration=1 confidence={:.1} gaps={}",
            achievement.confidence,
            achievement.gaps.len()
        ),
    );
    let refinement_ctx = RefinementContext {
        original_objective: merged_program.objective.clone(),
        step_results: step_results.clone(),
        current_program: merged_program.clone(),
        iteration: 0,
        refinement_reason: format!(
            "Objective not achieved (confidence: {:.1}%). Gaps: {}",
            achievement.confidence * 100.0,
            achievement.gaps.join("; ")
        ),
    };
    let Ok(refined_program) = refine_program(
        client,
        chat_url,
        refinement_cfg,
        &refinement_ctx,
        &achievement,
    )
    .await
    else {
        return Ok(());
    };
    trace(args, "refinement_success applying refined program");
    let (refined_results, refined_reply) = execute_program(
        args,
        client,
        chat_url,
        session,
        workdir,
        &refined_program,
        status_message_cfg,
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
        &merged_program.objective,
        false,
        false,
    )
    .await?;
    step_results.extend(refined_results);
    if refined_reply.is_some() {
        *final_reply = refined_reply;
    }
    plan.current_program = refined_program;
    plan.program_history.push(plan.current_program.clone());
    Ok(())
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
    status_message_cfg: &Profile,
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
    refinement_cfg: &Profile,
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
    let strict = !route_decision.route.eq_ignore_ascii_case("CHAT");

    loop {
        if plan.executed_steps >= plan.max_steps {
            final_reply = Some("Tell the user plainly that Elma stopped because the workflow hit the maximum step budget before reaching a reliable conclusion. Suggest one narrower next step.".to_string());
            break;
        }
        if let Err(reason) = validate_evidence_requirements(
            &plan.current_program,
            route_decision,
            complexity,
            formula,
        ) {
            if let Some(msg) = handle_policy_mismatch(
                args,
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
                &mut plan,
                &step_results,
                &reason,
            )
            .await?
            {
                final_reply = Some(msg);
                break;
            }
            continue;
        }

        let (mut batch_results, batch_reply) = execute_program(
            args,
            client,
            chat_url,
            session,
            workdir,
            &plan.current_program,
            status_message_cfg,
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
        if batch_reply.is_none() {
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
        }
        let good = results_are_good(&batch_results);
        plan.executed_steps += count_executed_steps(&batch_results);
        step_results.extend(batch_results);
        if batch_reply.is_some() {
            final_reply = batch_reply;
        }

        let drift = check_goal_drift(&plan.objective, &plan.current_program, &step_results);
        if handle_drift(args, &drift) {
            let refined = run_refinement_phase(
                client,
                chat_url,
                refinement_cfg,
                &plan.objective,
                &step_results,
                drift.reason.as_deref().unwrap_or("Goal alignment lost"),
                ws,
                ws_brief,
            )
            .await?;
            plan.current_program = refined.clone();
            plan.program_history.push(refined);
            plan.attempts += 1;
            continue;
        }
        if route_decision.route.eq_ignore_ascii_case("CHAT") {
            return Ok(AutonomousLoopOutcome {
                program: merged_program_from_history(&plan),
                step_results,
                final_reply,
                reasoning_clean,
            });
        }
        if has_preflight_failure(&step_results) {
            break;
        }

        let merged = merged_program_from_history(&plan);
        let missing_evidence =
            request_requires_workspace_evidence(route_decision, complexity, formula)
                && !step_results_have_workspace_evidence(&step_results);
        let sufficiency = match check_execution_sufficiency_once(
            client,
            chat_url,
            execution_sufficiency_cfg,
            &user_message,
            route_decision,
            &merged,
            &step_results,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                reasoning_clean = false;
                handle_sufficiency_parse_error(
                    args,
                    &format!("sufficiency_parse_error={e}"),
                    strict,
                    good,
                )
            }
        };
        trace(
            args,
            &format!(
                "sufficiency_status={} reason={}",
                sufficiency.status, sufficiency.reason
            ),
        );
        let accept_exact =
            should_accept_exact_selection_workflow(&step_results, missing_evidence, &sufficiency);
        if accept_exact {
            trace(
                args,
                "sufficiency_status=ok reason=trusted_exact_selection_workflow",
            );
        }
        if missing_evidence {
            trace(
                args,
                "sufficiency_status=retry reason=missing_required_workspace_evidence",
            );
        }

        if check_should_retry(&sufficiency, missing_evidence, accept_exact) {
            if let Some(msg) = handle_sufficiency_retry(
                args,
                client,
                chat_url,
                logical_reviewer_cfg,
                efficiency_reviewer_cfg,
                risk_reviewer_cfg,
                &user_message,
                route_decision,
                &merged,
                &step_results,
                &sufficiency,
                missing_evidence,
                orchestrator_cfg,
                workflow_plan,
                complexity,
                scope,
                formula,
                ws,
                ws_brief,
                messages,
                &mut plan,
                &mut reasoning_clean,
            )
            .await?
            {
                final_reply = Some(msg);
                break;
            }
            continue;
        }

        let critic = match run_critic_once(
            client,
            chat_url,
            critic_cfg,
            &user_message,
            route_decision,
            &merged,
            &step_results,
            Some(&sufficiency),
            plan.attempts,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                reasoning_clean = false;
                handle_critic_parse_error(args, &format!("critic_parse_error={e}"), strict, good)
            }
        };
        trace(
            args,
            &format!("critic_status={} reason={}", critic.status, critic.reason),
        );
        if critic.status.eq_ignore_ascii_case("retry") {
            if let Some(msg) = handle_critic_retry(
                args,
                client,
                chat_url,
                &critic,
                logical_reviewer_cfg,
                efficiency_reviewer_cfg,
                risk_reviewer_cfg,
                &user_message,
                route_decision,
                &merged,
                &step_results,
                &sufficiency,
                orchestrator_cfg,
                workflow_plan,
                complexity,
                scope,
                formula,
                ws,
                ws_brief,
                messages,
                &mut plan,
                &mut reasoning_clean,
            )
            .await?
            {
                final_reply = Some(msg);
                break;
            }
            continue;
        }

        let merged = merged_program_from_history(&plan);
        try_refinement_if_needed(
            args,
            client,
            chat_url,
            session,
            workdir,
            &merged,
            &mut step_results,
            &mut final_reply,
            &mut plan,
            status_message_cfg,
            planner_cfg,
            planner_master_cfg,
            decider_cfg,
            selector_cfg,
            summarizer_cfg,
            command_repair_cfg,
            command_preflight_cfg,
            task_semantics_guard_cfg,
            evidence_compactor_cfg,
            artifact_classifier_cfg,
            scope,
            complexity,
            formula,
            refinement_cfg,
        )
        .await?;

        return Ok(AutonomousLoopOutcome {
            program: merged,
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

#[cfg(test)]
mod tests {
    use super::should_accept_exact_selection_workflow;
    use crate::{ExecutionSufficiencyVerdict, StepResult};

    #[test]
    fn accepts_retry_override_for_successful_exact_selection_workflow() {
        let step_results = vec![StepResult {
            id: "sel1".to_string(),
            kind: "select".to_string(),
            ok: true,
            raw_output: Some("_stress_testing/_opencode_for_testing/main.go".to_string()),
            ..StepResult::default()
        }];
        let sufficiency = ExecutionSufficiencyVerdict {
            status: "retry".to_string(),
            reason: "hallucinated mismatch".to_string(),
            program: None,
        };

        assert!(should_accept_exact_selection_workflow(
            &step_results,
            false,
            &sufficiency,
        ));
    }
}
