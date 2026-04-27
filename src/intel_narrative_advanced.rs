//! @efficiency-role: util-pure
//!
//! Intel Narrative - Advanced Assessment Functions
//!
//! Builds narrative context for advanced assessment intel units:
//! - Domain difficulty
//! - Freshness requirements
//! - Assumption tracking
//! - Edge case evaluation

use crate::intel_narrative_utils::format_conversation_excerpt;
use crate::*;

/// Build domain difficulty classifier narrative.
pub(crate) fn build_domain_difficulty_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation_excerpt: &[ChatMessage],
) -> String {
    let excerpt = format_conversation_excerpt(conversation_excerpt, 12);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act_choice: {speech_act}
- workflow_choice: {workflow}
- mode_choice: {mode}

WORKSPACE FACTS:
{workspace_facts}

WORKSPACE BRIEF:
{workspace_brief}

CONVERSATION EXCERPT:
{excerpt}

DECISION NEEDED:
Classify the domain difficulty of this request:
- What domain expertise is required?
- Is this common knowledge or specialized?
- Are there sensitive areas (medical, legal, financial, security)?
- What knowledge level is needed (basic, intermediate, advanced)?"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        workflow = route_decision.workflow.choice,
        mode = route_decision.mode.choice,
        workspace_facts = workspace_facts.trim(),
        workspace_brief = workspace_brief.trim(),
        excerpt = excerpt,
    )
}

/// Build freshness requirement assessment narrative.
pub(crate) fn build_freshness_requirement_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation_excerpt: &[ChatMessage],
) -> String {
    let excerpt = format_conversation_excerpt(conversation_excerpt, 12);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act_choice: {speech_act}
- workflow_choice: {workflow}
- mode_choice: {mode}

WORKSPACE FACTS:
{workspace_facts}

WORKSPACE BRIEF:
{workspace_brief}

CONVERSATION EXCERPT:
{excerpt}

DECISION NEEDED:
Assess the freshness requirements for this request:
- Does the user need current, up-to-date information?
- Is there a risk of providing stale or outdated data?
- What sources would need to be current (APIs, news, documentation, standards)?
- How time-sensitive is this request?"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        workflow = route_decision.workflow.choice,
        mode = route_decision.mode.choice,
        workspace_facts = workspace_facts.trim(),
        workspace_brief = workspace_brief.trim(),
        excerpt = excerpt,
    )
}

/// Build assumption tracker narrative.
pub(crate) fn build_assumption_tracker_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation_excerpt: &[ChatMessage],
) -> String {
    let excerpt = format_conversation_excerpt(conversation_excerpt, 12);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act_choice: {speech_act}
- workflow_choice: {workflow}
- mode_choice: {mode}

WORKSPACE FACTS:
{workspace_facts}

WORKSPACE BRIEF:
{workspace_brief}

CONVERSATION EXCERPT:
{excerpt}

DECISION NEEDED:
Identify and track assumptions that must be made to answer this request:
- What assumptions are implicit in the user's question?
- What assumptions about the environment, tools, or context are required?
- Which assumptions are risky if wrong?
- Which assumptions can be verified vs. must be accepted?
- What would change if key assumptions were different?"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        workflow = route_decision.workflow.choice,
        mode = route_decision.mode.choice,
        workspace_facts = workspace_facts.trim(),
        workspace_brief = workspace_brief.trim(),
        excerpt = excerpt,
    )
}

/// Build edge case evaluator narrative.
pub(crate) fn build_edge_case_evaluator_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation_excerpt: &[ChatMessage],
) -> String {
    let excerpt = format_conversation_excerpt(conversation_excerpt, 12);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act_choice: {speech_act}
- workflow_choice: {workflow}
- mode_choice: {mode}

WORKSPACE FACTS:
{workspace_facts}

WORKSPACE BRIEF:
{workspace_brief}

CONVERSATION EXCERPT:
{excerpt}

DECISION NEEDED:
Identify edge cases, failure modes, and hidden dependencies for this request:
- What scenarios could cause this to fail or produce incorrect results?
- What are the likely failure modes?
- Are there hidden dependencies on environment, configuration, or external state?
- What edge cases should be considered (empty inputs, boundary conditions, unusual formats)?
- What mitigations would reduce risk for each identified edge case?"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        workflow = route_decision.workflow.choice,
        mode = route_decision.mode.choice,
        workspace_facts = workspace_facts.trim(),
        workspace_brief = workspace_brief.trim(),
        excerpt = excerpt,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_route_decision() -> RouteDecision {
        RouteDecision {
            route: "CHAT".to_string(),
            source: "test".to_string(),
            distribution: vec![("CHAT".to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: ProbabilityDecision {
                choice: "CHAT".to_string(),
                source: "test".to_string(),
                distribution: vec![("CHAT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            workflow: ProbabilityDecision {
                choice: "CHAT".to_string(),
                source: "test".to_string(),
                distribution: vec![("CHAT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            mode: ProbabilityDecision {
                choice: "DECIDE".to_string(),
                source: "test".to_string(),
                distribution: vec![("DECIDE".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            evidence_required: false,
        }
    }

    #[test]
    fn test_build_domain_difficulty_narrative_contains_user_message() {
        let narrative = build_domain_difficulty_narrative(
            "What is the capital of France?",
            &test_route_decision(),
            "src/main.rs\nCargo.toml",
            "Rust CLI project",
            &[],
        );
        assert!(narrative.contains("What is the capital of France?"));
        assert!(narrative.contains("DECISION NEEDED"));
    }

    #[test]
    fn test_build_freshness_requirement_narrative_contains_route_context() {
        let narrative = build_freshness_requirement_narrative(
            "What are the current Rust stable versions?",
            &test_route_decision(),
            "src/main.rs",
            "Rust CLI project",
            &[],
        );
        assert!(narrative.contains("route: CHAT"));
        assert!(narrative.contains("speech_act_choice: CHAT"));
    }

    #[test]
    fn test_build_assumption_tracker_narrative_contains_workspace() {
        let narrative = build_assumption_tracker_narrative(
            "How do I compile this project?",
            &test_route_decision(),
            "src/main.rs\nCargo.toml",
            "Rust CLI project with dependencies",
            &[],
        );
        assert!(narrative.contains("Rust CLI project with dependencies"));
        assert!(narrative.contains("WORKSPACE FACTS"));
    }

    #[test]
    fn test_build_edge_case_evaluator_narrative_contains_conversation() {
        let messages = vec![
            ChatMessage::simple("system", "You are a helpful assistant"),
            ChatMessage::simple("user", "Hello"),
            ChatMessage::simple("assistant", "Hi, how can I help?"),
        ];
        let narrative = build_edge_case_evaluator_narrative(
            "Can you explain the error handling?",
            &test_route_decision(),
            "src/main.rs",
            "Rust CLI project",
            &messages,
        );
        assert!(narrative.contains("CONVERSATION EXCERPT"));
        assert!(narrative.contains("Hello"));
    }

    #[test]
    fn test_all_narratives_include_decision_needed_section() {
        let narratives = [
            build_domain_difficulty_narrative(
                "test",
                &test_route_decision(),
                "facts",
                "brief",
                &[],
            ),
            build_freshness_requirement_narrative(
                "test",
                &test_route_decision(),
                "facts",
                "brief",
                &[],
            ),
            build_assumption_tracker_narrative(
                "test",
                &test_route_decision(),
                "facts",
                "brief",
                &[],
            ),
            build_edge_case_evaluator_narrative(
                "test",
                &test_route_decision(),
                "facts",
                "brief",
                &[],
            ),
        ];
        for narrative in &narratives {
            assert!(narrative.contains("DECISION NEEDED"));
        }
    }

    #[test]
    fn test_narratives_handle_empty_conversation() {
        let narrative = build_domain_difficulty_narrative(
            "Test message",
            &test_route_decision(),
            "facts",
            "brief",
            &[],
        );
        assert!(narrative.contains("Test message"));
        assert!(narrative.contains("facts"));
        assert!(narrative.contains("brief"));
    }
}
