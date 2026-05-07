//! @efficiency-role: domain-logic
//!
//! Finalization Hardener — Task 766.
//!
//! Strengthens finalization verification by consuming CurrentTurnContext,
//! ObjectiveState, DeliverableContract, and ScopeCoverageLedger.
//! Rejects answers that mention artifacts not requested in the current turn,
//! rejects completion language when objectives remain unresolved, and
//! labels iteration-limit answers as partial unless objective state proves
//! completion.

use crate::*;

/// Verdict from the finalization hardener.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HardenerVerdict {
    /// Answer passes all checks.
    Pass,
    /// Answer mentions stale artifacts from prior turns.
    StaleArtifactDetected { mentioned: Vec<String>, reason: String },
    /// Answer claims completion but objectives are unresolved.
    UnresolvedObjective { reason: String },
    /// Iteration-limit answer that should be labeled partial.
    PartialCompletion { reason: String },
}

/// Check whether a final answer references artifacts that were not requested
/// in the current turn's deliverable contract.
pub(crate) fn check_stale_artifact_references(
    final_answer: &str,
    current_deliverables: &[String],
) -> HardenerVerdict {
    let lower = final_answer.to_lowercase();
    let mut stale_mentions: Vec<String> = Vec::new();

    // First pass: detect if this is a creation-oriented answer at all
    let is_creation_answer = lower.contains("created or updated")
        || lower.contains("created")
        || lower.contains("wrote")
        || lower.contains("saved to");

    if !is_creation_answer {
        return HardenerVerdict::Pass;
    }

    // Scan for path-like references in any line following creation context
    for line in final_answer.lines() {
        let line_lower = line.to_lowercase();

        for word in line.split_whitespace() {
            let clean = word
                .trim_start_matches('`')
                .trim_end_matches('`')
                .trim_end_matches(|c: char| c == '.' || c == ',' || c == ')' || c == ']');
            if (clean.contains('/') || clean.contains('.'))
                && clean.len() > 3
                && clean.len() < 200
            {
                // Check if this path is in the current-turn deliverable contract
                let is_current = current_deliverables.iter().any(|d| clean.contains(d.as_str()) || d.contains(clean));
                if !is_current {
                    stale_mentions.push(clean.to_string());
                }
            }
        }
    }

    if !stale_mentions.is_empty() {
        HardenerVerdict::StaleArtifactDetected {
            mentioned: stale_mentions.clone(),
            reason: format!(
                "Final answer references artifacts not requested this turn: {}",
                stale_mentions.join(", ")
            ),
        }
    } else {
        HardenerVerdict::Pass
    }
}

/// Check whether a final answer claims completion when objectives are unresolved.
pub(crate) fn check_unresolved_objective(
    final_answer: &str,
    objective_has_unresolved: bool,
    stop_reason_is_budget: bool,
) -> HardenerVerdict {
    let lower = final_answer.to_lowercase();
    let claims_completion = lower.contains("completed")
        || lower.contains("all done")
        || lower.contains("finished")
        || lower.contains("created or updated");

    // Budget exhaustion + unresolved objectives + completion claim = partial completion
    if stop_reason_is_budget && objective_has_unresolved && claims_completion {
        return HardenerVerdict::PartialCompletion {
            reason: "Iteration/budget limit reached with unresolved objectives — answer should be labeled partial"
                .to_string(),
        };
    }

    if objective_has_unresolved && claims_completion {
        return HardenerVerdict::UnresolvedObjective {
            reason: "Final answer claims completion but objective state has unresolved requirements"
                .to_string(),
        };
    }

    HardenerVerdict::Pass
}

/// Full hardener check combining all verification sources.
pub(crate) fn harden_finalization(
    final_answer: &str,
    current_deliverables: Option<&[String]>,
    objective_has_unresolved: bool,
    stop_reason_is_budget: bool,
) -> Vec<HardenerVerdict> {
    let mut verdicts = Vec::new();

    if let Some(deliverables) = current_deliverables {
        let stale_check = check_stale_artifact_references(final_answer, deliverables);
        if stale_check != HardenerVerdict::Pass {
            verdicts.push(stale_check);
        }
    }

    let objective_check = check_unresolved_objective(
        final_answer,
        objective_has_unresolved,
        stop_reason_is_budget,
    );
    if objective_check != HardenerVerdict::Pass {
        verdicts.push(objective_check);
    }

    verdicts
}

/// Build an appendix to append to the final answer if hardener finds issues.
pub(crate) fn build_hardener_appendix(verdicts: &[HardenerVerdict]) -> Option<String> {
    if verdicts.is_empty() {
        return None;
    }

    let mut notes: Vec<String> = Vec::new();
    for v in verdicts {
        match v {
            HardenerVerdict::StaleArtifactDetected { mentioned, .. } => {
                notes.push(format!(
                    "Stale artifact references detected: {}. These were not requested in the current turn.",
                    mentioned.join(", ")
                ));
            }
            HardenerVerdict::UnresolvedObjective { reason } => {
                notes.push(format!("Incomplete: {}", reason));
            }
            HardenerVerdict::PartialCompletion { reason } => {
                notes.push(format!("Partial completion: {}", reason));
            }
            HardenerVerdict::Pass => {}
        }
    }

    if notes.is_empty() {
        None
    } else {
        Some(format!(
            "\n\n---\n**Finalization Notes:**\n{}",
            notes.join("\n")
        ))
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_stale_artifacts() {
        let answer = "I found the AGENTS.md file in the workspace root.";
        let deliverables = vec!["AGENTS.md".to_string()];
        let verdict = check_stale_artifact_references(answer, &deliverables);
        assert_eq!(verdict, HardenerVerdict::Pass);
    }

    #[test]
    fn test_stale_artifact_detected() {
        let answer = "Completed the requested artifact work.\n\nCreated or updated:\n- `_testing_prompts/01_prompt.txt`\n- `_testing_prompts/06_prompt.txt`";
        let deliverables = vec!["current_report.md".to_string()];
        let verdict = check_stale_artifact_references(answer, &deliverables);
        assert!(
            matches!(verdict, HardenerVerdict::StaleArtifactDetected { .. }),
            "Should detect stale artifact references"
        );
    }

    #[test]
    fn test_unresolved_objective_detected() {
        let answer = "Completed all requested work successfully.";
        let verdict = check_unresolved_objective(answer, true, false);
        assert!(
            matches!(verdict, HardenerVerdict::UnresolvedObjective { .. }),
            "Should detect unresolved objective"
        );
    }

    #[test]
    fn test_unresolved_objective_clean_pass() {
        let answer = "I checked the files and here is what I found: ...";
        let verdict = check_unresolved_objective(answer, true, false);
        assert_eq!(verdict, HardenerVerdict::Pass);
    }

    #[test]
    fn test_partial_completion_detected() {
        let answer = "Completed all requested work.";
        let verdict = check_unresolved_objective(answer, true, true);
        assert!(
            matches!(verdict, HardenerVerdict::PartialCompletion { .. }),
            "Should detect partial completion"
        );
    }

    #[test]
    fn test_harden_finalization_all_clean() {
        let answer = "Found 3 files matching the pattern.";
        let verdicts = harden_finalization(answer, Some(&[]), false, false);
        assert!(verdicts.is_empty());
    }

    #[test]
    fn test_harden_finalization_stale_artifacts() {
        let answer = "Created or updated: `old_file.txt`";
        let deliverables = vec!["new_file.md".to_string()];
        let verdicts = harden_finalization(answer, Some(&deliverables), false, false);
        assert!(!verdicts.is_empty());
        assert!(matches!(
            verdicts[0],
            HardenerVerdict::StaleArtifactDetected { .. }
        ));
    }

    #[test]
    fn test_build_appendix_empty() {
        assert!(build_hardener_appendix(&[]).is_none());
    }

    #[test]
    fn test_build_appendix_with_verdicts() {
        let verdicts = vec![HardenerVerdict::StaleArtifactDetected {
            mentioned: vec!["stale.md".to_string()],
            reason: "test".to_string(),
        }];
        let appendix = build_hardener_appendix(&verdicts);
        assert!(appendix.is_some());
        assert!(appendix.unwrap().contains("Stale artifact"));
    }
}
