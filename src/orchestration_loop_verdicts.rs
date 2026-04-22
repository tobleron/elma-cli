//! @efficiency-role: service-orchestrator
//!
//! Verdict handler functions for the orchestration loop.
//!
//! Contains handlers for policy mismatch, sufficiency retry, critic retry,
//! drift detection, recovery attempts, reviewer coordination, and refinement.

use crate::orchestration_loop_helpers::*;
use crate::orchestration_loop_reviewers::*;
use crate::*;

pub(crate) fn handle_verdict_parse_error<T>(
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

pub(crate) fn handle_sufficiency_parse_error(
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

pub(crate) fn handle_critic_parse_error(
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

pub(crate) fn handle_recovery_failure(plan: &mut AgentPlan, reason: &str) -> Option<String> {
    plan.recovery_failures += 1;
    if plan.recovery_failures >= 2 {
        Some(format!("Tell the user plainly that Elma could not repair the workflow after failure. Reason: {}. Ask one concise clarifying question or suggest a narrower next step.", reason))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn attempt_recovery(
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

pub(crate) fn has_preflight_failure(step_results: &[StepResult]) -> bool {
    step_results.iter().any(|r| {
        r.summary.starts_with("preflight_rejected:")
            || r.summary.starts_with("preflight_requires_clarification")
            || r.summary.starts_with("command_unavailable:")
    })
}

pub(crate) fn results_are_good(rs: &[StepResult]) -> bool {
    !rs.is_empty()
        && rs.iter().all(|r| {
            r.ok && r
                .outcome_status
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case("ok"))
                .unwrap_or(true)
        })
}

pub(crate) fn count_executed_steps(rs: &[StepResult]) -> usize {
    rs.iter()
        .filter(|r| !r.kind.eq_ignore_ascii_case("reply"))
        .count()
}

pub(crate) fn handle_drift(args: &Args, v: &DriftVerdict) -> bool {
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
pub(crate) async fn run_reviewers_and_recover(
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
    let review_reason = crate::orchestration_loop::collect_review_reasons(
        logical_review.as_ref(),
        efficiency_review.as_ref(),
        risk_review.as_ref(),
    );

    let mut next_program = crate::orchestration_loop::pick_program_from_reviews(
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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_policy_mismatch(
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
pub(crate) async fn handle_sufficiency_retry(
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
pub(crate) async fn handle_critic_retry(
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
    let review_reason = crate::orchestration_loop::collect_review_reasons(
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
pub(crate) async fn try_refinement_if_needed(
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
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
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
        tui.as_deref_mut(),
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
