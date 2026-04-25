//! @efficiency-role: service-orchestrator
//!
//! Project guidance discovery and prompt snapshotting.

use crate::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct GuidanceSnapshot {
    pub(crate) agents_md: Option<String>,
    pub(crate) tasks_md: Option<String>,
    pub(crate) active_master_path: Option<String>,
    pub(crate) active_master_summary: Option<String>,
}

impl GuidanceSnapshot {
    pub(crate) fn render_for_system_prompt(&self) -> String {
        let mut sections = Vec::new();
        if let Some(agents) = &self.agents_md {
            sections.push(format!("AGENTS.md:\n{}", trim_guidance(agents, 1600)));
        }
        if let Some(tasks) = &self.tasks_md {
            sections.push(format!("_tasks/TASKS.md:\n{}", trim_guidance(tasks, 1200)));
        }
        if let (Some(path), Some(summary)) = (&self.active_master_path, &self.active_master_summary)
        {
            sections.push(format!(
                "Active master task ({path}):\n{}",
                trim_guidance(summary, 1200)
            ));
        }
        sections.join("\n\n")
    }
}

fn trim_guidance(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(max_chars).collect();
    format!("{head}\n...[truncated]")
}

pub(crate) fn load_project_guidance(root: &Path) -> GuidanceSnapshot {
    let agents_path = root.join("AGENTS.md");
    let tasks_path = root.join("_tasks").join("TASKS.md");
    let active_dir = root.join("_tasks").join("active");

    let active_master = std::fs::read_dir(&active_dir)
        .ok()
        .into_iter()
        .flat_map(|it| it.filter_map(|e| e.ok()))
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains("Master_Plan") || n.contains("MasterPlan"))
                .unwrap_or(false)
        });

    GuidanceSnapshot {
        agents_md: std::fs::read_to_string(&agents_path).ok(),
        tasks_md: std::fs::read_to_string(&tasks_path).ok(),
        active_master_path: active_master
            .as_ref()
            .and_then(|p| p.strip_prefix(root).ok())
            .map(|p| p.display().to_string()),
        active_master_summary: active_master
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok()),
    }
}

pub(crate) fn persist_guidance_snapshot(
    args: &Args,
    session: &SessionPaths,
    guidance: &GuidanceSnapshot,
) -> Result<()> {
    let rendered = guidance.render_for_system_prompt();
    if rendered.is_empty() {
        return Ok(());
    }
    let path = session.root.join("project_guidance.txt");
    std::fs::write(&path, rendered).with_context(|| format!("write {}", path.display()))?;
    trace(args, &format!("project_guidance_saved={}", path.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_for_system_prompt_includes_available_sections() {
        let snapshot = GuidanceSnapshot {
            agents_md: Some("agent rules".to_string()),
            tasks_md: Some("task index".to_string()),
            active_master_path: Some("_tasks/active/001.md".to_string()),
            active_master_summary: Some("master summary".to_string()),
        };
        let rendered = snapshot.render_for_system_prompt();
        assert!(rendered.contains("AGENTS.md"));
        assert!(rendered.contains("_tasks/TASKS.md"));
        assert!(rendered.contains("master summary"));
    }
}
