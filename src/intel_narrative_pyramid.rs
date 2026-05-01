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
/// Asks the model to produce a single-line OBJECTIVE DSL. GOAL/TASK
/// decomposition was removed (Task 419) because 3B models cannot reliably
/// produce multi-line block DSL with quoted fields and END terminators.
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
        r#"Decompose the following request into one objective.

USER REQUEST:
{user_message}

ROUTE: {route}
ROUTE MARGIN: {margin:.2}
ROUTE ENTROPY: {entropy:.2}
{failure_hint}
WORKSPACE FACTS:
{workspace_facts}

WORKSPACE BRIEF:
{workspace_brief}

CONVERSATION (most recent last):
{conversation}

Output exactly one DSL line:
OBJECTIVE text="<one-line objective>" risk=low|medium|high

Rules:
- text: one sentence capturing what the user wants done, inside double quotes.
- risk: low if read-only, medium if modifies files, high if it runs unknown commands.
- No prose before or after. No fenced code block. Just the DSL line."#,
        user_message = user_message.trim(),
        route = route_decision.route,
        margin = route_decision.margin,
        entropy = route_decision.entropy,
        workspace_facts = workspace_facts.trim(),
        workspace_brief = workspace_brief.trim(),
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
