//! @efficiency-role: ignored
//! Tests for app_chat_orchestrator fast paths

use crate::app_chat_fast_paths::{
    should_use_direct_reply_fast_path, should_use_direct_shell_fast_path,
};
use crate::*;

fn test_probability_decision(choice: &str) -> ProbabilityDecision {
    ProbabilityDecision {
        choice: choice.to_string(),
        source: "test".to_string(),
        distribution: vec![(choice.to_string(), 1.0)],
        margin: 1.0,
        entropy: 0.0,
    }
}

fn test_route_decision(route: &str) -> RouteDecision {
    RouteDecision {
        route: route.to_string(),
        source: "test".to_string(),
        distribution: vec![(route.to_string(), 1.0)],
        margin: 1.0,
        entropy: 0.0,
        speech_act: test_probability_decision("INSTRUCT"),
        workflow: test_probability_decision("WORKFLOW"),
        mode: test_probability_decision("EXECUTE"),
        evidence_required: false,
    }
}

#[test]
fn direct_shell_fast_path_accepts_direct_workflow_plan() {
    let route = test_route_decision("SHELL");
    let workflow_plan = WorkflowPlannerOutput {
        complexity: "DIRECT".to_string(),
        risk: "LOW".to_string(),
        ..WorkflowPlannerOutput::default()
    };
    let complexity = ComplexityAssessment {
        complexity: "MULTISTEP".to_string(),
        risk: "LOW".to_string(),
        ..ComplexityAssessment::default()
    };

    assert!(should_use_direct_shell_fast_path(
        "git status --short",
        &route,
        Some(&workflow_plan),
        &complexity
    ));
}

#[test]
fn direct_shell_fast_path_rejects_natural_language_read_request() {
    let route = test_route_decision("SHELL");
    let workflow_plan = WorkflowPlannerOutput {
        complexity: "DIRECT".to_string(),
        risk: "LOW".to_string(),
        ..WorkflowPlannerOutput::default()
    };
    let complexity = ComplexityAssessment {
        complexity: "DIRECT".to_string(),
        risk: "LOW".to_string(),
        ..ComplexityAssessment::default()
    };

    assert!(!should_use_direct_shell_fast_path(
        "Read the README.md in _stress_testing/_opencode_for_testing/ and create a 3-bullet point executive summary.",
        &route,
        Some(&workflow_plan),
        &complexity
    ));
}

#[test]
fn direct_reply_fast_path_accepts_direct_reply_only_even_when_route_is_not_chat() {
    let route = test_route_decision("DECIDE");
    let complexity = ComplexityAssessment {
        complexity: "DIRECT".to_string(),
        needs_evidence: false,
        needs_tools: false,
        needs_decision: false,
        needs_plan: false,
        risk: "LOW".to_string(),
        suggested_pattern: "reply_only".to_string(),
    };
    let formula = FormulaSelection {
        primary: "reply_only".to_string(),
        alternatives: Vec::new(),
        reason: "test".to_string(),
        memory_id: String::new(),
    };

    assert!(should_use_direct_reply_fast_path(
        "hello",
        &route,
        &complexity,
        &formula
    ));
}
