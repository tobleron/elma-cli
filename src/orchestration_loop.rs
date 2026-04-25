//! @efficiency-role: service-orchestrator
//!
//! Orchestration Loop Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - orchestration_loop_helpers: Helper functions
//! - orchestration_loop_reviewers: Reviewer coordination
//! - orchestration_loop_verdicts: Verdict handlers

use crate::orchestration_loop_helpers::*;
use crate::orchestration_loop_reviewers::*;
use crate::orchestration_loop_verdicts::*;
use crate::*;

pub(crate) fn has_trusted_exact_selection_result(step_results: &[StepResult]) -> bool {
    step_results.iter().any(|r| {
        r.kind == "select"
            && r.ok
            && r.raw_output
                .as_ref()
                .is_some_and(|raw| raw.lines().any(|l| l.trim().contains('/')))
    })
}

pub(crate) fn should_accept_exact_selection_workflow(
    step_results: &[StepResult],
    missing_required_evidence: bool,
    sufficiency: &ExecutionSufficiencyVerdict,
) -> bool {
    !missing_required_evidence
        && sufficiency.status.eq_ignore_ascii_case("retry")
        && has_trusted_exact_selection_result(step_results)
        && step_results.iter().all(|r| r.ok)
}

pub(crate) fn collect_review_reasons(
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

pub(crate) fn pick_program_from_reviews(
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

fn check_should_retry(
    sufficiency: &ExecutionSufficiencyVerdict,
    missing_required_evidence: bool,
    accept_exact_selection: bool,
) -> bool {
    missing_required_evidence
        || (sufficiency.status.eq_ignore_ascii_case("retry") && !accept_exact_selection)
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
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
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
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
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
                &reason.to_string(),
            )
            .await?
            {
                final_reply = Some(msg);
                break;
            }
            continue;
        }

        // Log concurrency safety flags for each step (Task 265)
        for step in &plan.current_program.steps {
            let common = crate::step_common(step);
            trace(
                args,
                &format!(
                    "step id={} is_concurrency_safe={}",
                    crate::step_id(step),
                    common.is_concurrency_safe
                ),
            );
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
            tui.as_deref_mut(),
        )
        .await?;
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
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
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
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
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
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
            tui.as_deref_mut(),
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
