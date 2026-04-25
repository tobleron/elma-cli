//! @efficiency-role: util-pure
//!
//! Portable Elma project scaffold generator.

use crate::*;

pub(crate) fn init_project_scaffold(root: &Path) -> Result<Vec<PathBuf>> {
    let mut created = Vec::new();

    let dirs = [
        root.join("_tasks"),
        root.join("_tasks").join("active"),
        root.join("_tasks").join("pending"),
        root.join("_tasks").join("completed"),
        root.join("_tasks").join("postponed"),
        root.join("_dev-tasks"),
    ];

    for dir in dirs {
        if !dir.exists() {
            std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
            created.push(dir);
        }
    }

    created.extend(write_if_missing(
        &root.join("AGENTS.md"),
        default_agents_template(),
    )?);
    created.extend(write_if_missing(
        &root.join("_tasks").join("TASKS.md"),
        default_tasks_template(),
    )?);
    created.extend(write_if_missing(
        &root
            .join("_tasks")
            .join("active")
            .join("001_Project_Master_Plan.md"),
        default_master_plan_template(),
    )?);

    Ok(created)
}

fn write_if_missing(path: &Path, content: String) -> Result<Vec<PathBuf>> {
    if path.exists() {
        return Ok(Vec::new());
    }
    std::fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(vec![path.to_path_buf()])
}

fn default_agents_template() -> String {
    r#"# AGENTS.md

## Project Guidance
- Read `_tasks/TASKS.md` before substantial work.
- Keep changes grounded in actual workspace evidence.
- Prefer surgical edits over broad refactors.
- Use root-relative paths in reasoning and edits.

## Task Protocol
1. Move the intended task into `_tasks/active/` when formally starting it.
2. Implement surgically.
3. Verify with build/tests relevant to the change.
4. Report results while the task is still active.
5. Archive only after approval.
"#
    .to_string()
}

fn default_tasks_template() -> String {
    r#"# Task Management

## Current Master Plan
- `001_Project_Master_Plan.md`

## Workflow
1. Pickup the task into `_tasks/active/`.
2. Implement surgically.
3. Verify build/tests.
4. Report before archiving.
"#
    .to_string()
}

fn default_master_plan_template() -> String {
    r#"# Task 001: Project Master Plan

## Objective
Capture the current project direction and the next implementation slices.

## Initial Checklist
- [ ] Confirm the project goal.
- [ ] Break the work into pending tasks.
- [ ] Verify the first implementation slice.
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_project_scaffold_creates_portable_layout() {
        let root = std::env::temp_dir().join(format!(
            "elma_init_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let created = init_project_scaffold(&root).unwrap();
        assert!(!created.is_empty());
        assert!(root.join("AGENTS.md").exists());
        assert!(root.join("_tasks").join("TASKS.md").exists());
        assert!(root.join("_tasks").join("active").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
