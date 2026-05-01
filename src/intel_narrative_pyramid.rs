//! @efficiency-role: util-pure
//!
//! Pyramid Narrative Builders
//!
//! Transforms user request + route context into plain-text narratives
//! for the decomposition pyramid intel unit and next-action selector.

use crate::intel_narrative_utils::{format_conversation_excerpt, render_json_value};
use crate::{ChatMessage, RouteDecision};

/// Build the narrative prompt for the decomposition intel unit.
///
/// Asks the model to produce objective → goals → tasks in compact DSL.
pub(crate) fn build_decomposition_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
    prior_failures: usize,
) -> String {
    let conversation_text = format_conversation_excerpt(conversation, 8);

    let failure_hint = if prior_failures > 0 {
        format!("\nNOTE: Prior action-DSL failures: {prior_failures}. Decompose carefully to keep each step small.\n")
    } else {
        String::new()
    };

    format!(
        r#"Decompose the following request into one objective, bounded goals, and numbered tasks.

USER REQUEST:
{user_message}

ROUTE: {route}
ROUTE MARGIN: {margin:.2}
ROUTE ENTROPY: {entropy:.2}
{failure_hint}
WORKSPACE FACTS:
{facts}

WORKSPACE BRIEF:
{brief}

CONVERSATION (most recent last):
{conversation}

Output compact DSL ONLY:

OBJECTIVE text="<one-line objective>" risk=low|medium|high
GOAL text="<goal description>" evidence_needed=true|false
GOAL text="<goal description>" evidence_needed=true|false
TASK id=1 text="<task description>" status=ready
TASK id=2 text="<task description>" status=pending
TASK id=3 text="<task description>" status=pending
END

Rules:
- One OBJECTIVE that captures the whole request.
- 1-3 GOALs covering different aspects.
- 1-6 TASKs total, each actionable. id must be a unique integer.
- Set status=ready for the first task to do, status=pending for the rest.
- Use evidence_needed=true only if a goal requires reading/searching/looking up facts.
- risk=high only if the request modifies files, runs unknown commands, or has irreversible effects.
- No prose before or after. No fenced code block. No JSON. Just the DSL block."#,
        user_message = user_message.trim(),
        route = route_decision.route,
        margin = route_decision.margin,
        entropy = route_decision.entropy,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}

/// Build the narrative prompt for next-action selection.
///
/// Used when a repair loop is stuck: ask the model to pick a different task.
pub(crate) fn build_next_action_narrative(
    objective: &str,
    tasks_json: &serde_json::Value,
    last_error: &str,
) -> String {
    let tasks_text = render_json_value(tasks_json);

    format!(
        r#"OBJECTIVE: {objective}

AVAILABLE TASKS:
{tasks_text}

LAST ACTION DSL ERROR: {last_error}

Select the NEXT task to advance the objective.

Output ONE line of compact DSL:
NEXT task_id=<id> action=read|list|search|shell|edit|ask|done reason="<why this task>"

Rules:
- Pick a task whose status is "ready" or "active".
- If the last error was a parse/syntax issue, prefer a simpler action (read/list/search).
- No prose. No fenced code. No JSON. Just the ONE NEXT line."#,
        objective = objective.trim(),
        tasks_text = tasks_text,
        last_error = last_error.trim(),
    )
}
