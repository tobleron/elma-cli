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

use crate::{Program, Step, StepResult};

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
        "logical" => "Is this workflow logically coherent with no contradictory steps or broken dataflow?",
        "efficiency" => "Is this workflow reasonably efficient with no avoidable waste or redundant steps?",
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

/// Shared helper: build steps narrative
///
/// Converts program steps and their results into readable narrative format.
fn build_steps_narrative(
    program: &Program,
    step_results: &[StepResult],
) -> String {
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
            format!("Edit {}: {} \"{}\"", spec.path.trim(), spec.operation, spec.content.trim())
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
                (Some(exit), Some(output)) => format!("Command executed {} (output: {})", exit, output),
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

        let narrative = build_critic_narrative(
            &program.objective,
            &program,
            &step_results,
            1,
            2,
        );

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
}
