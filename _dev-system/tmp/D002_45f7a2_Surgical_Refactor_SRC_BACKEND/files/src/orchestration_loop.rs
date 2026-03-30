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

        let results_are_good = !batch_results.is_empty() && batch_results.iter().all(|r| {
            r.ok && r.outcome_status.as_deref().map(|s| s.eq_ignore_ascii_case("ok")).unwrap_or(true)
        });

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
                || result
                    .summary
                    .starts_with("preflight_requires_clarification")
                || result.summary.starts_with("command_unavailable:")
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

                if strict_post_judgment && !results_are_good {
                    ExecutionSufficiencyVerdict {
                        status: "retry".to_string(),
                        reason: "sufficiency_parse_error".to_string(),
                        program: None,
                    }
                } else {
                    if strict_post_judgment {
                        trace(args, "sufficiency_parse_error: assuming ok due to successful outcome verification");
                    }
                    ExecutionSufficiencyVerdict {
                        status: "ok".to_string(),
                        reason: "sufficiency_parse_error_assumed_ok".to_string(),
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

                if strict_post_judgment && !results_are_good {
                    CriticVerdict {
                        status: "retry".to_string(),
                        reason: "critic_parse_error".to_string(),
                        program: None,
                    }
                } else {
                    if strict_post_judgment {
                        trace(args, "critic_parse_error: assuming ok due to successful outcome verification");
                    }
                    CriticVerdict {
                        status: "ok".to_string(),
                        reason: "critic_parse_error_assumed_ok".to_string(),
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
            plan.program_history.push(next_program);
            continue;
        }

        let merged_program = merged_program_from_history(&plan);
        let achievement = check_objective_achievement(&merged_program.objective, &step_results);

        if !achievement.is_achieved && step_results.len() > 0 {
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

            match refine_program(client, chat_url, refinement_cfg, &refinement_ctx, &achievement).await {
                Ok(refined_program) => {
                    trace(args, "refinement_success applying refined program");
                    let (refined_results, refined_reply) = execute_program(
                        args,
                        client,
                        chat_url,
                        session,
                        workdir,
                        &refined_program,
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
                    ).await?;

                    step_results.extend(refined_results);
                    if refined_reply.is_some() {
                        final_reply = refined_reply;
                    }

                    plan.current_program = refined_program;
                    plan.program_history.push(plan.current_program.clone());
                }
                Err(error) => {
                    trace(args, &format!("refinement_failed error={}", error));
                }
            }
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
