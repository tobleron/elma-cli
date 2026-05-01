//! @efficiency-role: util-pure
//!
//! Intel Narrative Module
//!
//! Transforms structured program/step data into plain-text narratives
//! for intel units (critic, sufficiency, reviewers, etc.)
//!
//! This module centralizes narrative transformation logic, ensuring:
//! - Consistent format across all intel units
//! - Single point of change for narrative format updates
//! - Future-proof: can swap to model-based narrative without changing callers

use crate::{Program, RouteDecision, StepResult};
use serde_json::Value;

// Re-export for external callers and bring into local scope
pub(crate) use crate::intel_narrative_steps::{
    build_step_results_narrative, build_steps_narrative, step_detail, step_id, step_kind,
    step_purpose, step_result_text,
};
// Re-export planning narratives
pub(crate) use crate::intel_narrative_planning::{
    build_action_needs_narrative, build_claim_check_narrative, build_complexity_narrative,
    build_evidence_needs_narrative, build_formula_selector_narrative,
    build_rename_suggester_narrative, build_scope_builder_narrative, build_selector_narrative,
    build_workflow_planner_narrative,
};
// Re-export pyramid narratives
pub(crate) use crate::intel_narrative_pyramid::{
    build_decomposition_narrative, build_next_action_narrative,
};
use crate::intel_narrative_utils::{format_conversation_excerpt, render_json_value};

// Re-export test helpers
#[cfg(test)]
pub(crate) use crate::intel_narrative_steps::{make_reply_step, make_shell_step};
// Re-export utils for tests
#[cfg(test)]
pub(crate) use crate::intel_narrative_utils::{fallback_text, snippet};

/// Build evidence mode classifier narrative.
pub(crate) fn build_evidence_mode_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
    has_command_request: bool,
    has_command_execution: bool,
    has_artifact: bool,
) -> String {
    build_evidence_mode_narrative_with_ledger(
        user_message,
        route_decision,
        reply_instructions,
        step_results,
        has_command_request,
        has_command_execution,
        has_artifact,
        None,
    )
}

pub(crate) fn build_evidence_mode_narrative_with_ledger(
    user_message: &str,
    route_decision: &RouteDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
    has_command_request: bool,
    has_command_execution: bool,
    has_artifact: bool,
    ledger: Option<&crate::evidence_ledger::EvidenceLedger>,
) -> String {
    let step_results_narrative = build_step_results_narrative(step_results, ledger);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act_choice: {speech_act}

REPLY INSTRUCTIONS:
{reply_instructions}

EXECUTION SIGNALS:
- explicit_command_request: {has_command_request}
- observed_command_execution: {has_command_execution}
- artifact_captured: {has_artifact}

STEP RESULTS:
{step_results_narrative}"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        reply_instructions = reply_instructions.trim(),
        has_command_request = has_command_request,
        has_command_execution = has_command_execution,
        has_artifact = has_artifact,
        step_results_narrative = step_results_narrative,
    )
}

pub(crate) fn build_evidence_compactor_narrative(
    objective: &Value,
    purpose: &Value,
    scope: &Value,
    cmd: &Value,
    output: &Value,
) -> String {
    format!(
        r#"OBJECTIVE:
{objective}

STEP PURPOSE:
{purpose}

SCOPE:
{scope}

COMMAND:
{cmd}

RAW EVIDENCE TO COMPACT:
{output}"#,
        objective = render_json_value(objective),
        purpose = render_json_value(purpose),
        scope = render_json_value(scope),
        cmd = render_json_value(cmd),
        output = render_json_value(output),
    )
}

pub(crate) fn build_artifact_classifier_narrative(
    objective: &Value,
    scope: &Value,
    evidence: &Value,
) -> String {
    format!(
        r#"OBJECTIVE:
{objective}

SCOPE:
{scope}

ARTIFACT EVIDENCE TO CLASSIFY:
{evidence}"#,
        objective = render_json_value(objective),
        scope = render_json_value(scope),
        evidence = render_json_value(evidence),
    )
}

pub(crate) fn build_result_presenter_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    runtime_context: &Value,
    evidence_mode: &Value,
    response_advice: &Value,
    reply_instructions: &Value,
    step_results: &Value,
    intent_surface: &serde_json::Value,
    intent_real: &serde_json::Value,
    user_expectation: &serde_json::Value,
) -> String {
    let step_results_narrative = render_json_value(step_results);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act: {speech_act}

RUNTIME CONTEXT:
{runtime_context}

EVIDENCE MODE:
{evidence_mode}

EXPERT RESPONSE ADVICE:
{response_advice}

REPLY INSTRUCTIONS:
{reply_instructions}

OBSERVED STEP RESULTS (GROUNDING DATA):
{step_results}

INTENT SURFACE ANALYSIS:
{intent_surface}

INTENT REAL ANALYSIS:
{intent_real}

USER EXPECTATION ANALYSIS:
{user_expectation}

PRESENTATION RULES:
1. ONLY use the provided STEP RESULTS for technical claims.
2. If the results are empty or do not support the user's request, state that clearly and honestly.
3. DO NOT add "I am Elma" or "Here are your results" boilerplate.
4. DO NOT provide tutorials, marketing fluff, or slide-deck formatting unless explicitly asked in the USER MESSAGE.
5. PRESERVE exact relative paths (e.g. "src/main.rs") and identifiers."#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        runtime_context = render_json_value(runtime_context),
        evidence_mode = render_json_value(evidence_mode),
        response_advice = render_json_value(response_advice),
        reply_instructions = render_json_value(reply_instructions),
        step_results = step_results_narrative,
        intent_surface = render_json_value(intent_surface),
        intent_real = render_json_value(intent_real),
        user_expectation = render_json_value(user_expectation),
    )
}

pub(crate) fn build_expert_advisor_narrative(
    user_message: &str,
    route_decision: &RouteDecision,
    evidence_mode: &Value,
    reply_instructions: &Value,
    step_results: &Value,
) -> String {
    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act: {speech_act}

EVIDENCE MODE:
{evidence_mode}

REPLY INSTRUCTIONS:
{reply_instructions}

OBSERVED STEP RESULTS:
{step_results}

TASK:
Return compact response advice that helps Elma present the outcome in the most useful way.
Identify if the evidence is sufficient, partial, or missing.
Advise on the most direct and honest posture."#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        evidence_mode = render_json_value(evidence_mode),
        reply_instructions = render_json_value(reply_instructions),
        step_results = render_json_value(step_results),
    )
}

pub(crate) fn build_status_message_narrative(
    current_action: &Value,
    step_type: &Value,
    step_purpose: &Value,
) -> String {
    format!(
        r#"CURRENT ACTION:
{current_action}

STEP TYPE:
{step_type}

STEP PURPOSE:
{step_purpose}"#,
        current_action = render_json_value(current_action),
        step_type = render_json_value(step_type),
        step_purpose = render_json_value(step_purpose),
    )
}

pub(crate) fn build_command_repair_narrative(
    objective: &Value,
    purpose: &Value,
    cmd: &str,
    output: &Value,
) -> String {
    format!(
        r#"OBJECTIVE:
{objective}

STEP PURPOSE:
{purpose}

FAILED COMMAND:
{cmd}

FAILED OUTPUT:
{output}"#,
        objective = render_json_value(objective),
        purpose = render_json_value(purpose),
        cmd = cmd.trim(),
        output = render_json_value(output),
    )
}

// ============================================================================
// Workflow Execution Narratives (Original)
// ============================================================================

/// Build critic input narrative
///
/// Transforms structured program and step results into a plain-text story
/// that the critic can reason about without JSON noise.
pub(crate) fn build_critic_narrative(
    objective: &str,
    program: &Program,
    step_results: &[StepResult],
    attempt: u32,
    max_retries: u32,
) -> String {
    build_critic_narrative_with_ledger(objective, program, step_results, attempt, max_retries, None)
}

pub(crate) fn build_critic_narrative_with_ledger(
    objective: &str,
    program: &Program,
    step_results: &[StepResult],
    attempt: u32,
    max_retries: u32,
    ledger: Option<&crate::evidence_ledger::EvidenceLedger>,
) -> String {
    let steps_narrative = build_steps_narrative(program, step_results, ledger);

    format!(
        r#"OBJECTIVE:
{objective}

WORKFLOW GENERATED:
{steps_narrative}

ATTEMPT: {attempt} of {max_retries}

YOUR TASK:
Does this workflow and its results achieve the objective?
Answer with ONLY: {{"status": "ok" or "retry", "reason": "one short sentence"}}"#,
        objective = objective.trim(),
        steps_narrative = steps_narrative,
        attempt = attempt,
        max_retries = max_retries,
    )
}

/// Build sufficiency verifier input narrative
///
/// Transforms structured data into plain-text for sufficiency verification.
pub(crate) fn build_sufficiency_narrative(
    objective: &str,
    program: &Program,
    step_results: &[StepResult],
) -> String {
    build_sufficiency_narrative_with_ledger(objective, program, step_results, None)
}

pub(crate) fn build_sufficiency_narrative_with_ledger(
    objective: &str,
    program: &Program,
    step_results: &[StepResult],
    ledger: Option<&crate::evidence_ledger::EvidenceLedger>,
) -> String {
    let steps_narrative = build_steps_narrative(program, step_results, ledger);

    format!(
        r#"OBJECTIVE:
{objective}

WORKFLOW GENERATED:
{steps_narrative}

YOUR TASK:
Does the workflow output satisfy the objective?
Answer with ONLY: {{"status": "ok" or "retry", "reason": "one short sentence"}}"#,
        objective = objective.trim(),
        steps_narrative = steps_narrative,
    )
}

/// Build reviewer input narrative (logical, efficiency, risk)
///
/// Transforms structured data into plain-text for reviewer intel units.
pub(crate) fn build_reviewer_narrative(
    objective: &str,
    program: &Program,
    step_results: &[StepResult],
    review_type: &str,
) -> String {
    build_reviewer_narrative_with_ledger(objective, program, step_results, review_type, None)
}

pub(crate) fn build_reviewer_narrative_with_ledger(
    objective: &str,
    program: &Program,
    step_results: &[StepResult],
    review_type: &str,
    ledger: Option<&crate::evidence_ledger::EvidenceLedger>,
) -> String {
    let steps_narrative = build_steps_narrative(program, step_results, ledger);

    let task_description = match review_type {
        "logical" => {
            "Is this workflow logically coherent with no contradictory steps or broken dataflow?"
        }
        "efficiency" => {
            "Is this workflow reasonably efficient with no avoidable waste or redundant steps?"
        }
        "risk" => "Does this workflow have any safety concerns or risky operations?",
        _ => "Review this workflow for issues.",
    };

    format!(
        r#"OBJECTIVE:
{objective}

WORKFLOW GENERATED:
{steps_narrative}

YOUR TASK:
{task_description}
Answer with ONLY: {{"status": "ok" or "retry", "reason": "one short sentence"}}"#,
        objective = objective.trim(),
        steps_narrative = steps_narrative,
        task_description = task_description,
    )
}

/// Build repair semantics guard narrative.
pub(crate) fn build_repair_semantics_narrative(
    objective: &str,
    purpose: &str,
    original_cmd: &str,
    repaired_cmd: &str,
    failed_output_summary: &str,
) -> String {
    format!(
        r#"OBJECTIVE:
{objective}

STEP PURPOSE:
{purpose}

ORIGINAL COMMAND:
{original_cmd}

REPAIRED COMMAND:
{repaired_cmd}

FAILED OUTPUT SUMMARY:
{failed_output_summary}"#,
        objective = objective.trim(),
        purpose = purpose.trim(),
        original_cmd = original_cmd.trim(),
        repaired_cmd = repaired_cmd.trim(),
        failed_output_summary = failed_output_summary.trim(),
    )
}
