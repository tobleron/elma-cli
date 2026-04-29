//! @efficiency-role: domain-logic
//!
//! Goal Consistency Intel Unit
//!
//! One job: fire every 18 tool calls to check if the current tool-call
//! trajectory still serves the original objective. Output:
//! CONSISTENT, DRIFTING, or OFF_TRACK with a steering message.

use crate::intel_trait::*;
use crate::*;

/// Verdict from the goal consistency check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GoalConsistencyVerdict {
    pub status: String,    // CONsISTENT | DRIFTING | OFF_TRACK
    pub reason: String,    // one short sentence
    pub steering: String,  // what to focus on instead (empty if CONSISTENT)
}

/// Run the goal consistency check as a fire-and-forget intel call.
///
/// Returns `None` if the call fails (network, model error, parse error) —
/// the tool loop continues regardless. Only DRIFTING/OFF_TRACK verdicts
/// produce a non-empty steering message.
pub(crate) async fn run_goal_consistency_check(
    client: &reqwest::Client,
    profile: &Profile,
    goal_state: &GoalState,
    recent_tool_summary: &str,
) -> Option<String> {
    let objective = goal_state.active_objective.as_deref()?;

    // Only run the check if there's an active objective and subgoals to track against
    if objective.trim().is_empty() {
        return None;
    }

    let pending = goal_state
        .pending_subgoals
        .iter()
        .map(|s| format!("- {}", s))
        .collect::<Vec<_>>()
        .join("\n");

    let completed = goal_state
        .completed_subgoals
        .iter()
        .map(|s| format!("- {}", s))
        .collect::<Vec<_>>()
        .join("\n");

    let narrative = format!(
        r#"OBJECTIVE:
{objective}

PENDING SUBGOALS:
{pending}

COMPLETED SUBGOALS:
{completed}

RECENT TOOL CALLS:
{recent_tool_summary}

TASK:
Compare the recent tool-call trajectory against the original objective and subgoals.
CONSISTENT — the tools directly serve the objective and subgoals.
DRIFTING — the tools are somewhat related but losing focus or exploring tangents.
OFF_TRACK — the tools no longer serve the original objective at all.

Output contract:
{{"status": "CONSISTENT|DRIFTING|OFF_TRACK", "reason": "one short sentence", "steering": "specific redirection message (empty if CONSISTENT)"}}"#,
    );

    let result: serde_json::Value =
        match execute_intel_json_from_user_content(client, profile, narrative).await {
            Ok(v) => v,
            Err(e) => {
                trace_fallback("goal_consistency", &format!("LLM call failed: {}", e));
                return None;
            }
        };

    let verdict: GoalConsistencyVerdict = match serde_json::from_value(result) {
        Ok(v) => v,
        Err(e) => {
            trace_fallback(
                "goal_consistency",
                &format!("parse failed: {}", e),
            );
            return None;
        }
    };

    match verdict.status.as_str() {
        "CONSISTENT" => None,
        "DRIFTING" | "OFF_TRACK" => {
            if verdict.steering.trim().is_empty() {
                None
            } else {
                Some(verdict.steering)
            }
        }
        _ => {
            // Unknown status — treat as no-op
            trace_fallback(
                "goal_consistency",
                &format!("unexpected status: {}", verdict.status),
            );
            None
        }
    }
}
