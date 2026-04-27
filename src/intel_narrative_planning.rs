//! @efficiency-role: util-pure
//!
//! Intel Narrative Planning Module
//!
//! Transforms structured program/step data into plain-text narratives
//! for planning-related intel units (complexity, workflow, scope, formula,
//! selector, rename, claim check, action needs).

use crate::intel_narrative_utils::{format_conversation_excerpt, render_json_value};
use crate::{ChatMessage, RouteDecision, ScopePlan};
use serde_json::Value;

/// Build complexity assessor input narrative
///
/// Transforms classification context into plain-text narrative format.
pub(crate) fn build_complexity_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
    intent_surface: &serde_json::Value,
    intent_real: &serde_json::Value,
    user_expectation: &serde_json::Value,
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 12);

    format!(
        r#"User message:
{user_message}

Route prior:
- route: {route}
- distribution: {dist}
- margin: {margin:.2}
- entropy: {entropy:.2}

Workspace facts:
{facts}

Workspace brief:
{brief}

Conversation so far (most recent last):
{conversation}

Intent surface analysis:
{intent_surface}

Intent real analysis:
{intent_real}

User expectation analysis:
{user_expectation}"#,
        user_message = user_message,
        route = route_decision.route,
        dist = crate::routing_calc::format_route_distribution(&route_decision.distribution),
        margin = route_decision.margin,
        entropy = route_decision.entropy,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
        intent_surface = render_json_value(intent_surface),
        intent_real = render_json_value(intent_real),
        user_expectation = render_json_value(user_expectation),
    )
}

/// Build evidence needs assessor input narrative
///
/// Transforms classification context into plain-text narrative format.
pub(crate) fn build_evidence_needs_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 12);

    format!(
        r#"User message:
{user_message}

Route: {route}

Workspace facts:
{facts}

Workspace brief:
{brief}

Conversation so far (most recent last):
{conversation}"#,
        user_message = user_message,
        route = route_decision.route,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

/// Build action needs assessor input narrative
///
/// Transforms classification context into plain-text narrative format.
pub(crate) fn build_action_needs_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 12);

    format!(
        r#"User message:
{user_message}

Route: {route}

Workspace facts:
{facts}

Workspace brief:
{brief}

Conversation so far (most recent last):
{conversation}"#,
        user_message = user_message,
        route = route_decision.route,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

/// Build workflow planner input narrative
///
/// Transforms classification context into plain-text narrative format.
pub(crate) fn build_workflow_planner_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 12);

    format!(
        r#"User message:
{user_message}

Classification priors:
- speech_act: {speech_act}
- workflow: {workflow}
- mode: {mode}
- route: {route}

Workspace facts:
{facts}

Workspace brief:
{brief}

Conversation so far (most recent last):
{conversation}"#,
        user_message = user_message,
        speech_act = route_decision.speech_act.choice,
        workflow = route_decision.workflow.choice,
        mode = route_decision.mode.choice,
        route = route_decision.route,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

pub(crate) fn build_scope_builder_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &Value,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 12);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act: {speech_act}

COMPLEXITY CONTEXT:
{complexity}

WORKSPACE FACTS:
{facts}

WORKSPACE BRIEF:
{brief}

CONVERSATION SO FAR (most recent last):
{conversation}"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        complexity = render_json_value(complexity),
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

pub(crate) fn build_formula_selector_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &Value,
    scope: &ScopePlan,
    memory_candidates: &Value,
    conversation: &[ChatMessage],
    intent_surface: &serde_json::Value,
    intent_real: &serde_json::Value,
    user_expectation: &serde_json::Value,
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 12);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act: {speech_act}

COMPLEXITY CONTEXT:
{complexity}

SCOPE PLAN:
{scope}

MEMORY CANDIDATES:
{memory_candidates}

CONVERSATION SO FAR (most recent last):
{conversation}

INTENT SURFACE ANALYSIS:
{intent_surface}

INTENT REAL ANALYSIS:
{intent_real}

USER EXPECTATION ANALYSIS:
{user_expectation}"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        complexity = render_json_value(complexity),
        scope = serde_json::to_string_pretty(scope).unwrap_or_default(),
        memory_candidates = render_json_value(memory_candidates),
        conversation = conversation_text,
        intent_surface = render_json_value(intent_surface),
        intent_real = render_json_value(intent_real),
        user_expectation = render_json_value(user_expectation),
    )
}

pub(crate) fn build_selector_narrative(
    objective: &str,
    purpose: &Value,
    instructions: &Value,
    evidence: &Value,
) -> String {
    format!(
        r#"OBJECTIVE:
{objective}

STEP PURPOSE:
{purpose}

SELECTION INSTRUCTIONS:
{instructions}

OBSERVED EVIDENCE:
{evidence}"#,
        objective = objective.trim(),
        purpose = render_json_value(purpose),
        instructions = render_json_value(instructions),
        evidence = render_json_value(evidence),
    )
}

pub(crate) fn build_rename_suggester_narrative(
    objective: &str,
    purpose: &Value,
    instructions: &Value,
    evidence: &Value,
) -> String {
    format!(
        r#"OBJECTIVE:
{objective}

STEP PURPOSE:
{purpose}

RENAME INSTRUCTIONS:
{instructions}

GROUNDED EVIDENCE:
{evidence}"#,
        objective = objective.trim(),
        purpose = render_json_value(purpose),
        instructions = render_json_value(instructions),
        evidence = render_json_value(evidence),
    )
}

pub(crate) fn build_claim_check_narrative(
    user_message: &str,
    evidence_mode: &str,
    draft: &str,
    step_results: &[crate::StepResult],
) -> String {
    let step_results_narrative =
        crate::intel_narrative_steps::build_step_results_narrative(step_results, None);

    format!(
        r#"USER MESSAGE:
{user_message}

EVIDENCE PRESENTATION MODE:
{evidence_mode}

DRAFT RESPONSE TO CHECK:
{draft}

OBSERVED STEP RESULTS:
{step_results_narrative}"#,
        user_message = user_message.trim(),
        evidence_mode = evidence_mode.trim(),
        draft = draft.trim(),
        step_results_narrative = step_results_narrative,
    )
}
