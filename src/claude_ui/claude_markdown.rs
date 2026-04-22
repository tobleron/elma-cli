//! @efficiency-role: ui-component
//!
//! Claude Code-style Terminal Markdown Renderer
//!
//! Renders markdown using Ratatui Line/Span structures for the Pink/Cyan theme.

use crate::ui_theme::*;
use ratatui::prelude::*;

pub(crate) fn render_markdown_ratatui(text: &str) -> Vec<Line<'static>> {
    let theme = current_theme();
    let all_lines: Vec<&str> = text.lines().collect();
    let mut output_lines = Vec::new();
    let mut i = 0;

    while i < all_lines.len() {
        let line = all_lines[i];

        // Check if this is the start of a table
        if line.trim().starts_with('|') && is_table_row(line) {
            let table_start = i;
            let mut table_end = i;
            while table_end < all_lines.len() && is_table_row(all_lines[table_end]) {
                table_end += 1;
            }
            // Render table
            let table_lines = render_table(&all_lines[table_start..table_end], theme);
            output_lines.extend(table_lines);
            i = table_end;
        } else {
            // Render normal line
            output_lines.extend(render_single_line(line, theme));
            i += 1;
        }
    }

    output_lines
}

fn is_table_row(line: &str) -> bool {
    line.trim().starts_with('|') && line.trim().ends_with('|') && line.contains('|')
}

fn render_table(table_lines: &[&str], theme: &crate::ui_theme::Theme) -> Vec<Line<'static>> {
    if table_lines.is_empty() {
        return Vec::new();
    }

    // Parse rows
    let rows: Vec<Vec<String>> = table_lines
        .iter()
        .map(|line| {
            line.trim()
                .trim_start_matches('|')
                .trim_end_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    if rows.is_empty() || rows[0].is_empty() {
        return Vec::new();
    }

    // Calculate column widths
    let num_cols = rows[0].len();
    let mut col_widths = vec![0; num_cols];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    // Render rows
    let mut lines = Vec::new();
    for (row_idx, row) in rows.iter().enumerate() {
        let mut spans = Vec::new();
        for (col_idx, cell) in row.iter().enumerate() {
            if col_idx < num_cols {
                let width = col_widths[col_idx];
                let padded = format!(" {:<width$} ", cell, width = width);
                spans.push(Span::styled(
                    padded,
                    Style::default().fg(theme.fg.to_ratatui_color()),
                ));
                if col_idx < num_cols - 1 {
                    spans.push(Span::styled(
                        "|",
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    lines
}

fn render_single_line(line: &str, theme: &crate::ui_theme::Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let trimmed = line.trim();
    if trimmed.starts_with("```") {
        // Skip
        return vec![];
    }

    if trimmed.is_empty() {
        lines.push(Line::default());
        return lines;
    }

    if let Some(rest) = trimmed.strip_prefix("# ") {
        lines.push(Line::from(vec![Span::styled(
            rest.to_string(),
            Style::default()
                .fg(theme.fg.to_ratatui_color())
                .add_modifier(Modifier::BOLD),
        )]));
        return lines;
    }

    if let Some(rest) = trimmed.strip_prefix("## ") {
        lines.push(Line::from(vec![Span::styled(
            rest.to_string(),
            Style::default()
                .fg(theme.fg.to_ratatui_color())
                .add_modifier(Modifier::BOLD),
        )]));
        return lines;
    }

    if let Some(rest) = trimmed.strip_prefix("- ") {
        lines.push(Line::from(vec![
            Span::styled(
                "• ",
                Style::default().fg(theme.accent_secondary.to_ratatui_color()),
            ),
            Span::styled(
                rest.to_string(),
                Style::default().fg(theme.fg.to_ratatui_color()),
            ),
        ]));
        return lines;
    }

    if let Some(rest) = trimmed.strip_prefix("> ") {
        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(theme.fg_dim.to_ratatui_color())),
            Span::styled(
                rest.to_string(),
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            ),
        ]));
        return lines;
    }

    if trimmed == "---" {
        lines.push(Line::from(vec![Span::styled(
            "────────────────────────",
            Style::default().fg(theme.fg_dim.to_ratatui_color()),
        )]));
        return lines;
    }

    // Inline markdown (bold + inline code) for common answer formatting
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut chars = line.chars().peekable();
    let mut buffer = String::new();
    let mut bold = false;
    let mut code = false;
    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() == Some(&'*') {
            chars.next();
            if !buffer.is_empty() {
                let style = if code {
                    Style::default()
                        .fg(theme.accent_secondary.to_ratatui_color())
                        .add_modifier(Modifier::DIM)
                } else if bold {
                    Style::default()
                        .fg(theme.fg.to_ratatui_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg.to_ratatui_color())
                };
                spans.push(Span::styled(buffer.clone(), style));
                buffer.clear();
            }
            bold = !bold;
            continue;
        }
        if ch == '`' {
            if !buffer.is_empty() {
                let style = if code {
                    Style::default()
                        .fg(theme.accent_secondary.to_ratatui_color())
                        .add_modifier(Modifier::DIM)
                } else if bold {
                    Style::default()
                        .fg(theme.fg.to_ratatui_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg.to_ratatui_color())
                };
                spans.push(Span::styled(buffer.clone(), style));
                buffer.clear();
            }
            code = !code;
            continue;
        }
        buffer.push(ch);
    }

    if !buffer.is_empty() {
        let style = if code {
            Style::default()
                .fg(theme.accent_secondary.to_ratatui_color())
                .add_modifier(Modifier::DIM)
        } else if bold {
            Style::default()
                .fg(theme.fg.to_ratatui_color())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg.to_ratatui_color())
        };
        spans.push(Span::styled(buffer, style));
    }

    lines.push(Line::from(spans));
    lines
}
