//! @efficiency-role: domain-logic
//!
//! Task management intel unit — Task 494.
//!
//! Creates task files in `_elma-tasks/` for both:
//!   Type A: Auto-generated when an Instruction node resolves to a Step
//!   Type B: User-initiated via direct request
//!
//! Each task maps back to its WorkGraph node and session for full traceability.

use crate::intel_trait::*;
use crate::task_persistence;
use crate::*;

/// Input for task creation — derived from an Instruction node or user request.
#[derive(Debug, Clone)]
pub(crate) struct TaskCreationInput {
    pub description: String,
    pub instruction_id: String,
    pub approach_id: String,
    pub session_id: String,
    pub step_type: String,
    pub source: TaskSource,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TaskSource {
    Workflow,
    User,
}

impl TaskSource {
    fn label(&self) -> &'static str {
        match self {
            TaskSource::Workflow => "auto",
            TaskSource::User => "user",
        }
    }
}

/// Intel unit that handles task file creation.
pub(crate) struct TaskManagementUnit {
    workspace: PathBuf,
    session_id: String,
}

impl TaskManagementUnit {
    pub fn new(workspace: PathBuf, session_id: String) -> Self {
        Self {
            workspace,
            session_id,
        }
    }

    /// Create a task from an instruction node context (Type A: auto-generated).
    /// Also writes to `_elma-tasks/` and session task list.
    pub fn create_instruction_task(
        &self,
        instruction_id: &str,
        approach_id: &str,
        description: &str,
        step_type: &str,
    ) -> Result<(u32, PathBuf)> {
        self.create_task_inner(
            instruction_id,
            approach_id,
            description,
            step_type,
            TaskSource::Workflow,
        )
    }

    /// Create a task from direct user request (Type B).
    pub fn create_user_task(
        &self,
        instruction_id: &str,
        approach_id: &str,
        description: &str,
    ) -> Result<(u32, PathBuf)> {
        self.create_task_inner(
            instruction_id,
            approach_id,
            description,
            "user_request",
            TaskSource::User,
        )
    }

    fn create_task_inner(
        &self,
        instruction_id: &str,
        approach_id: &str,
        description: &str,
        step_type: &str,
        source: TaskSource,
    ) -> Result<(u32, PathBuf)> {
        let elma_tasks_dir = self.workspace.join("_elma-tasks");
        std::fs::create_dir_all(&elma_tasks_dir)
            .context("create _elma-tasks dir")?;

        let (number, _) = task_persistence::next_task_filename(&elma_tasks_dir);
        let title = self.derive_task_title(description);

        let path = task_persistence::write_task_file(
            &self.workspace,
            number,
            &title,
            description,
            &self.session_id,
            approach_id,
            instruction_id,
            match source {
                TaskSource::Workflow => task_persistence::TaskSource::Workflow,
                TaskSource::User => task_persistence::TaskSource::User,
            },
        )?;

        Ok((number, path))
    }

    /// Derive a concise task title from the description.
    fn derive_task_title(&self, description: &str) -> String {
        let title = description
            .lines()
            .next()
            .unwrap_or(description)
            .trim()
            .to_string();
        // Truncate to reasonable length
        if title.len() > 80 {
            format!("{}...", &title[..77])
        } else {
            title
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_instruction_task() {
        let tmp = tempdir().unwrap();
        let unit = TaskManagementUnit::new(
            tmp.path().to_path_buf(),
            "sess_test".to_string(),
        );

        let (num, path) = unit
            .create_instruction_task(
                "instr_001",
                "a_123",
                "Read Cargo.toml to inspect dependencies",
                "read",
            )
            .unwrap();

        assert_eq!(num, 1);
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("001_"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Read Cargo.toml"));
        assert!(content.contains("auto"));
        assert!(content.contains("sess_test"));
        assert!(content.contains("instr_001"));

        let fname = path.file_name().unwrap().to_string_lossy();
        assert!(fname.starts_with("001_auto_"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("instr_001"));
    }

    #[test]
    fn test_create_user_task() {
        let tmp = tempdir().unwrap();
        let unit = TaskManagementUnit::new(
            tmp.path().to_path_buf(),
            "sess_test".to_string(),
        );

        let (num, path) = unit
            .create_user_task("usr_001", "a_456", "Add dark mode toggle")
            .unwrap();

        assert!(path.exists());
        let fname = path.file_name().unwrap().to_string_lossy();
        assert!(fname.starts_with("001_user_"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("user"));
        assert!(content.contains("Add dark mode toggle"));
    }

    #[test]
    fn test_sequential_task_numbers() {
        let tmp = tempdir().unwrap();
        let unit = TaskManagementUnit::new(
            tmp.path().to_path_buf(),
            "sess_1".to_string(),
        );

        let (n1, _) = unit
            .create_instruction_task("i1", "a1", "Task one", "shell")
            .unwrap();
        let (n2, _) = unit
            .create_instruction_task("i2", "a1", "Task two", "read")
            .unwrap();
        let (n3, _) = unit
            .create_instruction_task("i3", "a1", "Task three", "edit")
            .unwrap();

        assert_eq!(n1, 1);
        assert_eq!(n2, 2);
        assert_eq!(n3, 3);
    }

    #[test]
    fn test_task_title_truncation() {
        let tmp = tempdir().unwrap();
        let unit = TaskManagementUnit::new(tmp.path().to_path_buf(), "s".to_string());
        let long = "A".repeat(200);
        let title = unit.derive_task_title(&long);
        assert!(title.len() <= 80);
        assert!(title.ends_with("..."));
    }
}
