//! @efficiency-role: ui-component
//!
//! Claude Code-style Task List
//!
//! Implements Todo tool integration:
//! - Show tasks with ○/◐/✓ status
//! - Recent completion fade (30s)
//! - Press ctrl+t to toggle

use crate::ui_theme::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

// ============================================================================
// Task Status
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

// ============================================================================
// Task Item
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) struct TaskItem {
    pub id: u32,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

impl TaskItem {
    pub(crate) fn new(id: u32, description: String) -> Self {
        Self {
            id,
            description,
            status: TaskStatus::Pending,
            created_at: 0,
            completed_at: None,
        }
    }

    pub(crate) fn to_line(&self) -> String {
        let symbol = match self.status {
            TaskStatus::Pending => dim("◻"),
            TaskStatus::InProgress => elma_accent("◼"),
            TaskStatus::Completed => success_green("✔"),
            TaskStatus::Blocked => dim("▸"),
        };
        format!("{}. {} {}", self.id, symbol, &self.description)
    }
}

// ============================================================================
// Task List
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) struct TaskList {
    pub tasks: Vec<TaskItem>,
    pub visible: bool,
    pub max_visible: usize,
}

impl TaskList {
    pub(crate) fn new() -> Self {
        Self {
            tasks: Vec::new(),
            visible: false,
            max_visible: 10,
        }
    }

    pub(crate) fn push(&mut self, description: String) -> u32 {
        let id = self.tasks.len() as u32 + 1;
        self.tasks.push(TaskItem::new(id, description));
        id
    }

    pub(crate) fn complete(&mut self, id: u32) {
        if let Some(task) = self.tasks.get_mut((id - 1) as usize) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            );
        }
    }

    pub(crate) fn start(&mut self, id: u32) {
        if let Some(task) = self.tasks.get_mut((id - 1) as usize) {
            task.status = TaskStatus::InProgress;
        }
    }

    pub(crate) fn block(&mut self, id: u32, reason: Option<String>) {
        if let Some(task) = self.tasks.get_mut((id - 1) as usize) {
            task.status = TaskStatus::Blocked;
            if let Some(reason) = reason {
                task.description = format!("{} ({})", task.description, reason);
            }
        }
    }

    pub(crate) fn update_text(&mut self, id: u32, text: String) {
        if let Some(task) = self.tasks.get_mut((id - 1) as usize) {
            task.description = text;
        }
    }

    pub(crate) fn remove(&mut self, id: u32) -> bool {
        if id == 0 || id as usize > self.tasks.len() {
            return false;
        }
        self.tasks.remove((id - 1) as usize);
        for (idx, t) in self.tasks.iter_mut().enumerate() {
            t.id = (idx + 1) as u32;
        }
        true
    }

    pub(crate) fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub(crate) fn show(&mut self) {
        self.visible = true;
    }

    pub(crate) fn hide(&mut self) {
        self.visible = false;
    }

    pub(crate) fn render(&self) -> Vec<String> {
        if !self.visible || self.tasks.is_empty() {
            return vec![];
        }

        let (visible_tasks, hidden_count) = self.visible_tasks_with_hidden();

        if visible_tasks.is_empty() && hidden_count == 0 {
            return vec![];
        }

        let done = visible_tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let mut lines = vec![format!(
            "{} {}/{}",
            elma_accent("●"),
            done,
            visible_tasks.len()
        )];
        lines.extend(visible_tasks.iter().map(|t| t.to_line()));
        if hidden_count > 0 {
            lines.push(dim(&format!("  … +{}", hidden_count)));
        }
        lines
    }

    pub(crate) fn render_ratatui(&self) -> Vec<Line<'static>> {
        if !self.visible || self.tasks.is_empty() {
            return vec![];
        }

        let theme = current_theme();
        let (visible_tasks, hidden_count) = self.visible_tasks_with_hidden();

        if visible_tasks.is_empty() && hidden_count == 0 {
            return vec![];
        }

        let total = self.tasks.len();
        let done = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let in_progress = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::InProgress)
            .count();
        let open = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count();
        let blocked = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Blocked)
            .count();

        let header_text = format!("● {}/{}", done, total);
        let mut lines = vec![Line::from(vec![
            Span::styled(
                header_text,
                Style::default()
                    .fg(theme.accent_primary.to_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if in_progress > 0 {
                    format!("  {} in progress", in_progress)
                } else {
                    String::new()
                },
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            ),
        ])];

        for task in &visible_tasks {
            let (symbol, symbol_style) = match task.status {
                TaskStatus::Pending => ("◻ ", Style::default().fg(theme.fg_dim.to_ratatui_color())),
                TaskStatus::InProgress => (
                    "◼ ",
                    Style::default()
                        .fg(theme.accent_primary.to_ratatui_color())
                        .add_modifier(Modifier::BOLD),
                ),
                TaskStatus::Completed => (
                    "✔ ",
                    Style::default()
                        .fg(theme.success.to_ratatui_color())
                        .add_modifier(Modifier::DIM),
                ),
                TaskStatus::Blocked => ("▸ ", Style::default().fg(theme.fg_dim.to_ratatui_color())),
            };

            let text_style = match task.status {
                TaskStatus::Completed => Style::default()
                    .fg(theme.fg_dim.to_ratatui_color())
                    .add_modifier(Modifier::DIM | Modifier::CROSSED_OUT),
                TaskStatus::InProgress => Style::default()
                    .fg(theme.fg.to_ratatui_color())
                    .add_modifier(Modifier::BOLD),
                TaskStatus::Blocked => Style::default()
                    .fg(theme.fg_dim.to_ratatui_color())
                    .add_modifier(Modifier::ITALIC),
                TaskStatus::Pending => Style::default().fg(theme.fg.to_ratatui_color()),
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(symbol, symbol_style),
                Span::styled(format!("{}. {}", task.id, task.description), text_style),
            ]));
        }

        if hidden_count > 0 {
            lines.push(Line::from(Span::styled(
                format!("  … +{}", hidden_count),
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            )));
        }

        lines
    }

    pub(crate) fn visible_tasks_with_hidden(&self) -> (Vec<&TaskItem>, usize) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let filtered: Vec<&TaskItem> = self
            .tasks
            .iter()
            .filter(|t| {
                if t.status == TaskStatus::Completed {
                    if let Some(completed) = t.completed_at {
                        now.saturating_sub(completed) < 30000
                    } else {
                        true
                    }
                } else {
                    true
                }
            })
            .collect();

        let total = filtered.len();
        let visible = filtered
            .into_iter()
            .take(self.max_visible)
            .collect::<Vec<_>>();
        let hidden = total.saturating_sub(visible.len());
        (visible, hidden)
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let mut list = TaskList::new();
        let id = list.push("Test task".to_string());
        assert_eq!(id, 1);
        assert_eq!(list.tasks.len(), 1);
    }

    #[test]
    fn test_task_complete() {
        let mut list = TaskList::new();
        list.push("Test".to_string());
        list.complete(1);
        assert_eq!(list.tasks[0].status, TaskStatus::Completed);
    }

    #[test]
    fn test_task_toggle() {
        let mut list = TaskList::new();
        assert!(!list.visible);
        list.toggle();
        assert!(list.visible);
    }

    #[test]
    fn test_task_render() {
        let mut list = TaskList::new();
        list.push("Task 1".to_string());
        list.push("Task 2".to_string());
        list.toggle(); // Make visible
        let lines = list.render();
        assert!(!lines.is_empty());
    }
}
