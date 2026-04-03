//! @efficiency-role: service-orchestrator
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

use crate::{ChatMessage, Program, RouteDecision, ScopePlan, Step, StepResult};
use serde_json::Value;

// ============================================================================
// Classification Intel Narratives (Task 047)
// ============================================================================

/// Build complexity assessor input narrative
///
/// Transforms classification context into plain-text narrative format.
pub(crate) fn build_complexity_narrative(
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
{conversation}"#,
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

/// Format conversation excerpt into plain text
///
/// Converts conversation messages into readable narrative format.
fn format_conversation_excerpt(messages: &[ChatMessage], max_items: usize) -> String {
    messages
        .iter()
        .skip(1) // Skip first system message
        .rev()
        .take(max_items)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content.replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_json_value(value: &Value) -> String {
    if value.is_null() {
        "-".to_string()
    } else if let Some(text) = value.as_str() {
        text.trim().to_string()
    } else {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    }
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
{conversation}"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        complexity = render_json_value(complexity),
        scope = serde_json::to_string_pretty(scope).unwrap_or_default(),
        memory_candidates = render_json_value(memory_candidates),
        conversation = conversation_text,
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
    evidence_mode: &Value,
    response_advice: &Value,
    reply_instructions: &Value,
    step_results: &Value,
) -> String {
    let step_results_narrative = render_json_value(step_results);

    format!(
        r#"USER MESSAGE:
{user_message}

ROUTE CONTEXT:
- route: {route}
- speech_act: {speech_act}

EVIDENCE MODE:
{evidence_mode}

EXPERT RESPONSE ADVICE:
{response_advice}

REPLY INSTRUCTIONS:
{reply_instructions}

OBSERVED STEP RESULTS:
{step_results}"#,
        user_message = user_message.trim(),
        route = route_decision.route,
        speech_act = route_decision.speech_act.choice,
        evidence_mode = render_json_value(evidence_mode),
        response_advice = render_json_value(response_advice),
        reply_instructions = render_json_value(reply_instructions),
        step_results = step_results_narrative,
    )
}

pub(crate) fn build_expert_responder_narrative(
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
Return compact response advice that helps Elma present the outcome in the most useful way."#,
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
    let steps_narrative = build_steps_narrative(program, step_results);

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
    let steps_narrative = build_steps_narrative(program, step_results);

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
    let steps_narrative = build_steps_narrative(program, step_results);

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
    let step_results_narrative = build_step_results_narrative(step_results);

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

/// Build claim checker narrative.
pub(crate) fn build_claim_check_narrative(
    user_message: &str,
    evidence_mode: &str,
    draft: &str,
    step_results: &[StepResult],
) -> String {
    let step_results_narrative = build_step_results_narrative(step_results);

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

/// Shared helper: build steps narrative
///
/// Converts program steps and their results into readable narrative format.
fn build_steps_narrative(program: &Program, step_results: &[StepResult]) -> String {
    program
        .steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            let step_num = idx + 1;
            let step_type = step_kind(step);
            let step_detail = step_detail(step);
            let purpose = step_purpose(step);
            let result = step_result_text(step, step_results);

            format!(
                "Step {step_num} ({step_type}): {step_detail}\n  To: {purpose}\n  Result: {result}",
                step_num = step_num,
                step_type = step_type,
                step_detail = step_detail,
                purpose = purpose.trim(),
                result = result,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_step_results_narrative(step_results: &[StepResult]) -> String {
    if step_results.is_empty() {
        return "No step results available.".to_string();
    }

    step_results
        .iter()
        .enumerate()
        .map(|(idx, step_result)| {
            let output_excerpt = step_result
                .raw_output
                .as_deref()
                .map(snippet)
                .unwrap_or_else(|| "none".to_string());

            format!(
                "Result {step_num} ({kind}) id={id}\n  Purpose: {purpose}\n  Status: ok={ok}, exit_code={exit_code:?}, outcome={outcome}\n  Summary: {summary}\n  Output: {output}",
                step_num = idx + 1,
                kind = fallback_text(&step_result.kind, "unknown"),
                id = fallback_text(&step_result.id, "unknown"),
                purpose = fallback_text(&step_result.purpose, "unspecified"),
                ok = step_result.ok,
                exit_code = step_result.exit_code,
                outcome = step_result.outcome_status.as_deref().unwrap_or("unknown"),
                summary = fallback_text(&step_result.summary, "none"),
                output = output_excerpt,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn fallback_text<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

fn snippet(text: &str) -> String {
    let compact = text.replace('\n', " ");
    if compact.len() <= 240 {
        compact
    } else {
        format!("{}...", &compact[..240])
    }
}

/// Extract step kind (shell, reply, plan, etc.)
fn step_kind(step: &Step) -> &'static str {
    match step {
        Step::Shell { .. } => "shell",
        Step::Read { .. } => "read",
        Step::Search { .. } => "search",
        Step::Select { .. } => "select",
        Step::Plan { .. } => "plan",
        Step::MasterPlan { .. } => "masterplan",
        Step::Decide { .. } => "decide",
        Step::Summarize { .. } => "summarize",
        Step::Edit { .. } => "edit",
        Step::Reply { .. } => "reply",
    }
}

/// Extract step detail (command, instructions, goal, etc.)
fn step_detail(step: &Step) -> String {
    match step {
        Step::Shell { cmd, .. } => format!("Run \"{}\"", cmd.trim()),
        Step::Read { path, .. } => format!("Read \"{}\"", path.trim()),
        Step::Search { query, paths, .. } => {
            format!("Search for \"{}\" in {:?}", query.trim(), paths)
        }
        Step::Select { instructions, .. } => {
            format!("Select from options: \"{}\"", instructions.trim())
        }
        Step::Plan { goal, .. } | Step::MasterPlan { goal, .. } => {
            format!("Create plan: \"{}\"", goal.trim())
        }
        Step::Decide { prompt, .. } => {
            format!("Decide: \"{}\"", prompt.trim())
        }
        Step::Summarize { instructions, .. } => {
            format!("Summarize: \"{}\"", instructions.trim())
        }
        Step::Edit { spec, .. } => {
            format!(
                "Edit {}: {} \"{}\"",
                spec.path.trim(),
                spec.operation,
                spec.content.trim()
            )
        }
        Step::Reply { instructions, .. } => {
            format!("Reply: \"{}\"", instructions.trim())
        }
    }
}

/// Extract step purpose
fn step_purpose(step: &Step) -> String {
    let common = match step {
        Step::Shell { common, .. }
        | Step::Read { common, .. }
        | Step::Search { common, .. }
        | Step::Select { common, .. }
        | Step::Plan { common, .. }
        | Step::MasterPlan { common, .. }
        | Step::Decide { common, .. }
        | Step::Summarize { common, .. }
        | Step::Edit { common, .. }
        | Step::Reply { common, .. } => common,
    };

    if !common.purpose.trim().is_empty() {
        common.purpose.trim().to_string()
    } else {
        step_kind(step).to_string()
    }
}

/// Extract step result text
fn step_result_text(step: &Step, step_results: &[StepResult]) -> String {
    let step_id = step_id(step);

    // Find matching result
    let result = step_results.iter().find(|r| r.id == step_id);

    match result {
        Some(r) => {
            // Check if step was successful
            let exit_code_text = r.exit_code.map(|code| {
                if code == 0 {
                    "successfully".to_string()
                } else {
                    format!("with error (exit_code={})", code)
                }
            });

            // Get output preview
            let output_preview = r.raw_output.as_ref().map(|o: &String| {
                if o.len() > 200 {
                    format!("{}...", &o[..200])
                } else {
                    o.clone()
                }
            });

            match (exit_code_text, output_preview) {
                (Some(exit), Some(output)) => {
                    format!("Command executed {} (output: {})", exit, output)
                }
                (Some(exit), None) => format!("Command executed {}", exit),
                (None, Some(output)) => format!("Output: {}", output),
                (None, None) => "Completed".to_string(),
            }
        }
        None => "Not yet executed".to_string(),
    }
}

/// Extract step ID
fn step_id(step: &Step) -> &str {
    match step {
        Step::Shell { id, .. }
        | Step::Read { id, .. }
        | Step::Search { id, .. }
        | Step::Select { id, .. }
        | Step::Plan { id, .. }
        | Step::MasterPlan { id, .. }
        | Step::Decide { id, .. }
        | Step::Summarize { id, .. }
        | Step::Edit { id, .. }
        | Step::Reply { id, .. } => id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types_core::StepCommon;

    fn make_shell_step(id: &str, cmd: &str, purpose: &str) -> Step {
        Step::Shell {
            id: id.to_string(),
            cmd: cmd.to_string(),
            common: StepCommon {
                purpose: purpose.to_string(),
                depends_on: vec![],
                success_condition: "done".to_string(),
                ..Default::default()
            },
        }
    }

    fn make_reply_step(id: &str, instructions: &str, purpose: &str) -> Step {
        Step::Reply {
            id: id.to_string(),
            instructions: instructions.to_string(),
            common: StepCommon {
                purpose: purpose.to_string(),
                depends_on: vec![],
                success_condition: "done".to_string(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_build_critic_narrative_format() {
        let program = Program {
            objective: "List files in test directory".to_string(),
            steps: vec![
                make_shell_step("s1", "find test/ -type f", "List all files"),
                make_reply_step("r1", "The files are...", "Answer user's request"),
            ],
        };

        let step_results = vec![
            StepResult {
                id: "s1".to_string(),
                exit_code: Some(0),
                raw_output: Some("file1.txt\nfile2.txt".to_string()),
                ..Default::default()
            },
            StepResult {
                id: "r1".to_string(),
                exit_code: None,
                raw_output: Some("Response generated".to_string()),
                ..Default::default()
            },
        ];

        let narrative = build_critic_narrative(&program.objective, &program, &step_results, 1, 2);

        assert!(narrative.contains("OBJECTIVE:"));
        assert!(narrative.contains("WORKFLOW GENERATED:"));
        assert!(narrative.contains("Step 1 (shell):"));
        assert!(narrative.contains("Step 2 (reply):"));
        assert!(narrative.contains("ATTEMPT: 1 of 2"));
        assert!(narrative.contains("YOUR TASK:"));
    }

    #[test]
    fn test_step_result_text_success() {
        let step = make_shell_step("s1", "ls", "list files");
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            exit_code: Some(0),
            raw_output: Some("file1\nfile2".to_string()),
            ..Default::default()
        }];

        let result = step_result_text(&step, &step_results);
        assert!(result.contains("successfully"));
        assert!(result.contains("file1"));
    }

    #[test]
    fn test_step_result_text_error() {
        let step = make_shell_step("s1", "ls", "list files");
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            exit_code: Some(2),
            raw_output: Some("error: not found".to_string()),
            ..Default::default()
        }];

        let result = step_result_text(&step, &step_results);
        assert!(result.contains("error"));
        assert!(result.contains("exit_code=2"));
    }

    #[test]
    fn test_build_sufficiency_narrative_format() {
        let program = Program {
            objective: "List files in test directory".to_string(),
            steps: vec![make_shell_step(
                "s1",
                "find test/ -type f",
                "List all files",
            )],
        };
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            purpose: "List all files".to_string(),
            ok: true,
            exit_code: Some(0),
            raw_output: Some("file1.txt\nfile2.txt".to_string()),
            ..Default::default()
        }];

        let narrative = build_sufficiency_narrative(&program.objective, &program, &step_results);

        assert!(narrative.contains("OBJECTIVE:"));
        assert!(narrative.contains("WORKFLOW GENERATED:"));
        assert!(narrative.contains("Does the workflow output satisfy the objective?"));
    }

    #[test]
    fn test_build_reviewer_narrative_format() {
        let program = Program {
            objective: "List files in test directory".to_string(),
            steps: vec![make_shell_step(
                "s1",
                "find test/ -type f",
                "List all files",
            )],
        };
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            purpose: "List all files".to_string(),
            ok: true,
            exit_code: Some(0),
            raw_output: Some("file1.txt\nfile2.txt".to_string()),
            ..Default::default()
        }];

        let narrative =
            build_reviewer_narrative(&program.objective, &program, &step_results, "logical");

        assert!(narrative.contains("OBJECTIVE:"));
        assert!(narrative.contains("WORKFLOW GENERATED:"));
        assert!(narrative.contains("logically coherent"));
    }

    #[test]
    fn test_build_claim_check_narrative_includes_step_results() {
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            purpose: "List all files".to_string(),
            ok: true,
            summary: "Command succeeded".to_string(),
            raw_output: Some("file1.txt\nfile2.txt".to_string()),
            exit_code: Some(0),
            ..Default::default()
        }];

        let narrative = build_claim_check_narrative(
            "Show me the files",
            "RAW",
            "The files are file1.txt and file2.txt",
            &step_results,
        );

        assert!(narrative.contains("DRAFT RESPONSE TO CHECK:"));
        assert!(narrative.contains("OBSERVED STEP RESULTS:"));
        assert!(narrative.contains("Result 1 (shell)"));
    }
}
