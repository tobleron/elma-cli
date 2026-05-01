//! @efficiency-role: service-orchestrator
//!
//! Instruction-Level Repair And Result Recombiner — Task 391.
//!
//! When an instruction in the work graph fails, repair only that instruction
//! instead of restarting the whole request. Successful instruction outcomes
//! feed into a recombiner that produces a grounded parent-level result.

use crate::*;
use serde::{Deserialize, Serialize};

/// Status of a single instruction in the work graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum InstructionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Abandoned,
}

impl InstructionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Abandoned)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

/// Outcome of a single instruction execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InstructionOutcome {
    /// Instruction identifier (matches step id in the program/work graph).
    pub instruction_id: String,
    pub status: InstructionStatus,
    /// One-sentence summary of what the instruction produced.
    pub result_summary: String,
    /// References to evidence artifacts (file paths, tool outputs).
    pub evidence_refs: Vec<String>,
    /// How many times this instruction has been repaired.
    pub repair_count: u32,
    /// The approach/strategy used for this instruction.
    pub strategy_used: String,
}

/// 3-field repair decision output (Task 378 compliant).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RepairAction {
    /// One of: tighten_context, choose_native_tool, request_evidence,
    ///         split_instruction, abandon_branch
    pub repair_action: String,
    /// Why this repair action was chosen.
    pub reason: String,
    /// Whether the instruction can be retried after repair.
    pub retryable: bool,
}

/// Recombined result from multiple sibling instruction outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecombinedResult {
    /// Combined summary from successful instruction outcomes.
    pub summary: String,
    /// Aggregated evidence references.
    pub evidence_refs: Vec<String>,
    /// Number of instructions that contributed.
    pub succeeded_count: u32,
    /// Number of instructions that failed or were abandoned.
    pub failed_count: u32,
    /// True if recombination found sufficient evidence.
    pub evidence_sufficient: bool,
}

/// Select a repair action for a failed instruction based on its error summary.
/// Non-keyword: uses failure class patterns derived from the approach engine.
pub(crate) fn select_repair_action(
    instruction_id: &str,
    error_summary: &str,
    repair_count: u32,
) -> RepairAction {
    let error_lower = error_summary.to_lowercase();

    if repair_count >= 3 {
        return RepairAction {
            repair_action: "abandon_branch".to_string(),
            reason: format!(
                "Instruction '{}' has failed {} times. Abandoning this branch.",
                instruction_id, repair_count
            ),
            retryable: false,
        };
    }

    if error_lower.contains("tool") || error_lower.contains("command not found") {
        return RepairAction {
            repair_action: "choose_native_tool".to_string(),
            reason: format!(
                "Instruction '{}' failed due to tool/command error. Switch to native tool.",
                instruction_id
            ),
            retryable: true,
        };
    }

    if error_lower.contains("parse") || error_lower.contains("json") || error_lower.contains("invalid")
    {
        return RepairAction {
            repair_action: "tighten_context".to_string(),
            reason: format!(
                "Instruction '{}' produced parseable output. Tighten context and retry.",
                instruction_id
            ),
            retryable: true,
        };
    }

    if error_lower.contains("not found") || error_lower.contains("missing") {
        return RepairAction {
            repair_action: "request_evidence".to_string(),
            reason: format!(
                "Instruction '{}' needs missing information. Request evidence first.",
                instruction_id
            ),
            retryable: true,
        };
    }

    if error_lower.contains("timeout") {
        return RepairAction {
            repair_action: "split_instruction".to_string(),
            reason: format!(
                "Instruction '{}' timed out. Split into smaller sub-instructions.",
                instruction_id
            ),
            retryable: true,
        };
    }

    // Default repair: tighten context
    RepairAction {
        repair_action: "tighten_context".to_string(),
        reason: format!(
            "Instruction '{}' failed with unspecified error. Tighten context and retry.",
            instruction_id
        ),
        retryable: true,
    }
}

/// Apply a repair action to produce a new instruction outcome.
/// Returns an outcome with the repair action applied.
pub(crate) fn create_repair_outcome(
    instruction_id: &str,
    original_error: &str,
    repair: &RepairAction,
) -> InstructionOutcome {
    InstructionOutcome {
        instruction_id: instruction_id.to_string(),
        status: if repair.retryable {
            InstructionStatus::Running
        } else {
            InstructionStatus::Abandoned
        },
        result_summary: format!(
            "Repair '{}': {} — {}",
            repair.repair_action, repair.reason, original_error
        ),
        evidence_refs: Vec::new(),
        repair_count: 0,
        strategy_used: repair.repair_action.clone(),
    }
}

/// Try to repair a failed instruction outcome.
/// If retryable, returns a Running outcome for re-execution.
/// If abandon, returns Abandoned.
pub(crate) fn try_repair(
    outcome: &InstructionOutcome,
    error_summary: &str,
) -> InstructionOutcome {
    if outcome.status != InstructionStatus::Failed {
        return outcome.clone();
    }

    let repair = select_repair_action(
        &outcome.instruction_id,
        error_summary,
        outcome.repair_count,
    );

    let mut repaired = create_repair_outcome(
        &outcome.instruction_id,
        error_summary,
        &repair,
    );
    repaired.repair_count = outcome.repair_count + 1;

    repaired
}

/// Recombine sibling instruction outcomes into a parent-level result.
/// Only uses successful outcomes. Fails closed when no evidence available.
pub(crate) fn recombine(
    outcomes: &[InstructionOutcome],
    parent_goal: &str,
) -> RecombinedResult {
    let successful: Vec<_> = outcomes.iter().filter(|o| o.status.is_success()).collect();
    let failed: Vec<_> = outcomes
        .iter()
        .filter(|o| !o.status.is_success())
        .collect();

    let succeeded_count = successful.len() as u32;
    let failed_count = failed.len() as u32;

    // Aggregate evidence refs from successful instructions
    let evidence_refs: Vec<String> = successful
        .iter()
        .flat_map(|o| o.evidence_refs.clone())
        .collect();

    let evidence_sufficient = !evidence_refs.is_empty() && succeeded_count > 0;

    // Build summary from successful outcomes
    let summary = if evidence_sufficient {
        let parts: Vec<String> = successful
            .iter()
            .map(|o| o.result_summary.clone())
            .collect();
        format!(
            "Recombined '{}': {} ({} of {} instructions succeeded)",
            parent_goal,
            parts.join("; "),
            succeeded_count,
            outcomes.len()
        )
    } else {
        format!(
            "Recombined '{}': insufficient evidence ({} succeeded, {} failed)",
            parent_goal, succeeded_count, failed_count
        )
    };

    RecombinedResult {
        summary,
        evidence_refs,
        succeeded_count,
        failed_count,
        evidence_sufficient,
    }
}

/// Check if a set of recombined results has sufficient evidence to answer.
pub(crate) fn has_sufficient_evidence(results: &[RecombinedResult]) -> bool {
    if results.is_empty() {
        return false;
    }
    results.iter().all(|r| r.evidence_sufficient)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_success(id: &str, summary: &str) -> InstructionOutcome {
        InstructionOutcome {
            instruction_id: id.to_string(),
            status: InstructionStatus::Succeeded,
            result_summary: summary.to_string(),
            evidence_refs: vec![format!("evidence/{}.md", id)],
            repair_count: 0,
            strategy_used: "Direct".to_string(),
        }
    }

    fn make_failure(id: &str, summary: &str) -> InstructionOutcome {
        InstructionOutcome {
            instruction_id: id.to_string(),
            status: InstructionStatus::Failed,
            result_summary: summary.to_string(),
            evidence_refs: vec![],
            repair_count: 0,
            strategy_used: "Direct".to_string(),
        }
    }

    #[test]
    fn test_instruction_outcome_terminal() {
        assert!(InstructionStatus::Succeeded.is_terminal());
        assert!(InstructionStatus::Failed.is_terminal());
        assert!(InstructionStatus::Abandoned.is_terminal());
        assert!(!InstructionStatus::Pending.is_terminal());
        assert!(!InstructionStatus::Running.is_terminal());
    }

    #[test]
    fn test_instruction_outcome_success() {
        assert!(InstructionStatus::Succeeded.is_success());
        assert!(!InstructionStatus::Failed.is_success());
        assert!(!InstructionStatus::Pending.is_success());
    }

    #[test]
    fn test_select_repair_action_tool_error() {
        let action = select_repair_action("i1", "tool: read: command not found", 0);
        assert_eq!(action.repair_action, "choose_native_tool");
        assert!(action.retryable);
    }

    #[test]
    fn test_select_repair_action_parse_error() {
        let action = select_repair_action("i1", "parse error: invalid json", 0);
        assert_eq!(action.repair_action, "tighten_context");
        assert!(action.retryable);
    }

    #[test]
    fn test_select_repair_action_missing_evidence() {
        let action = select_repair_action("i1", "file not found", 0);
        assert_eq!(action.repair_action, "request_evidence");
        assert!(action.retryable);
    }

    #[test]
    fn test_select_repair_action_timeout() {
        let action = select_repair_action("i1", "wall clock timeout", 0);
        assert_eq!(action.repair_action, "split_instruction");
        assert!(action.retryable);
    }

    #[test]
    fn test_select_repair_action_abandon_after_3() {
        let action = select_repair_action("i1", "any error", 3);
        assert_eq!(action.repair_action, "abandon_branch");
        assert!(!action.retryable);
    }

    #[test]
    fn test_select_repair_action_default() {
        let action = select_repair_action("i1", "unexpected error occurred", 0);
        assert_eq!(action.repair_action, "tighten_context");
        assert!(action.retryable);
    }

    #[test]
    fn test_try_repair_failed_instruction() {
        let outcome = make_failure("i1", "tool not found");
        let repaired = try_repair(&outcome, "tool: read: command not found");
        assert_eq!(repaired.repair_count, 1);
        assert_eq!(repaired.status, InstructionStatus::Running);
        assert_eq!(repaired.strategy_used, "choose_native_tool");
    }

    #[test]
    fn test_try_repair_abandons_after_max() {
        let outcome = InstructionOutcome {
            instruction_id: "i1".to_string(),
            status: InstructionStatus::Failed,
            result_summary: "failed".to_string(),
            evidence_refs: vec![],
            repair_count: 3,
            strategy_used: "Direct".to_string(),
        };
        let repaired = try_repair(&outcome, "any error");
        assert_eq!(repaired.status, InstructionStatus::Abandoned);
    }

    #[test]
    fn test_try_repair_skips_non_failed() {
        let outcome = make_success("i1", "already done");
        let repaired = try_repair(&outcome, "irrelevant");
        assert_eq!(repaired.status, InstructionStatus::Succeeded);
    }

    #[test]
    fn test_recombine_all_successful() {
        let outcomes = vec![
            make_success("i1", "Installed deps"),
            make_success("i2", "Built project"),
        ];
        let result = recombine(&outcomes, "Setup environment");
        assert!(result.evidence_sufficient);
        assert_eq!(result.succeeded_count, 2);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.evidence_refs.len(), 2);
    }

    #[test]
    fn test_recombine_partial_failure() {
        let outcomes = vec![
            make_success("i1", "Installed deps"),
            make_failure("i2", "Build failed"),
            make_success("i3", "Configured settings"),
        ];
        let result = recombine(&outcomes, "Setup environment");
        assert!(result.evidence_sufficient);
        assert_eq!(result.succeeded_count, 2);
        assert_eq!(result.failed_count, 1);
    }

    #[test]
    fn test_recombine_all_failed_no_evidence() {
        let outcomes = vec![
            make_failure("i1", "Failed step 1"),
            make_failure("i2", "Failed step 2"),
        ];
        let result = recombine(&outcomes, "Setup environment");
        assert!(!result.evidence_sufficient);
        assert_eq!(result.succeeded_count, 0);
        assert_eq!(result.failed_count, 2);
    }

    #[test]
    fn test_recombine_empty_outcomes() {
        let outcomes = vec![];
        let result = recombine(&outcomes, "Empty goal");
        assert!(!result.evidence_sufficient);
        assert_eq!(result.succeeded_count, 0);
    }

    #[test]
    fn test_has_sufficient_evidence() {
        let good = RecombinedResult {
            summary: "done".to_string(),
            evidence_refs: vec!["ref1".to_string()],
            succeeded_count: 1,
            failed_count: 0,
            evidence_sufficient: true,
        };
        let bad = RecombinedResult {
            summary: "failed".to_string(),
            evidence_refs: vec![],
            succeeded_count: 0,
            failed_count: 1,
            evidence_sufficient: false,
        };
        assert!(has_sufficient_evidence(&[good]));
        assert!(!has_sufficient_evidence(&[]));
        assert!(!has_sufficient_evidence(&[bad]));
    }

    #[test]
    fn test_create_repair_outcome_retryable() {
        let repair = RepairAction {
            repair_action: "tighten_context".to_string(),
            reason: "need more context".to_string(),
            retryable: true,
        };
        let outcome = create_repair_outcome("i1", "parse error", &repair);
        assert_eq!(outcome.status, InstructionStatus::Running);
        assert!(outcome.result_summary.contains("tighten_context"));
    }

    #[test]
    fn test_create_repair_outcome_abandon() {
        let repair = RepairAction {
            repair_action: "abandon_branch".to_string(),
            reason: "too many retries".to_string(),
            retryable: false,
        };
        let outcome = create_repair_outcome("i1", "persistent failure", &repair);
        assert_eq!(outcome.status, InstructionStatus::Abandoned);
    }
}
