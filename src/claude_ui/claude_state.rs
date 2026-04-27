//! @efficiency-role: ui-component
//!
//! Claude Code-style UI State
//!
//! Implements the sparse transcript model for Claude Code parity:
//! - User messages: "> " prefix
//! - Assistant messages: "● " prefix
//! - Thinking: "∴ Thinking" (collapsed) or full content (expanded/transcript mode)
//! - Tool start: "▸ " prefix
//! - Tool result: "✓" (success) or "✗" (failure)
//! - Compact boundary: "✻ Conversation compacted"

use crate::claude_ui::{render_assistant_content, render_markdown_ratatui, AssistantContent};
use crate::ui_theme::*;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::{Duration, Instant};

const TELEMETRY_COLLAPSE_DELAY: Duration = Duration::from_secs(10);

// ============================================================================
// Message Types (Claude Code-style)
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) enum ToolTraceStatus {
    Running,
    Completed {
        success: bool,
        output: String,
        duration_ms: Option<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UiNoticeKind {
    Budget,
    Queue,
    Compaction,
    StopReason,
    InputHint,
    Session,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NoticePersistence {
    TranscriptPersistent,
    TranscriptCollapsible,
    EphemeralPromptHint,
}

#[derive(Clone, Debug)]
pub(crate) struct UiNotice {
    pub kind: UiNoticeKind,
    pub content: String,
    pub created_at: Instant,
    pub persistence: NoticePersistence,
    pub collapsed: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum ClaudeMessage {
    User {
        content: String,
    },
    Assistant {
        content: AssistantContent,
    },
    ToolStart {
        name: String,
        input: Option<String>,
    },
    ToolProgress {
        name: String,
        message: String,
    },
    ToolResult {
        name: String,
        success: bool,
        output: String,
        duration_ms: Option<u64>,
    },
    /// Unified tool trace — replaces separate ToolStart/ToolResult/ToolProgress.
    /// Shows command + live status indicator (spinner → checkmark/cross).
    /// `collapsed` hides output; auto-set when a newer tool starts.
    ToolTrace {
        name: String,
        command: String,
        status: ToolTraceStatus,
        collapsed: bool,
    },
    PermissionRequest {
        command: String,
        reason: Option<String>,
    },
    Thinking {
        content: String,
        is_streaming: bool,
        word_count: usize,
    },
    CompactBoundary,
    CompactSummary {
        message_count: usize,
        context_preview: Option<String>,
    },
    System {
        content: String,
    },
    Notice(UiNotice),
}

impl ClaudeMessage {
    pub(crate) fn to_ratatui_lines(&self, expanded: bool, width: usize) -> Vec<Line<'static>> {
        let theme = current_theme();
        match self {
            ClaudeMessage::User { content } => {
                // User messages: left gutter with "❯" indicator (Claude Code style)
                let content_str = if content.is_empty() {
                    String::new()
                } else if content.len() > 10000 {
                    let head = if content.is_char_boundary(2500) {
                        content[..2500].to_string()
                    } else {
                        let mut end = 2500;
                        while !content.is_char_boundary(end) && end > 0 {
                            end -= 1;
                        }
                        content[..end].to_string()
                    };
                    let tail_start = if content.len() >= 2500 {
                        let pos = content.len() - 2500;
                        if content.is_char_boundary(pos) {
                            pos
                        } else {
                            let mut start = pos;
                            while !content.is_char_boundary(start) {
                                start += 1;
                            }
                            start
                        }
                    } else {
                        0
                    };
                    let tail = content[tail_start..].to_string();
                    let skipped_lines = content[2500..tail_start].lines().count();
                    format!("{}\n… +{} lines …\n{}", head, skipped_lines, tail)
                } else {
                    content.clone()
                };
                vec![Line::from(vec![
                    Span::styled(
                        "❯",
                        Style::default()
                            .fg(theme.accent_primary.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::raw(content_str),
                ])]
            }
            ClaudeMessage::Assistant { content } => {
                let content_width = width.saturating_sub(2).max(12);
                let content_lines = render_assistant_content(content, content_width);
                let mut lines = Vec::new();
                for (i, content_line) in content_lines.into_iter().enumerate() {
                    if i == 0 {
                        let mut spans = vec![Span::styled(
                            ASSISTANT_DOT,
                            Style::default()
                                .fg(theme.accent_primary.to_ratatui_color())
                                .add_modifier(Modifier::BOLD),
                        )];
                        spans.push(Span::raw(" ")); // gutter gap
                        spans.extend(content_line.spans.into_iter());
                        lines.push(Line::from(spans));
                    } else {
                        let mut spans = vec![Span::raw("  ")]; // 2-char gutter
                        spans.extend(content_line.spans.into_iter());
                        lines.push(Line::from(spans));
                    }
                }
                lines
            }
            ClaudeMessage::Thinking {
                content,
                is_streaming,
                word_count,
            } => {
                let delay_secs = (*word_count as f64 / 300.0 * 60.0).clamp(3.0, 60.0) as u64;
                let time_label = if delay_secs >= 60 {
                    format!("{}m {}s", delay_secs / 60, delay_secs % 60)
                } else {
                    format!("{}s", delay_secs)
                };
                if *is_streaming {
                    let mut lines = vec![Line::from(vec![
                        Span::styled(
                            "∴",
                            Style::default()
                                .fg(theme.accent_primary.to_ratatui_color())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("Thinking... [{}]", time_label),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ])];
                    for line in content.lines() {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                line.to_string(),
                                Style::default().fg(theme.fg_dim.to_ratatui_color()),
                            ),
                        ]));
                    }
                    lines
                } else if expanded {
                    let mut lines = Vec::new();
                    lines.push(Line::from(vec![
                        Span::styled(
                            "⌄",
                            Style::default().fg(theme.accent_primary.to_ratatui_color()),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("Thinking [{}]", time_label),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                    for line in content.lines() {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                line.to_string(),
                                Style::default().fg(theme.fg_dim.to_ratatui_color()),
                            ),
                        ]));
                    }
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            "(ctrl+o to collapse)",
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                    lines
                } else {
                    vec![Line::from(vec![
                        Span::styled(
                            ">",
                            Style::default().fg(theme.accent_primary.to_ratatui_color()),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("Thinking.. [{}]", time_label),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ])]
                }
            }
            ClaudeMessage::ToolStart { name, input } => {
                let mut spans = vec![
                    Span::styled(
                        "▶",
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                ];
                if let Some(inp) = input {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        format!("({})", inp),
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ));
                }
                vec![Line::from(spans)]
            }
            ClaudeMessage::ToolProgress { name, message } => {
                vec![Line::from(vec![
                    Span::styled(
                        "◐",
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        name.clone(),
                        Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                    ),
                    Span::raw(": "),
                    Span::styled(
                        message.clone(),
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ),
                ])]
            }
            ClaudeMessage::PermissionRequest { command, reason } => {
                let mut spans = vec![
                    Span::styled(
                        "● ",
                        Style::default().fg(theme.accent_primary.to_ratatui_color()),
                    ),
                    Span::styled("Allow ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        command.clone(),
                        Style::default().add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled("? ", Style::default()),
                    Span::styled(
                        "[y/n]",
                        Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                    ),
                ];
                if let Some(r) = reason {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(
                        r.clone(),
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ));
                }
                vec![Line::from(spans)]
            }
            ClaudeMessage::ToolResult {
                name,
                success,
                output,
                duration_ms,
            } => {
                let symbol = if *success { CHECK } else { CROSS };
                let symbol_style = if *success {
                    Style::default()
                        .fg(theme.success.to_ratatui_color())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(theme.error.to_ratatui_color())
                        .add_modifier(Modifier::BOLD)
                };

                let first_line = if let Some(ms) = duration_ms {
                    let duration = if *ms > 1000 {
                        format!("{:.1}s", *ms as f64 / 1000.0)
                    } else {
                        format!("{}ms", ms)
                    };
                    Line::from(vec![
                        Span::styled(symbol, symbol_style),
                        Span::raw(" "),
                        Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", duration),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(symbol, symbol_style),
                        Span::raw(" "),
                        Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                    ])
                };

                let mut lines = vec![first_line];
                let output_lines: Vec<&str> = output.lines().collect();
                let max_lines = if expanded { output_lines.len() } else { 8 };
                for line in output_lines.iter().take(max_lines) {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            line.to_string(),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                }
                if output_lines.len() > max_lines {
                    let remaining = output_lines.len() - max_lines;
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            format!("+{} lines", remaining),
                            Style::default()
                                .fg(theme.fg_dim.to_ratatui_color())
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
                }
                lines
            }
            ClaudeMessage::ToolTrace {
                name,
                command,
                status,
                collapsed,
            } => {
                let (symbol, symbol_style) = match status {
                    ToolTraceStatus::Running => (
                        "◐",
                        Style::default()
                            .fg(theme.warning.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    ToolTraceStatus::Completed { success: true, .. } => (
                        "✓",
                        Style::default()
                            .fg(theme.success.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    ToolTraceStatus::Completed { success: false, .. } => (
                        "✗",
                        Style::default()
                            .fg(theme.error.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                };

                let is_expanded = expanded || !*collapsed;
                let chevron = if is_expanded { "▾" } else { "▸" };

                let mut lines = vec![Line::from(vec![
                    Span::styled(
                        chevron,
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ),
                    Span::raw(" "),
                    Span::styled(symbol, symbol_style),
                    Span::raw(" "),
                    Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(
                        command.clone(),
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ),
                ])];

                if let ToolTraceStatus::Completed {
                    success,
                    output,
                    duration_ms,
                } = status
                {
                    let output_line_count = output.lines().count();
                    if output_line_count > 0 {
                        lines[0].spans.push(Span::raw(" "));
                        lines[0].spans.push(Span::styled(
                            format!("[{} lines]", output_line_count),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ));
                    }
                    if let Some(ms) = duration_ms {
                        let duration = if *ms > 1000 {
                            format!("{:.1}s", *ms as f64 / 1000.0)
                        } else {
                            format!("{}ms", ms)
                        };
                        lines[0].spans.push(Span::raw(" "));
                        lines[0].spans.push(Span::styled(
                            format!("({})", duration),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ));
                    }

                    if is_expanded {
                        let output_lines: Vec<&str> = output.lines().collect();
                        let max_lines = if *success {
                            if expanded {
                                output_lines.len()
                            } else {
                                8
                            }
                        } else {
                            output_lines.len()
                        };
                        for line in output_lines.iter().take(max_lines) {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    line.to_string(),
                                    if *success {
                                        Style::default().fg(theme.fg_dim.to_ratatui_color())
                                    } else {
                                        Style::default().fg(theme.error.to_ratatui_color())
                                    },
                                ),
                            ]));
                        }
                        if output_lines.len() > max_lines {
                            let remaining = output_lines.len() - max_lines;
                            lines.push(Line::from(vec![
                                Span::raw("    "),
                                Span::styled(
                                    format!("+{} lines", remaining),
                                    Style::default()
                                        .fg(theme.fg_dim.to_ratatui_color())
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }
                    }
                }
                lines
            }
            ClaudeMessage::CompactBoundary => {
                if expanded {
                    vec![Line::from(vec![
                        Span::styled(
                            "✻ Conversation compacted ",
                            Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                        ),
                        Span::styled(
                            "(ctrl+o for history)",
                            Style::default()
                                .fg(theme.fg_dim.to_ratatui_color())
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ])]
                } else {
                    vec![]
                }
            }
            ClaudeMessage::CompactSummary {
                message_count,
                context_preview,
            } => {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{} ", ASSISTANT_DOT),
                            Style::default().fg(theme.fg.to_ratatui_color()),
                        ),
                        Span::raw("Summarized conversation"),
                    ]),
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            "Summarized ",
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                        Span::styled(
                            format!("{} messages", message_count),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]),
                ];
                if let Some(ctx) = context_preview {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            "Context:",
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                    for md_line in render_markdown_ratatui(ctx) {
                        let mut prefixed = vec![Span::raw("      ")];
                        prefixed.extend(md_line.spans);
                        lines.push(Line::from(prefixed));
                    }
                }
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        "ctrl+o to expand history",
                        Style::default()
                            .fg(theme.fg_dim.to_ratatui_color())
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
                lines
            }
            ClaudeMessage::System { content } => {
                vec![Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(theme.warning.to_ratatui_color())),
                    Span::styled(
                        content.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ])]
            }
            ClaudeMessage::Notice(notice) => {
                let kind_label = notice.kind.label();
                if notice.collapsed && !expanded {
                    vec![Line::from(vec![Span::styled(
                        format!("◦ {} ({})", kind_label, notice.content),
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    )])]
                } else {
                    vec![Line::from(vec![
                        Span::styled(
                            format!("◦ {}: ", kind_label),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                        Span::styled(
                            notice.content.clone(),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ])]
                }
            }
        }
    }

    pub(crate) fn to_lines(&self, expanded: bool) -> Vec<String> {
        // Fallback for non-ratatui usage if still needed
        match self {
            ClaudeMessage::User { content } => {
                let mut lines = vec![format!("> {}", content)];
                for line in content.lines().skip(1) {
                    lines.push(format!("  {}", line));
                }
                lines
            }
            ClaudeMessage::Assistant { content } => {
                let mut raw_lines = content.raw_markdown.lines();
                let first = raw_lines.next().unwrap_or_default();
                let mut lines = vec![format!("● {}", first)];
                for line in raw_lines {
                    lines.push(format!("  {}", line));
                }
                lines
            }
            ClaudeMessage::Thinking {
                content,
                is_streaming,
                word_count,
            } => {
                let delay_secs = (*word_count as f64 / 300.0 * 60.0).clamp(3.0, 60.0) as u64;
                let time_label = if delay_secs >= 60 {
                    format!("{}m {}s", delay_secs / 60, delay_secs % 60)
                } else {
                    format!("{}s", delay_secs)
                };
                if *is_streaming {
                    let mut lines = vec![format!(
                        "  {} {}",
                        accent_primary("∴"),
                        dim(&format!("Thinking... [{}]", time_label))
                    )];
                    for line in content.lines() {
                        lines.push(format!("    {}", dim(line)));
                    }
                    lines
                } else if expanded {
                    let mut lines = vec![format!(
                        "  {} {}",
                        accent_primary("⌄"),
                        dim(&format!("Thinking [{}]", time_label))
                    )];
                    for line in content.lines() {
                        lines.push(format!("    {}", dim(line)));
                    }
                    lines.push(format!("    {}", dim("(ctrl+o to collapse)")));
                    lines
                } else {
                    vec![format!(
                        "  {} {}",
                        accent_primary(">"),
                        dim(&format!("Thinking.. [{}]", time_label))
                    )]
                }
            }
            ClaudeMessage::ToolStart { name, input } => {
                if let Some(inp) = input {
                    vec![format!(
                        "  {} {} ({})",
                        EXPAND_ARROW_RIGHT,
                        bold(name),
                        dim(inp)
                    )]
                } else {
                    vec![format!("  {} {}", EXPAND_ARROW_RIGHT, bold(name))]
                }
            }
            ClaudeMessage::ToolProgress { name, message } => {
                vec![format!("  ● {}: {}", info_cyan(name), dim(message))]
            }
            ClaudeMessage::PermissionRequest { command, reason } => {
                let mut lines = vec![format!(
                    "{} {}",
                    elma_accent("●"),
                    bold("Permission required")
                )];
                if let Some(r) = reason {
                    lines.push(format!("  {}", dim(r)));
                }
                lines.push(format!("  {}", italic(command)));
                lines
            }
            ClaudeMessage::ToolResult {
                name,
                success,
                output,
                duration_ms,
            } => {
                let symbol = if *success { CHECK } else { CROSS };
                let mut lines = if let Some(ms) = duration_ms {
                    let duration = if *ms > 1000 {
                        format!("{:.1}s", *ms as f64 / 1000.0)
                    } else {
                        format!("{}ms", ms)
                    };
                    vec![format!(
                        "{} {} ({})",
                        info_cyan(symbol),
                        bold(name),
                        meta_comment(&duration)
                    )]
                } else {
                    vec![format!("{} {}", info_cyan(symbol), bold(name))]
                };
                let output_lines: Vec<&str> = output.lines().collect();
                let max_lines = if expanded { output_lines.len() } else { 8 };
                for line in output_lines.iter().take(max_lines) {
                    lines.push(format!("    {}", dim(line)));
                }
                if output_lines.len() > max_lines {
                    let remaining = output_lines.len() - max_lines;
                    let line = format!("    +{} lines", remaining);
                    lines.push(dim(&line));
                }
                lines
            }
            ClaudeMessage::ToolTrace {
                name,
                command,
                status,
                collapsed,
            } => {
                let symbol = match status {
                    ToolTraceStatus::Running => "◐",
                    ToolTraceStatus::Completed { success: true, .. } => "✓",
                    ToolTraceStatus::Completed { success: false, .. } => "✗",
                };
                let is_expanded = expanded || !*collapsed;
                let chevron = if is_expanded { "▾" } else { "▸" };
                let mut lines = vec![format!(
                    "{} {} {} {}",
                    dim(chevron),
                    info_cyan(symbol),
                    bold(name),
                    dim(command)
                )];
                if let ToolTraceStatus::Completed {
                    success,
                    output,
                    duration_ms,
                } = status
                {
                    if let Some(ms) = duration_ms {
                        let duration = if *ms > 1000 {
                            format!("{:.1}s", *ms as f64 / 1000.0)
                        } else {
                            format!("{}ms", ms)
                        };
                        lines[0] = format!(
                            "{} {} {} {} ({})",
                            dim(chevron),
                            info_cyan(symbol),
                            bold(name),
                            dim(command),
                            meta_comment(&duration)
                        );
                    }
                    if is_expanded {
                        let output_lines: Vec<&str> = output.lines().collect();
                        let max_lines = if *success {
                            if expanded {
                                output_lines.len()
                            } else {
                                8
                            }
                        } else {
                            output_lines.len()
                        };
                        for line in output_lines.iter().take(max_lines) {
                            lines.push(format!("    {}", dim(line)));
                        }
                        if output_lines.len() > max_lines {
                            let remaining = output_lines.len() - max_lines;
                            lines.push(dim(&format!("    +{} lines", remaining)));
                        }
                    }
                }
                lines
            }
            ClaudeMessage::CompactBoundary => {
                if expanded {
                    vec!["✻ compacted".to_string()]
                } else {
                    vec![]
                }
            }
            ClaudeMessage::CompactSummary {
                message_count,
                context_preview,
            } => {
                let mut lines = vec![
                    format!("{} Summarized conversation", ASSISTANT_DOT),
                    format!(
                        "    {} {} up to this point",
                        dim("Summarized"),
                        meta_comment(&format!("{} messages", message_count))
                    ),
                ];
                if let Some(ctx) = context_preview {
                    lines.push(dim("    Context:"));
                    for line in ctx.lines() {
                        lines.push(dim(&format!("      {}", line)));
                    }
                }
                lines.push(dim("ctrl+o to expand history"));
                lines
            }
            ClaudeMessage::System { content } => {
                vec![format!("{} {}", warn_yellow("⚠"), bold(content))]
            }
            ClaudeMessage::Notice(notice) => {
                let label = notice.kind.label();
                if notice.collapsed && !expanded {
                    vec![dim(&format!("◦ {} ({})", label, notice.content))]
                } else {
                    vec![dim(&format!("◦ {}: {}", label, notice.content))]
                }
            }
        }
    }
}

impl UiNoticeKind {
    fn label(&self) -> &'static str {
        match self {
            UiNoticeKind::Budget => "budget",
            UiNoticeKind::Queue => "queue",
            UiNoticeKind::Compaction => "compaction",
            UiNoticeKind::StopReason => "stop",
            UiNoticeKind::InputHint => "input",
            UiNoticeKind::Session => "session",
        }
    }
}

// ============================================================================
// Transcript (Claude Code-style)
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) struct ClaudeTranscript {
    pub messages: Vec<ClaudeMessage>,
    pub expanded: bool,
    pub scroll_offset: usize,
    pub live_thinking_index: Option<usize>,
    pub thinking_collapse_deadline: Option<(usize, Instant)>,
    /// Index of thinking message explicitly expanded via click
    pub thinking_expanded_index: Option<usize>,
    /// Index of the last message when user first scrolled up (for unseen divider)
    pub divider_index: Option<usize>,
    /// Y-position snapshot at first scroll-away
    pub divider_y: Option<usize>,
}

impl ClaudeTranscript {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push(&mut self, msg: ClaudeMessage) {
        // Auto-collapse previous completed tool traces when a new tool starts.
        if matches!(msg, ClaudeMessage::ToolTrace { .. }) {
            for existing in self.messages.iter_mut().rev() {
                if let ClaudeMessage::ToolTrace {
                    status, collapsed, ..
                } = existing
                {
                    if matches!(status, ToolTraceStatus::Completed { .. }) {
                        *collapsed = true;
                    }
                }
            }
        }
        self.messages.push(msg);
        // Only auto-scroll to bottom on conversational messages (user/assistant).
        // Tool output and thinking should not disrupt the user's scroll position.
        match self.messages.last() {
            Some(ClaudeMessage::User { .. } | ClaudeMessage::Assistant { .. }) => {
                self.scroll_offset = 0;
            }
            _ => {}
        }
    }

    pub(crate) fn start_live_thinking(&mut self) {
        if let Some(index) = self.live_thinking_index {
            if self.thinking_collapse_deadline.is_none()
                && matches!(
                    self.messages.get(index),
                    Some(ClaudeMessage::Thinking { .. })
                )
            {
                return;
            }
        }
        self.thinking_collapse_deadline = None;
        self.thinking_expanded_index = None;
        self.messages.push(ClaudeMessage::Thinking {
            content: String::new(),
            is_streaming: true,
            word_count: 0,
        });
        self.live_thinking_index = Some(self.messages.len().saturating_sub(1));
        self.scroll_offset = 0;
    }

    pub(crate) fn append_live_thinking(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.live_thinking_index.is_none() || self.thinking_collapse_deadline.is_some() {
            self.start_live_thinking();
        }
        if let Some(index) = self.live_thinking_index {
            if let Some(ClaudeMessage::Thinking {
                content,
                word_count,
                ..
            }) = self.messages.get_mut(index)
            {
                content.push_str(text);
                *word_count += text.split_whitespace().count();
                self.scroll_offset = 0;
            }
        }
    }

    pub(crate) fn finish_live_thinking(&mut self) {
        if let Some(index) = self.live_thinking_index {
            if let Some(ClaudeMessage::Thinking { is_streaming, .. }) = self.messages.get_mut(index)
            {
                *is_streaming = false;
            }
            let word_count = match self.messages.get(index) {
                Some(ClaudeMessage::Thinking { word_count, .. }) => *word_count,
                _ => 0,
            };
            let should_remove = match self.messages.get(index) {
                Some(ClaudeMessage::Thinking { content, .. }) if content.trim().is_empty() => true,
                _ => false,
            };
            if should_remove {
                self.messages.remove(index);
                self.live_thinking_index = None;
                self.thinking_collapse_deadline = None;
                self.thinking_expanded_index = None;
                return;
            }
            let delay_secs = (word_count as f64 / 300.0 * 60.0).clamp(3.0, 60.0);
            self.thinking_collapse_deadline =
                Some((index, Instant::now() + Duration::from_secs_f64(delay_secs)));
        }
    }

    pub(crate) fn thinking_redraw_deadline(&self) -> Option<Instant> {
        self.thinking_collapse_deadline
            .map(|(_, deadline)| deadline)
            .filter(|deadline| Instant::now() < *deadline)
    }

    fn thinking_expanded_for_index(&self, index: usize) -> bool {
        if self.expanded {
            return true;
        }
        if self.thinking_expanded_index == Some(index) {
            return true;
        }
        if self.live_thinking_index == Some(index) {
            return match self.thinking_collapse_deadline {
                Some((deadline_index, deadline)) if deadline_index == index => {
                    Instant::now() < deadline
                }
                Some(_) => false,
                None => true,
            };
        }
        if let Some(ClaudeMessage::Notice(notice)) = self.messages.get(index) {
            return notice.persistence == NoticePersistence::TranscriptCollapsible
                && Instant::now().duration_since(notice.created_at) < TELEMETRY_COLLAPSE_DELAY;
        }
        false
    }

    pub(crate) fn scroll_up(&mut self, lines: usize) {
        let was_at_bottom = self.scroll_offset == 0;
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        // Set divider on first scroll-away
        if was_at_bottom && self.scroll_offset > 0 {
            if self.divider_index.is_none() {
                self.divider_index = Some(self.messages.len().saturating_sub(1));
            }
        }
    }

    pub(crate) fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        // Clear divider when returning to bottom
        if self.scroll_offset == 0 {
            self.divider_index = None;
            self.divider_y = None;
        }
    }

    /// Find the last running ToolTrace with matching name and update its status.
    pub(crate) fn update_last_tool_trace(&mut self, name: &str, status: ToolTraceStatus) {
        for msg in self.messages.iter_mut().rev() {
            if let ClaudeMessage::ToolTrace {
                name: n,
                status: s,
                collapsed,
                ..
            } = msg
            {
                if n == name && matches!(s, ToolTraceStatus::Running) {
                    *s = status;
                    *collapsed = true;
                    return;
                }
            }
        }
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.divider_index = None;
        self.divider_y = None;
    }

    /// Get the last user message content (for sticky header)
    pub(crate) fn last_user_message(&self) -> Option<String> {
        self.messages.iter().rev().find_map(|m| {
            if let ClaudeMessage::User { content } = m {
                Some(content.clone())
            } else {
                None
            }
        })
    }

    /// Count unseen assistant turns from divider_index to end
    pub(crate) fn count_unseen_assistant_turns(&self) -> usize {
        let idx = self.divider_index.unwrap_or(0);
        self.messages
            .iter()
            .skip(idx)
            .filter(|m| {
                matches!(
                    m,
                    ClaudeMessage::Assistant { .. } | ClaudeMessage::Thinking { .. }
                )
            })
            .count()
    }

    /// Returns rendered lines plus a parallel vector mapping each line to its
    /// source message index (for click-to-expand and other hit-testing).
    pub(crate) fn render_ratatui(&self, width: usize) -> (Vec<Line<'static>>, Vec<usize>) {
        let mut lines = Vec::new();
        let mut mapping = Vec::new();

        let mut i = 0usize;
        while i < self.messages.len() {
            let msg = &self.messages[i];

            // Add blank line only on speaker changes (user → assistant transition)
            if let ClaudeMessage::Assistant { .. } = msg {
                if let Some(ClaudeMessage::User { .. }) = self.messages.get(i.wrapping_sub(1)) {
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                        mapping.push(i); // map blank line to the assistant message too
                    }
                }
            }

            let msg_lines =
                self.messages[i].to_ratatui_lines(self.thinking_expanded_for_index(i), width);
            for _ in &msg_lines {
                mapping.push(i);
            }
            lines.extend(msg_lines);
            i += 1;
        }
        (lines, mapping)
    }

    /// Toggle collapse/expand for a ToolTrace or Thinking at the given message index.
    pub(crate) fn toggle_trace_collapse(&mut self, message_index: usize) {
        if let Some(msg) = self.messages.get_mut(message_index) {
            match msg {
                ClaudeMessage::ToolTrace { collapsed, .. } => {
                    *collapsed = !*collapsed;
                }
                ClaudeMessage::Thinking { .. } => {
                    if self.thinking_expanded_index == Some(message_index) {
                        self.thinking_expanded_index = None;
                        if self.live_thinking_index == Some(message_index) {
                            self.live_thinking_index = None;
                        }
                    } else {
                        self.thinking_expanded_index = Some(message_index);
                        if self.live_thinking_index == Some(message_index) {
                            self.live_thinking_index = None;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn render(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for (i, msg) in self.messages.iter().enumerate() {
            lines.extend(msg.to_lines(self.thinking_expanded_for_index(i)));
        }
        lines
    }
}

// ============================================================================
// Footer Hints (Claude Code-style)
// ============================================================================

pub(crate) const FOOTER_HINTS: &[&str] = &["ctrl+o history · ctrl+t tasks · ctrl+c exit"];

pub(crate) fn render_footer_hints() -> String {
    FOOTER_HINTS
        .iter()
        .map(|s| dim(s))
        .collect::<Vec<_>>()
        .join("  ")
}
