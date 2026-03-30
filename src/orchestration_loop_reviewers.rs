//! @efficiency-role: service-orchestrator
//!
//! Orchestration Loop - Reviewer Coordination

use crate::orchestration_loop_helpers::*;
use crate::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_staged_reviewers_once(
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
