//! @efficiency-role: ui-component
//!
//! Structured Diff Engine — Side-by-side file comparison viewer.
//!
//! Provides rich terminal-based diff rendering matching Claude Code's StructuredDiff.

use crate::ui_theme::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use similar::{ChangeTag, TextDiff};

/// Represents a single diff hunk with line numbers and changes
#[derive(Clone, Debug)]
pub struct DiffHunk {
    pub old_start: usize,
    pub new_start: usize,
    pub lines: Vec<DiffLine>,
}

/// A single line in the diff
#[derive(Clone, Debug)]
pub struct DiffLine {
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub tag: ChangeTag,
    pub content: String,
}

/// Structured diff viewer widget
pub struct StructuredDiff {
    pub old_path: String,
    pub new_path: String,
    pub hunks: Vec<DiffHunk>,
}

impl StructuredDiff {
    /// Create a new structured diff from old and new content
    pub fn new(old_path: &str, new_path: &str, old_content: &str, new_content: &str) -> Self {
        let diff = TextDiff::from_lines(old_content, new_content);
        let mut hunks = Vec::new();

        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            let mut lines = Vec::new();
            for change in hunk.iter_changes() {
                let tag = change.tag();
                let content = change.value().trim_end_matches('\n').to_string();
                let old_line = if tag == ChangeTag::Delete || tag == ChangeTag::Equal {
                    Some(change.old_index().unwrap_or(0) + 1)
                } else {
                    None
                };
                let new_line = if tag == ChangeTag::Insert || tag == ChangeTag::Equal {
                    Some(change.new_index().unwrap_or(0) + 1)
                } else {
                    None
                };
                lines.push(DiffLine {
                    old_line,
                    new_line,
                    tag,
                    content,
                });
            }
            if !lines.is_empty() {
                let old_start = lines.iter().find_map(|l| l.old_line).unwrap_or(1);
                let new_start = lines.iter().find_map(|l| l.new_line).unwrap_or(1);
                hunks.push(DiffHunk {
                    old_start,
                    new_start,
                    lines,
                });
            }
        }

        Self {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
            hunks,
        }
    }

    /// Render the diff to ratatui lines
    pub fn render_ratatui(&self, width: usize) -> Vec<Line<'static>> {
        let theme = current_theme();
        let mut lines = Vec::new();

        // Header
        lines.push(Line::from(vec![
            Span::styled(
                "Diff: ",
                Style::default()
                    .fg(theme.fg.to_ratatui_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                self.old_path.clone(),
                Style::default().fg(theme.accent_primary.to_ratatui_color()),
            ),
            Span::raw(" → "),
            Span::styled(
                self.new_path.clone(),
                Style::default().fg(theme.accent_primary.to_ratatui_color()),
            ),
        ]));
        lines.push(Line::from(""));

        for hunk in &self.hunks {
            // Hunk header
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start,
                    hunk.lines
                        .iter()
                        .filter(|l| l.tag == ChangeTag::Delete || l.tag == ChangeTag::Equal)
                        .count(),
                    hunk.new_start,
                    hunk.lines
                        .iter()
                        .filter(|l| l.tag == ChangeTag::Insert || l.tag == ChangeTag::Equal)
                        .count(),
                ),
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            )]));

            for line in &hunk.lines {
                let prefix = match line.tag {
                    ChangeTag::Equal => " ",
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                };
                let color = match line.tag {
                    ChangeTag::Equal => theme.fg_dim.to_ratatui_color(),
                    ChangeTag::Delete => theme.error.to_ratatui_color(),
                    ChangeTag::Insert => theme.success.to_ratatui_color(),
                };
                let line_num = match line.tag {
                    ChangeTag::Equal | ChangeTag::Delete => line
                        .old_line
                        .map(|n| format!("{:4}", n))
                        .unwrap_or("    ".to_string()),
                    ChangeTag::Insert => line
                        .new_line
                        .map(|n| format!("{:4}", n))
                        .unwrap_or("    ".to_string()),
                };
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(color)),
                    Span::styled(
                        line_num,
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ),
                    Span::raw(" "),
                    Span::styled(line.content.clone(), Style::default().fg(color)),
                ]));
            }
            lines.push(Line::from(""));
        }

        lines
    }

    /// Render as a ratatui widget
    pub fn widget(&self) -> Paragraph<'static> {
        let lines = self.render_ratatui(80); // Default width
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Structured Diff"),
            )
            .wrap(Wrap { trim: false })
    }
}
