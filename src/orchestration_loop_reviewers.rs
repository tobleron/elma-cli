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
            record_json_failure(args, "logical_reviewer");
            reasoning_clean = false;
            trace(args, &format!("logical_review_parse_error={error}"));
            // FALLBACK: Assume ok rather than block execution
            let fallback = default_critic_verdict();
            log_fallback_usage(args, "logical_reviewer", &error.to_string(), "default_ok");
            Some(fallback)
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
            record_json_failure(args, "efficiency_reviewer");
            reasoning_clean = false;
            trace(args, &format!("efficiency_review_parse_error={error}"));
            // FALLBACK: Assume ok rather than block execution
            let fallback = default_critic_verdict();
            log_fallback_usage(args, "efficiency_reviewer", &error.to_string(), "default_ok");
            Some(fallback)
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
                record_json_failure(args, "risk_reviewer");
                reasoning_clean = false;
                trace(args, &format!("risk_review_parse_error={error}"));
                // FALLBACK: Assume caution rather than block execution
                let fallback = RiskReviewVerdict {
                    status: "caution".to_string(),
                    reason: "Risk review unavailable, proceed with caution".to_string(),
                };
                log_fallback_usage(args, "risk_reviewer", &error.to_string(), "default_caution");
                Some(fallback)
            }
        }
    } else {
        None
    };

    (logical, efficiency, risk, reasoning_clean)
}
