//! @efficiency-role: util-pure
//!
//! Intel Narrative Intent Module
//!
//! Transforms structured context into plain-text narratives for intent analysis intel units.

use crate::intel_narrative_utils::format_conversation_excerpt;
use crate::{ChatMessage, RouteDecision};

/// Build surface intent narrative
///
/// Transforms context into plain-text narrative for surface intent analysis.
pub(crate) fn build_surface_intent_narrative(
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

Analyze the surface-level intent of this user message:
- What is the literal request type? (question, task, advice)
- What output type is expected? (explanation, list, command, code)
- What format preference is indicated? (paragraph, table, concise)

Return JSON with: surface_intent, output_type, format_pref, plus choice/label/reason/entropy."#,
        user_message = user_message,
        route = route_decision.route,
        dist = crate::routing_calc::format_route_distribution(&route_decision.distribution),
        margin = route_decision.margin,
        entropy = route_decision.entropy,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

/// Build real intent narrative
///
/// Transforms context into plain-text narrative for real intent inference.
pub(crate) fn build_real_intent_narrative(
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

Infer the real underlying intent behind this user message:
- What is the actual problem they're trying to solve? (debug, learn, build, compare, safety)
- What type of problem is it? (specific, general)
- Is a decision needed? (boolean)

Return JSON with: real_intent, problem_type, decision_needed, plus choice/label/reason/entropy."#,
        user_message = user_message,
        route = route_decision.route,
        dist = crate::routing_calc::format_route_distribution(&route_decision.distribution),
        margin = route_decision.margin,
        entropy = route_decision.entropy,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

/// Build user expectation narrative
///
/// Transforms context into plain-text narrative for user expectation analysis.
pub(crate) fn build_user_expectation_narrative(
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

Determine the user's expectations from this message:
- What type of advice do they want? (practical, theory)
- What depth level? (quick, deep)
- What certainty preference? (high, probabilistic)
- What effort level do they expect? (low, high)

Return JSON with: expectation_type, depth_level, certainty_pref, effort_level, plus choice/label/reason/entropy."#,
        user_message = user_message,
        route = route_decision.route,
        dist = crate::routing_calc::format_route_distribution(&route_decision.distribution),
        margin = route_decision.margin,
        entropy = route_decision.entropy,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}
