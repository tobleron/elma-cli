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
// Refinement Phase
// ============================================================================

/// Run refinement phase to get back on track
pub async fn run_refinement_phase(
    client: &reqwest::Client,
    chat_url: &Url,
    refinement_cfg: &Profile,
    original_objective: &str,
    step_results: &[StepResult],
    drift_reason: &str,
    ws: &str,
    ws_brief: &str,
) -> Result<Program> {
    // Build refinement prompt
    let prompt =
        build_refinement_prompt(original_objective, step_results, drift_reason, ws, ws_brief);

    let req = ChatCompletionRequest {
        model: refinement_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: refinement_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: refinement_cfg.temperature,
        top_p: refinement_cfg.top_p,
        stream: false,
        max_tokens: refinement_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(refinement_cfg.repeat_penalty),
        reasoning_format: Some(refinement_cfg.reasoning_format.clone()),
        grammar: Some(crate::json_program_grammar()),
    };

    let (program, _) = crate::chat_json_with_repair_text(client, chat_url, &req).await?;
    Ok(program)
}

/// Build refinement prompt
fn build_refinement_prompt(
    original_objective: &str,
    step_results: &[StepResult],
    drift_reason: &str,
    ws: &str,
    ws_brief: &str,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("=== CONTEXT DRIFT DETECTED ===\n\n");

    prompt.push_str(&format!("**Original Goal:** {}\n\n", original_objective));

    prompt.push_str("**Why We're Off-Track:**\n");
    prompt.push_str(&format!("{}\n\n", drift_reason));

    prompt.push_str("**What We've Done So Far:**\n");
    for (i, result) in step_results.iter().enumerate() {
        let status = if result.ok { "✅" } else { "❌" };
        prompt.push_str(&format!(
            "{}. {} [{}]: {} {}\n",
            i + 1,
            result.kind,
            status,
            truncate_text(&result.summary, 60),
            if !result.ok {
                format!(
                    " (Error: {})",
                    truncate_text(&result.outcome_reason.as_deref().unwrap_or("failed"), 30)
                )
            } else {
                String::new()
            }
        ));
    }
    prompt.push('\n');

    prompt.push_str("**Workspace Context:**\n");
    prompt.push_str(ws.trim());
    prompt.push_str("\n\n");
    prompt.push_str(ws_brief.trim());
    prompt.push_str("\n\n");

    prompt.push_str("**YOUR TASK:**\n");
    prompt.push_str("1. Acknowledge the original goal\n");
    prompt.push_str("2. Identify what went off-track\n");
    prompt.push_str("3. Create a NEW focused program that:\n");
    prompt.push_str("   - Directly addresses the original goal\n");
    prompt.push_str("   - Avoids the tangents that caused drift\n");
    prompt.push_str("   - Uses the minimum steps necessary\n");
    prompt.push_str("   - Has clear success criteria\n\n");

    prompt.push_str("Output ONLY valid Program JSON.\n");

    prompt
}

// ============================================================================
// Helper Functions
// ============================================================================

fn truncate_objective(obj: &str, max_len: usize) -> String {
    if obj.chars().count() <= max_len {
        obj.to_string()
    } else {
        format!("{}...", obj.chars().take(max_len).collect::<String>())
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(max_len).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_goal_drift_no_drift() {
        let objective = "Delete unused log files";
        let program = Program {
            objective: objective.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: "find . -name '*.log' -mtime +30".to_string(),
                    common: StepCommon::default(),
                },
                Step::Shell {
                    id: "s2".to_string(),
                    cmd: "find . -name '*.log' -mtime +30 -delete".to_string(),
                    common: StepCommon::default(),
                },
            ],
        };
        let results = vec![];

        let verdict = check_goal_drift(objective, &program, &results);
        assert!(!verdict.drift_detected);
    }

    #[test]
    fn test_check_goal_drift_action_mismatch() {
        let objective = "Delete unused files";
        let program = Program {
            objective: objective.to_string(),
            steps: vec![
                Step::Read {
                    id: "r1".to_string(),
                    path: "README.md".to_string(),
                    common: StepCommon::default(),
                },
                Step::Read {
                    id: "r2".to_string(),
                    path: "Cargo.toml".to_string(),
                    common: StepCommon::default(),
                },
                Step::Read {
                    id: "r3".to_string(),
                    path: "src/main.rs".to_string(),
                    common: StepCommon::default(),
                },
            ],
        };
        let results = vec![];

        let verdict = check_goal_drift(objective, &program, &results);
        assert!(verdict.drift_detected);
        assert!(verdict.reason.as_ref().unwrap().contains("read-only"));
    }

    #[test]
    fn test_check_goal_drift_meta_planning() {
        let objective = "Fix the bug";
        let program = Program {
            objective: objective.to_string(),
            steps: vec![
                Step::Plan {
                    id: "p1".to_string(),
                    goal: "Plan the fix".to_string(),
                    common: StepCommon::default(),
                },
                Step::Plan {
                    id: "p2".to_string(),
                    goal: "Plan the planning".to_string(),
                    common: StepCommon::default(),
                },
                Step::Plan {
                    id: "p3".to_string(),
                    goal: "Plan the plan planning".to_string(),
                    common: StepCommon::default(),
                },
            ],
        };
        let results = vec![];

        let verdict = check_goal_drift(objective, &program, &results);
        assert!(verdict.drift_detected);
        assert!(verdict.reason.as_ref().unwrap().contains("meta-planning"));
    }

    #[test]
    fn test_check_goal_drift_allows_masterplan_plus_phase_plan() {
        let objective = "Implement phase 1";
        let program = Program {
            objective: objective.to_string(),
            steps: vec![
                Step::MasterPlan {
                    id: "m1".to_string(),
                    goal: "Strategic roadmap".to_string(),
                    common: StepCommon {
                        purpose: "roadmap".to_string(),
                        success_condition: "roadmap saved".to_string(),
                        ..StepCommon::default()
                    },
                },
                Step::Plan {
                    id: "p1".to_string(),
                    goal: "Phase 1 implementation".to_string(),
                    common: StepCommon {
                        purpose: "phase plan".to_string(),
                        depends_on: vec!["m1".to_string()],
                        success_condition: "phase plan saved".to_string(),
                        ..StepCommon::default()
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "answer".to_string(),
                    common: StepCommon {
                        purpose: "answer".to_string(),
                        depends_on: vec!["p1".to_string()],
                        success_condition: "answer sent".to_string(),
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let verdict = check_goal_drift(objective, &program, &[]);
        assert!(!verdict.drift_detected);
    }

    #[test]
    fn test_check_goal_drift_no_progress() {
        let objective = "Add feature X";
        let program = Program {
            objective: objective.to_string(),
            steps: vec![Step::Read {
                id: "r1".to_string(),
                path: "file.txt".to_string(),
                common: StepCommon::default(),
            }],
        };
        // 5+ steps executed but all are read-only (no modifications)
        let results = vec![
            StepResult {
                id: "r1".to_string(),
                kind: "read".to_string(),
                purpose: "Read file".to_string(),
                depends_on: vec![],
                success_condition: "File read".to_string(),
                ok: true,
                summary: "File content".to_string(),
                command: None,
                raw_output: Some("content".to_string()),
                exit_code: None,
                output_bytes: Some(100),
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: None,
                outcome_status: None,
                outcome_reason: None,
            },
            StepResult {
                id: "r2".to_string(),
                kind: "read".to_string(),
                purpose: "Read another file".to_string(),
                depends_on: vec![],
                success_condition: "File read".to_string(),
                ok: true,
                summary: "More content".to_string(),
                command: None,
                raw_output: Some("content".to_string()),
                exit_code: None,
                output_bytes: Some(100),
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: None,
                outcome_status: None,
                outcome_reason: None,
            },
            StepResult {
                id: "r3".to_string(),
                kind: "search".to_string(),
                purpose: "Search for something".to_string(),
                depends_on: vec![],
                success_condition: "Search done".to_string(),
                ok: true,
                summary: "Search results".to_string(),
                command: None,
                raw_output: Some("results".to_string()),
                exit_code: None,
                output_bytes: Some(100),
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: None,
                outcome_status: None,
                outcome_reason: None,
            },
            StepResult {
                id: "r4".to_string(),
                kind: "read".to_string(),
                purpose: "Read yet another".to_string(),
                depends_on: vec![],
                success_condition: "File read".to_string(),
                ok: true,
                summary: "Content".to_string(),
                command: None,
                raw_output: Some("content".to_string()),
                exit_code: None,
                output_bytes: Some(100),
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: None,
                outcome_status: None,
                outcome_reason: None,
            },
            StepResult {
                id: "r5".to_string(),
                kind: "read".to_string(),
                purpose: "Read one more".to_string(),
                depends_on: vec![],
                success_condition: "File read".to_string(),
                ok: true,
                summary: "More".to_string(),
                command: None,
                raw_output: Some("content".to_string()),
                exit_code: None,
                output_bytes: Some(100),
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: None,
                outcome_status: None,
                outcome_reason: None,
            },
        ];

        let verdict = check_goal_drift(objective, &program, &results);
        assert!(verdict.drift_detected);
        assert!(verdict
            .reason
            .as_ref()
            .unwrap()
            .contains("0 successful modifications"));
    }

    #[test]
    fn test_check_goal_drift_direct_shell_success_is_not_drift() {
        let objective = "ls -ltr";
        let program = Program {
            objective: objective.to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: "ls -ltr".to_string(),
                    common: StepCommon {
                        purpose: "execute the requested shell command directly".to_string(),
                        ..StepCommon::default()
                    },
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report the command result clearly.".to_string(),
                    common: StepCommon {
                        purpose: "present the shell result to the user".to_string(),
                        depends_on: vec!["s1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };
        let results = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            purpose: "execute the requested shell command directly".to_string(),
            depends_on: vec![],
            success_condition: "the requested command completes".to_string(),
            ok: true,
            summary: "Directory listing completed".to_string(),
            command: Some("ls -ltr".to_string()),
            raw_output: Some("total 0".to_string()),
            exit_code: Some(0),
            output_bytes: Some(8),
            truncated: false,
            timed_out: false,
            artifact_path: None,
            artifact_kind: None,
            outcome_status: Some("ok".to_string()),
            outcome_reason: None,
        }];

        let verdict = check_goal_drift(objective, &program, &results);
        assert!(!verdict.drift_detected);
    }

    #[test]
    fn test_truncate_helpers_handle_unicode() {
        let objective = "─".repeat(80);
        let text = "╭".repeat(90);

        let truncated_objective = truncate_objective(&objective, 50);
        let truncated_text = truncate_text(&text, 40);

        assert!(truncated_objective.ends_with("..."));
        assert!(truncated_text.ends_with("..."));
    }
}
