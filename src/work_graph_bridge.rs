//! @efficiency-role: service-orchestrator
//!
//! Bridge between pyramid work graph, task persistence, and step execution.
//! Task 494: Wires Instruction → PersistedTask → Step → StepResult.

use anyhow::{Context, Result};
use crate::task_persistence::{PersistedTask, PersistedTaskList, TaskItemStatus, TaskSource};
use crate::task_persistence;
use crate::intel_units::intel_units_task_management::TaskManagementUnit;
use crate::{PathBuf, SessionPaths};
use crate::{Program, Step, StepResult};

/// Create a PersistedTask from a Step and link it to a work graph instruction node.
pub(crate) fn step_to_persisted_task(
    step: &Step,
    instruction_id: &str,
    approach_id: &str,
    task_id: u32,
) -> PersistedTask {
    let (step_type, step_desc, step_id) = match step {
        Step::Shell { id, cmd, .. } => ("shell", cmd.clone(), id.clone()),
        Step::Read { id, path, .. } => (
            "read",
            path.as_ref().map(|p| p.to_string()).unwrap_or_default(),
            id.clone(),
        ),
        Step::Search { id, query, .. } => ("search", query.clone(), id.clone()),
        Step::Select { id, instructions, .. } => ("select", instructions.clone(), id.clone()),
        Step::Plan { id, goal, .. } => ("plan", goal.clone(), id.clone()),
        Step::MasterPlan { id, goal, .. } => ("masterplan", goal.clone(), id.clone()),
        Step::Decide { id, prompt, .. } => ("decide", prompt.clone(), id.clone()),
        Step::Summarize { id, text, .. } => ("summarize", text.clone(), id.clone()),
        Step::Edit { id, .. } => ("edit", String::new(), id.clone()),
        Step::Reply { id, instructions, .. } => ("reply", instructions.clone(), id.clone()),
        Step::Respond { id, instructions, .. } => {
            ("respond", instructions.clone(), id.clone())
        }
        Step::Explore { id, objective, .. } => ("explore", objective.clone(), id.clone()),
        Step::Write { id, path, .. } => ("write", path.clone(), id.clone()),
        Step::Delete { id, path, .. } => ("delete", path.clone(), id.clone()),
    };

    let description = if step_desc.len() > 200 {
        format!("{}...", &step_desc[..197])
    } else {
        step_desc
    };

    PersistedTask {
        id: task_id,
        instruction_id: instruction_id.to_string(),
        approach_id: approach_id.to_string(),
        description,
        status: TaskItemStatus::Pending,
        step_type: step_type.to_string(),
        step_result: None,
        created_at: now_unix_s(),
        completed_at: None,
    }
}

/// Update persisted tasks when step results are available.
/// Marks matching tasks as completed or failed based on StepResult.
pub(crate) fn update_tasks_from_results(
    tasks: &mut PersistedTaskList,
    step_results: &[StepResult],
) {
    for result in step_results {
        // Find task by matching step id or description
        if let Some(task) = tasks.tasks.iter_mut().find(|t| {
            t.instruction_id == result.id
                || t.description.contains(&result.id)
        }) {
            task.status = if result.ok {
                TaskItemStatus::Completed
            } else {
                TaskItemStatus::Failed
            };
            task.step_result = Some(if result.ok { "success" } else { "failure" }.to_string());
            task.completed_at = Some(now_unix_s());
        }
    }
}

/// Full pipeline: create tasks from a program, linking to work graph instruction node.
/// Returns the task list with one task per step.
pub(crate) fn create_tasks_from_program(
    program: &Program,
    instruction_id: &str,
    approach_id: &str,
) -> PersistedTaskList {
    let mut list = PersistedTaskList::new();
    for step in &program.steps {
        let task = step_to_persisted_task(step, instruction_id, approach_id, list.next_id);
        list.add_task(task);
    }
    list
}

/// Persist tasks to session storage and optionally generate _elma-tasks/ files.
pub(crate) fn finalize_tasks_to_session(
    session_root: &PathBuf,
    workspace: &PathBuf,
    tasks: &PersistedTaskList,
    session_id: &str,
    approach_id: &str,
    auto_generate_files: bool,
) -> Result<()> {
    // Persist to session
    task_persistence::save_session_tasks(session_root, tasks)
        .context("save session tasks")?;

    // Optionally auto-generate _elma-tasks/ files
    if auto_generate_files {
        let unit = TaskManagementUnit::new(workspace.clone(), session_id.to_string());
        for task in &tasks.tasks {
            let _ = unit.create_instruction_task(
                &task.instruction_id,
                approach_id,
                &task.description,
                &task.step_type,
            );
        }
    }

    Ok(())
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_to_persisted_task() {
        let step = Step::Shell {
            id: "s1".to_string(),
            cmd: "cargo build".to_string(),
            common: crate::StepCommon::default(),
        };

        let task = step_to_persisted_task(&step, "instr_001", "a_123", 1);
        assert_eq!(task.id, 1);
        assert_eq!(task.instruction_id, "instr_001");
        assert_eq!(task.approach_id, "a_123");
        assert_eq!(task.description, "cargo build");
        assert_eq!(task.step_type, "shell");
        assert_eq!(task.status, TaskItemStatus::Pending);
    }

    #[test]
    fn test_update_tasks_from_results() {
        let mut list = PersistedTaskList::new();
        list.add_task(PersistedTask {
            id: 1,
            instruction_id: "s1".to_string(),
            approach_id: "a_123".to_string(),
            description: "cargo build".to_string(),
            status: TaskItemStatus::InProgress,
            step_type: "shell".to_string(),
            step_result: None,
            created_at: 1000,
            completed_at: None,
        });

        let results = vec![StepResult {
            id: "s1".to_string(),
            ok: true,
            summary: "build succeeded".to_string(),
            ..StepResult::default()
        }];

        update_tasks_from_results(&mut list, &results);
        assert_eq!(list.tasks[0].status, TaskItemStatus::Completed);
        assert_eq!(list.tasks[0].step_result.as_deref(), Some("success"));
        assert!(list.tasks[0].completed_at.is_some());
    }

    #[test]
    fn test_create_tasks_from_program() {
        let program = Program {
            objective: "Build the project".to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: "cargo build".to_string(),
                    common: crate::StepCommon::default(),
                },
                Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Report results".to_string(),
                    common: crate::StepCommon::default(),
                },
            ],
        };

        let tasks = create_tasks_from_program(&program, "instr_root", "a_root");
        assert_eq!(tasks.tasks.len(), 2);
        assert_eq!(tasks.tasks[0].description, "cargo build");
        assert_eq!(tasks.tasks[0].step_type, "shell");
        assert_eq!(tasks.tasks[1].description, "Report results");
        assert_eq!(tasks.tasks[1].step_type, "reply");
    }
}
