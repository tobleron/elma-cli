//! @efficiency-role: util-pure
//!
//! Execution Ladder Depth Module
//!
//! Provides depth conversion, confidence calculation, request predicate functions,
//! and compatibility wrappers for the execution ladder system.

use crate::intel_trait::*;
use crate::ComplexityAssessment;
use crate::*;

use super::{ExecutionLadderAssessment, ExecutionLevel};

// ============================================================================
// Depth Conversion Functions
// ============================================================================

/// Convert assessment to legacy depth (compatibility wrapper)
pub fn assessment_to_depth(assessment: &ExecutionLadderAssessment) -> u8 {
    match assessment.level {
        ExecutionLevel::Action => 1,
        ExecutionLevel::Task => 2,
        ExecutionLevel::Plan => 3,
        ExecutionLevel::MasterPlan => 4,
    }
}

/// Convert legacy depth to level (compatibility wrapper)
pub fn depth_to_level(depth: u8) -> ExecutionLevel {
    match depth {
        0 | 1 => ExecutionLevel::Action,
        2 => ExecutionLevel::Task,
        3 => ExecutionLevel::Plan,
        _ => ExecutionLevel::MasterPlan,
    }
}

// ============================================================================
// Confidence Calculation
// ============================================================================

/// Calculate overall confidence from unit outputs
pub fn calculate_confidence(
    complexity: &IntelOutput,
    evidence: &IntelOutput,
    action: &IntelOutput,
    workflow: &IntelOutput,
) -> f64 {
    // Average confidence from all units
    let confidences = [
        complexity.confidence,
        evidence.confidence,
        action.confidence,
        workflow.confidence,
    ];

    let avg = confidences.iter().sum::<f64>() / confidences.len() as f64;

    // Reduce confidence if any unit used fallback
    let fallback_penalty = [
        complexity.fallback_used,
        evidence.fallback_used,
        action.fallback_used,
        workflow.fallback_used,
    ]
    .iter()
    .filter(|&&x| x)
    .count() as f64
        * 0.1;

    (avg - fallback_penalty).max(0.3).min(1.0)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Truncate message for display in reason
pub fn truncate_message(msg: &str) -> String {
    let truncated = msg.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
    if msg.len() > truncated.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

// ============================================================================
// Compatibility Functions
// ============================================================================

/// Check if hierarchical decomposition is needed (compatibility wrapper)
pub fn assessment_needs_decomposition(assessment: &ExecutionLadderAssessment) -> bool {
    matches!(
        assessment.level,
        ExecutionLevel::Plan | ExecutionLevel::MasterPlan
    )
}

/// Escalate execution level when model is struggling (Task 306)
pub fn escalate_on_weakness(current_level: ExecutionLevel) -> ExecutionLevel {
    match current_level {
        ExecutionLevel::Action => ExecutionLevel::Task,
        ExecutionLevel::Task => ExecutionLevel::Plan,
        ExecutionLevel::Plan => ExecutionLevel::MasterPlan,
        ExecutionLevel::MasterPlan => ExecutionLevel::MasterPlan, // Max level
    }
}

// ============================================================================
// Request Predicate Functions
// ============================================================================

/// Check if request explicitly asks for planning
pub fn requests_planning(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();

    // Principle: Look for planning SEMANTICS, not just keywords
    // Sequential language, decomposition language, planning language

    let planning_indicators = [
        "step-by-step",
        "step by step",
        "give me a plan",
        "create a plan",
        "break down",
        "breakdown",
        "detailed plan",
        "implementation plan",
        "how would you approach",
        "what steps",
        "ordered steps",
    ];

    planning_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Check if request implies strategic decomposition
pub fn requests_strategy(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();

    // Principle: Strategic = multi-phase, multi-session, or architectural

    let strategy_indicators = [
        "migration strategy",
        "architecture redesign",
        "phased approach",
        "long-term plan",
        "overall strategy",
        "master plan",
        "masterplan",
        "strategic overview",
        "roadmap",
        "multi-phase",
        "multi-session",
    ];

    strategy_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Check if request asks for phased decomposition
pub fn requests_phases(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();

    let phase_indicators = [
        "phases",
        "phase",
        "milestone",
        "stages",
        "stage",
        "rollout",
        "deployment plan",
        "staged approach",
    ];

    phase_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Check if request implies bulk processing
pub fn requests_bulk(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();

    let bulk_indicators = [
        "all files",
        "every file",
        "bulk ",
        "mass ",
        "multiple files",
        "several files",
        "whole repo",
        "entire workspace",
        "all documents",
    ];

    bulk_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Check if request implies multiple sequential verbs
pub fn requests_multi_step_verbs(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();

    let sequence_indicators = [
        " then ",
        " after ",
        " before ",
        " followed by ",
        " and then ",
        " once ",
        " as soon as ",
    ];

    sequence_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Check if request has dependencies requiring ordering
pub fn has_dependencies(user_message: &str, _workspace_brief: &str) -> bool {
    let lower = user_message.to_lowercase();

    // Look for dependency language
    let dependency_indicators = [
        "first x then y",
        "before doing",
        "after completing",
        "dependencies",
        "prerequisite",
        "must complete",
        "implement feature",
        "refactor",
        "clean up",
    ];

    dependency_indicators
        .iter()
        .any(|indicator| lower.contains(indicator))
}

/// Check if request needs revision loop
pub fn needs_revision_loop(user_message: &str, complexity: &ComplexityAssessment) -> bool {
    let lower = user_message.to_lowercase();

    // Revision indicators
    let revision_indicators = [
        "fix",
        "debug",
        "troubleshoot",
        "refactor",
        "iterate",
        "keep trying",
        "refine",
        "adjust",
        "verify after",
    ];

    let has_revision_language = revision_indicators
        .iter()
        .any(|indicator| lower.contains(indicator));

    // Edit operations often need revision
    let is_edit_heavy = lower.contains("edit")
        || lower.contains("modify")
        || lower.contains("update")
        || lower.contains("change");

    has_revision_language || (is_edit_heavy && complexity.complexity != "DIRECT")
}

/// Generate human-readable reason for level choice
pub fn generate_level_reason(
    level: ExecutionLevel,
    user_message: &str,
    escalation_factors: &[&str],
) -> String {
    let truncated = truncate_message(user_message);

    let base_reason = match level {
        ExecutionLevel::Action => format!("Direct execution: '{}'", truncated),
        ExecutionLevel::Task => {
            format!("Bounded outcome requiring evidence chain: '{}'", truncated)
        }
        ExecutionLevel::Plan => format!("Tactical breakdown required: '{}'", truncated),
        ExecutionLevel::MasterPlan => format!("Strategic decomposition required: '{}'", truncated),
    };

    if escalation_factors.is_empty() {
        base_reason
    } else {
        format!(
            "{} (escalated: {})",
            base_reason,
            escalation_factors.join(", ")
        )
    }
}

/// Generate optional strategy hint for formula selection/planning
pub fn generate_strategy_hint(
    level: ExecutionLevel,
    requires_evidence: bool,
    requires_ordering: bool,
) -> Option<String> {
    match (level, requires_evidence, requires_ordering) {
        (ExecutionLevel::Action, false, false) => {
            None // No hint needed for simple action
        }
        (ExecutionLevel::Task, true, false) => Some("gather evidence before execution".to_string()),
        (ExecutionLevel::Task, true, true) => {
            Some("gather evidence, then execute in order".to_string())
        }
        (ExecutionLevel::Plan, _, _) => Some("explicit planning structure required".to_string()),
        (ExecutionLevel::MasterPlan, _, _) => Some("phased strategic decomposition".to_string()),
        _ => None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_conversion_roundtrip() {
        for depth in 1..=4 {
            let level = depth_to_level(depth);
            let converted_back = assessment_to_depth(&ExecutionLadderAssessment {
                level,
                reason: "test".to_string(),
                requires_evidence: false,
                requires_ordering: false,
                requires_phases: false,
                requires_revision_loop: false,
                risk: "LOW".to_string(),
                complexity: "DIRECT".to_string(),
                strategy_hint: None,
                fallback_used: false,
                confidence: 0.9,
            });
            assert_eq!(converted_back, depth);
        }
    }

    #[test]
    fn test_truncate_message() {
        assert_eq!(truncate_message("hello world"), "hello world");
        assert_eq!(
            truncate_message("one two three four five six seven"),
            "one two three four five..."
        );
        assert_eq!(truncate_message(""), "");
    }

    #[test]
    fn test_assessment_needs_decomposition() {
        let action = ExecutionLadderAssessment {
            level: ExecutionLevel::Action,
            reason: "test".to_string(),
            requires_evidence: false,
            requires_ordering: false,
            requires_phases: false,
            requires_revision_loop: false,
            risk: "LOW".to_string(),
            complexity: "DIRECT".to_string(),
            strategy_hint: None,
            fallback_used: false,
            confidence: 0.9,
        };
        assert!(!assessment_needs_decomposition(&action));

        let plan = ExecutionLadderAssessment {
            level: ExecutionLevel::Plan,
            ..action.clone()
        };
        assert!(assessment_needs_decomposition(&plan));

        let master = ExecutionLadderAssessment {
            level: ExecutionLevel::MasterPlan,
            ..action
        };
        assert!(assessment_needs_decomposition(&master));
    }

    #[test]
    fn test_requests_planning() {
        assert!(requests_planning("give me a step-by-step plan"));
        assert!(requests_planning("create a plan to refactor"));
        assert!(requests_planning("break down this task"));
        assert!(!requests_planning("run cargo test"));
        assert!(!requests_planning("read this file"));
    }

    #[test]
    fn test_requests_strategy() {
        assert!(requests_strategy("design a migration strategy"));
        assert!(requests_strategy("create a masterplan"));
        assert!(requests_strategy("phased approach for redesign"));
        assert!(!requests_strategy("run tests"));
        assert!(!requests_strategy("read file"));
    }

    #[test]
    fn test_generate_level_reason() {
        let reason = generate_level_reason(
            ExecutionLevel::Action,
            "hello world this is a test message",
            &[],
        );
        assert!(reason.contains("Direct execution"));

        let reason_escalated = generate_level_reason(
            ExecutionLevel::Task,
            "hello world this is a test message",
            &["high risk"],
        );
        assert!(reason_escalated.contains("escalated"));
        assert!(reason_escalated.contains("high risk"));
    }

    #[test]
    fn test_generate_strategy_hint() {
        assert_eq!(
            generate_strategy_hint(ExecutionLevel::Action, false, false),
            None
        );
        assert_eq!(
            generate_strategy_hint(ExecutionLevel::Task, true, false),
            Some("gather evidence before execution".to_string())
        );
        assert_eq!(
            generate_strategy_hint(ExecutionLevel::Plan, false, false),
            Some("explicit planning structure required".to_string())
        );
    }
}
