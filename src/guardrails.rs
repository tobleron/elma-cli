//! @efficiency-role: domain-logic
//!
//! State-Aware Guardrails Module (Task 011)
//!
//! Prevents context drift in long-running autonomous executions.
//! Monitors goal alignment and triggers refinement when agent goes off-track.

use crate::*;

// ============================================================================
// Goal Drift Detection
// ============================================================================

/// Result of goal drift check
#[derive(Debug, Clone)]
pub struct DriftVerdict {
    /// Whether drift was detected
    pub drift_detected: bool,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Reason for drift detection (if any)
    pub reason: Option<String>,
    /// Suggested correction
    pub correction: Option<String>,
}

/// Check if current execution is drifting from original goal
///
/// Analyzes:
/// 1. Step types vs. goal type mismatch
/// 2. No progress toward success criteria
/// 3. Self-referential steps (planning about planning)
pub fn check_goal_drift(
    original_objective: &str,
    current_program: &Program,
    step_results: &[StepResult],
) -> DriftVerdict {
    let mut drift_signals = Vec::new();

    // Check 1: Step types don't match goal type
    if let Some(mismatch) = check_step_goal_mismatch(original_objective, current_program) {
        drift_signals.push(mismatch);
    }

    // Check 2: No progress toward success criteria
    if let Some(no_progress) = check_no_progress(original_objective, step_results) {
        drift_signals.push(no_progress);
    }

    // Check 3: Self-referential steps (planning about planning)
    if let Some(meta_planning) = check_meta_planning(current_program) {
        drift_signals.push(meta_planning);
    }

    // Determine verdict based on signals
    if drift_signals.is_empty() {
        DriftVerdict {
            drift_detected: false,
            confidence: 1.0,
            reason: None,
            correction: None,
        }
    } else {
        let confidence = 0.5 + (drift_signals.len() as f64 * 0.15).min(0.5);
        let reason = Some(drift_signals.join("; "));
        let correction = Some(format!(
            "Refocus on original goal: \"{}\". Remove tangential steps.",
            truncate_objective(original_objective, 50)
        ));

        DriftVerdict {
            drift_detected: true,
            confidence,
            reason,
            correction,
        }
    }
}

/// Check if step types match goal type
fn check_step_goal_mismatch(objective: &str, program: &Program) -> Option<String> {
    let objective_lower = objective.to_lowercase();

    // Goal is action-oriented but steps are all read-only
    let action_keywords = [
        "delete", "remove", "add", "create", "update", "fix", "run", "execute",
    ];
    let is_action_goal = action_keywords
        .iter()
        .any(|kw| objective_lower.contains(kw));

    if is_action_goal {
        let has_action_step = program
            .steps
            .iter()
            .any(|s| matches!(s, Step::Shell { .. } | Step::Edit { .. }));

        let all_readonly = program.steps.iter().all(|s| {
            matches!(
                s,
                Step::Read { .. } | Step::Search { .. } | Step::Plan { .. }
            )
        });

        if all_readonly && !has_action_step && program.steps.len() >= 3 {
            return Some(format!(
                "Goal requires action but {} steps are read-only (no Shell/Edit steps)",
                program.steps.len()
            ));
        }
    }

    // Goal is research but steps are destructive
    let research_keywords = ["research", "analyze", "understand", "learn", "compare"];
    let is_research_goal = research_keywords
        .iter()
        .any(|kw| objective_lower.contains(kw));

    if is_research_goal {
        let has_destructive = program.steps.iter().any(|s| {
            if let Step::Shell { cmd, .. } = s {
                cmd.contains("rm ") || cmd.contains("delete") || cmd.contains("drop")
            } else {
                false
            }
        });

        if has_destructive {
            return Some("Research goal but steps include destructive operations".to_string());
        }
    }

    None
}

/// Check if there's no progress toward success
fn check_no_progress(objective: &str, step_results: &[StepResult]) -> Option<String> {
    // If we've executed 5+ steps with no successful modifications
    let executed_steps = step_results
        .iter()
        .filter(|s| !s.kind.eq_ignore_ascii_case("reply"))
        .count();

    if executed_steps >= 5 {
        let successful_modifications = step_results
            .iter()
            .filter(|s| {
                s.ok && (s.kind.eq_ignore_ascii_case("edit")
                    || s.kind.eq_ignore_ascii_case("shell") && s.exit_code == Some(0))
            })
            .count();

        if successful_modifications == 0 {
            return Some(format!(
                "{} steps executed with 0 successful modifications",
                executed_steps
            ));
        }
    }

    None
}

/// Check for self-referential planning (planning about planning)
fn check_meta_planning(program: &Program) -> Option<String> {
    let masterplan_count = program
        .steps
        .iter()
        .filter(|s| matches!(s, Step::MasterPlan { .. }))
        .count();
    let concrete_plan_count = program
        .steps
        .iter()
        .filter(|s| matches!(s, Step::Plan { .. }))
        .count();
    let plan_count = program
        .steps
        .iter()
        .filter(|s| matches!(s, Step::Plan { .. } | Step::MasterPlan { .. }))
        .count();

    let total_steps = program.steps.len();

    // A bounded "strategic roadmap + one concrete phase plan + reply" is valid.
    if masterplan_count == 1 && concrete_plan_count == 1 && total_steps <= 4 {
        return None;
    }

    // If more than half the steps are planning steps, we're planning about planning
    if plan_count >= 2 && plan_count * 2 >= total_steps && total_steps >= 3 {
        return Some(format!(
            "{} planning steps out of {} total (meta-planning detected)",
            plan_count, total_steps
        ));
    }

    None
}

// ============================================================================
// Helper Functions
// ============================================================================

pub(crate) fn truncate_objective(obj: &str, max_len: usize) -> String {
    if obj.chars().count() <= max_len {
        obj.to_string()
    } else {
        format!("{}...", obj.chars().take(max_len).collect::<String>())
    }
}
