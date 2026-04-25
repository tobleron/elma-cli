//! @efficiency-role: service-orchestrator
//!
//! Project Task Steward Skill — manages `_tasks` using `AGENTS.md`
//! and `_tasks/TASKS.md` as mandatory guidance.

use crate::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskStewardOp {
    pub op: String,
    pub task_id: String,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskInventory {
    pub active: Vec<TaskFile>,
    pub pending: Vec<TaskFile>,
    pub completed: Vec<TaskFile>,
    pub postponed: Vec<TaskFile>,
    pub next_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TaskFile {
    pub path: String,
    pub number: u32,
    pub title: String,
}

/// Scan all task directories and build an inventory.
pub(crate) fn scan_task_inventory(root: &Path) -> TaskInventory {
    let mut inventory = TaskInventory {
        active: Vec::new(),
        pending: Vec::new(),
        completed: Vec::new(),
        postponed: Vec::new(),
        next_number: 1,
    };

    let dirs = [
        (root.join("_tasks").join("active"), &mut inventory.active),
        (root.join("_tasks").join("pending"), &mut inventory.pending),
        (
            root.join("_tasks").join("completed"),
            &mut inventory.completed,
        ),
        (
            root.join("_tasks").join("postponed"),
            &mut inventory.postponed,
        ),
    ];

    let mut seen_numbers = HashSet::new();

    for (dir, vec) in dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.is_file() || !path.extension().map(|e| e == "md").unwrap_or(false) {
                    continue;
                }
                let fname = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let (num, title) = parse_task_filename(&fname);
                seen_numbers.insert(num);
                vec.push(TaskFile {
                    path: path.display().to_string(),
                    number: num,
                    title,
                });
            }
        }
    }

    // Compute next available number
    let mut n = 1;
    while seen_numbers.contains(&n) {
        n += 1;
    }
    inventory.next_number = n;

    inventory
}

fn parse_task_filename(fname: &str) -> (u32, String) {
    // Expected: "NNN_Title_Words.md" or "TNNN_Title_Words.md"
    let mut parts = fname.split('_');
    let first = parts.next().unwrap_or("0");
    let num: u32 = first.trim_start_matches('T').parse().unwrap_or(0);
    let title = parts.collect::<Vec<_>>().join(" ");
    (num, title)
}

/// Create a new task file in the specified folder.
pub(crate) fn create_task(
    root: &Path,
    folder: &str,
    number: u32,
    title: &str,
    content: &str,
) -> Result<PathBuf> {
    let dir = root.join("_tasks").join(folder);
    std::fs::create_dir_all(&dir)?;
    let filename = format!("{:03}_{}.md", number, title.replace(' ', "_"));
    let path = dir.join(&filename);
    let full_content = format!(
        "# Task {:03}: {}\n\n## Status\n{}\n\n## Objective\n{}\n",
        number, title, folder, content
    );
    std::fs::write(&path, full_content)?;
    Ok(path)
}

/// Move a task file from one folder to another.
pub(crate) fn move_task(
    root: &Path,
    number: u32,
    from: &str,
    to: &str,
    note: &str,
) -> Result<PathBuf> {
    let inventory = scan_task_inventory(root);
    let target = inventory
        .active
        .iter()
        .chain(&inventory.pending)
        .chain(&inventory.completed)
        .chain(&inventory.postponed)
        .find(|t| t.number == number)
        .cloned();

    let Some(task) = target else {
        anyhow::bail!("Task {} not found", number);
    };

    let src = PathBuf::from(&task.path);
    let dst_dir = root.join("_tasks").join(to);
    std::fs::create_dir_all(&dst_dir)?;
    let dst = dst_dir.join(src.file_name().unwrap_or_default());
    std::fs::rename(&src, &dst)?;

    // Append history note
    if !note.is_empty() {
        let mut content = std::fs::read_to_string(&dst).unwrap_or_default();
        content.push_str(&format!("\n\n## History Note\n{note}\n"));
        std::fs::write(&dst, content)?;
    }

    Ok(dst)
}

/// Supersede a task with a replacement target.
pub(crate) fn supersede_task(
    root: &Path,
    number: u32,
    replacement_number: u32,
    reason: &str,
) -> Result<PathBuf> {
    let inventory = scan_task_inventory(root);
    let target = inventory
        .active
        .iter()
        .chain(&inventory.pending)
        .chain(&inventory.completed)
        .chain(&inventory.postponed)
        .find(|t| t.number == number)
        .cloned();

    let Some(task) = target else {
        anyhow::bail!("Task {} not found", number);
    };

    let src = PathBuf::from(&task.path);
    let dst_dir = root.join("_tasks").join("postponed");
    std::fs::create_dir_all(&dst_dir)?;
    let dst = dst_dir.join(src.file_name().unwrap_or_default());
    std::fs::rename(&src, &dst)?;

    let mut content = std::fs::read_to_string(&dst).unwrap_or_default();
    content.push_str(&format!(
        "\n\n## Superseded\nReplaced by task {}. Reason: {}\n",
        replacement_number, reason
    ));
    std::fs::write(&dst, content)?;

    Ok(dst)
}

pub(crate) fn render_inventory_summary(inventory: &TaskInventory) -> String {
    let mut lines = Vec::new();
    lines.push("TASK INVENTORY".to_string());
    lines.push(format!("Active:     {}", inventory.active.len()));
    lines.push(format!("Pending:    {}", inventory.pending.len()));
    lines.push(format!("Completed:  {}", inventory.completed.len()));
    lines.push(format!("Postponed:  {}", inventory.postponed.len()));
    lines.push(format!("Next number: {}", inventory.next_number));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_task_filename_extracts_number_and_title() {
        let (num, title) = parse_task_filename("023_Expert_Responder");
        assert_eq!(num, 23);
        assert_eq!(title, "Expert Responder");
    }

    #[test]
    fn parse_troubleshooting_prefix() {
        let (num, title) = parse_task_filename("T023_Crash_Fix");
        assert_eq!(num, 23);
        assert_eq!(title, "Crash Fix");
    }

    #[test]
    fn next_number_allocates_correctly() {
        let temp = std::env::temp_dir().join(format!(
            "elma_steward_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp.join("_tasks").join("active")).unwrap();
        std::fs::create_dir_all(&temp.join("_tasks").join("pending")).unwrap();
        std::fs::write(
            temp.join("_tasks").join("active").join("001_First.md"),
            "# 1",
        )
        .unwrap();
        std::fs::write(
            temp.join("_tasks").join("pending").join("003_Third.md"),
            "# 3",
        )
        .unwrap();

        let inventory = scan_task_inventory(&temp);
        assert_eq!(inventory.next_number, 2);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn create_and_move_task_roundtrip() {
        let temp = std::env::temp_dir().join(format!(
            "elma_steward_roundtrip_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp.join("_tasks").join("active")).unwrap();

        let path = create_task(&temp, "active", 42, "Test Task", "Do something").unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("042_Test_Task"));

        let dst = move_task(&temp, 42, "active", "completed", "Done").unwrap();
        assert!(dst.exists());
        assert!(!path.exists());

        let content = std::fs::read_to_string(&dst).unwrap();
        assert!(content.contains("History Note"));
        assert!(content.contains("Done"));

        let _ = std::fs::remove_dir_all(&temp);
    }
}
