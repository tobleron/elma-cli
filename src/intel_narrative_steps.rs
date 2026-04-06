//! @efficiency-role: util-pure
//!
//! Intel Narrative Step Functions
//!
//! Transforms program steps and step results into plain-text narratives
//! for workflow execution intel units (critic, sufficiency, reviewers, etc.)

use crate::intel_narrative_utils::{fallback_text, snippet};
use crate::{Program, Step, StepResult};

/// Shared helper: build steps narrative
///
/// Converts program steps and their results into readable narrative format.
pub(crate) fn build_steps_narrative(program: &Program, step_results: &[StepResult]) -> String {
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

pub(crate) fn build_step_results_narrative(step_results: &[StepResult]) -> String {
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

/// Extract step kind (shell, reply, plan, etc.)
pub(crate) fn step_kind(step: &Step) -> &'static str {
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
        Step::Respond { .. } => "respond",
        Step::Explore { .. } => "explore",
        Step::Write { .. } => "write",
        Step::Delete { .. } => "delete",
    }
}

/// Extract step detail (command, instructions, goal, etc.)
pub(crate) fn step_detail(step: &Step) -> String {
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
        Step::Respond { instructions, .. } => format!("Respond: \"{}\"", instructions.trim()),
        Step::Explore { objective, .. } => format!("Explore: \"{}\"", objective.trim()),
        Step::Write { path, .. } => format!("Write to \"{}\"", path.trim()),
        Step::Delete { path, .. } => format!("Delete \"{}\"", path.trim()),
    }
}

/// Extract step purpose
pub(crate) fn step_purpose(step: &Step) -> String {
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
        | Step::Reply { common, .. }
        | Step::Respond { common, .. }
        | Step::Explore { common, .. }
        | Step::Write { common, .. }
        | Step::Delete { common, .. } => common,
    };

    if !common.purpose.trim().is_empty() {
        common.purpose.trim().to_string()
    } else {
        step_kind(step).to_string()
    }
}

/// Extract step result text
pub(crate) fn step_result_text(step: &Step, step_results: &[StepResult]) -> String {
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
pub(crate) fn step_id(step: &Step) -> &str {
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
        | Step::Reply { id, .. }
        | Step::Respond { id, .. }
        | Step::Explore { id, .. }
        | Step::Write { id, .. }
        | Step::Delete { id, .. } => id,
    }
}

// Test helpers - visible to sibling modules during test builds
#[cfg(test)]
pub(crate) fn make_shell_step(id: &str, cmd: &str, purpose: &str) -> Step {
    use crate::types_core::StepCommon;
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

#[cfg(test)]
pub(crate) fn make_reply_step(id: &str, instructions: &str, purpose: &str) -> Step {
    use crate::types_core::StepCommon;
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

#[cfg(test)]
mod tests {
    use super::*;

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

        let narrative = crate::intel_narrative::build_critic_narrative(
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
            raw_output: Some("file1\nfile2".to_string())
        , ..Default::default()
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
            raw_output: Some("error: not found".to_string())
        , ..Default::default()
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
            raw_output: Some("file1.txt\nfile2.txt".to_string())
        , ..Default::default()
        }];

        let narrative = crate::intel_narrative::build_sufficiency_narrative(
            &program.objective,
            &program,
            &step_results,
        );

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
            raw_output: Some("file1.txt\nfile2.txt".to_string())
        , ..Default::default()
        }];

        let narrative = crate::intel_narrative::build_reviewer_narrative(
            &program.objective,
            &program,
            &step_results,
            "logical",
        );

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
            exit_code: Some(0)
        , ..Default::default()
        }];

        let narrative = crate::intel_narrative::build_claim_check_narrative(
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
