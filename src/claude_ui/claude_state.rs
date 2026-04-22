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

use crate::claude_ui::render_markdown_ratatui;
use crate::ui_state::is_reasoning_visible;
use crate::ui_theme::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

// ============================================================================
// Message Types (Claude Code-style)
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) enum ClaudeMessage {
    User {
        content: String,
    },
    Assistant {
        content: String,
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
    PermissionRequest {
        command: String,
        reason: Option<String>,
    },
    Thinking {
        content: String,
    },
    CompactBoundary,
    CompactSummary {
        message_count: usize,
        context_preview: Option<String>,
    },
    System {
        content: String,
    },
}

impl ClaudeMessage {
    pub(crate) fn to_ratatui_lines(&self, expanded: bool) -> Vec<Line<'static>> {
        let theme = current_theme();
        match self {
            ClaudeMessage::User { content } => {
                // User messages: left gutter with "❯" indicator (Claude Code style)
                let content_str = if content.is_empty() {
                    String::new()
                } else if content.len() > 10000 {
                    // Hard-cap at 10,000 chars: head 2,500 + ellipsis + tail 2,500
                    let head = &content[..2500];
                    let tail = &content[content.len() - 2500..];
                    let skipped_lines = content[2500..content.len() - 2500].lines().count();
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
                // Left gutter approach: indicator on first line, empty gutter on subsequent
                let mut lines = Vec::new();
                let content_lines = render_markdown_ratatui(content);
                for (i, content_line) in content_lines.into_iter().enumerate() {
                    if i == 0 {
                        // First line: gutter indicator + content
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
                        // Subsequent lines: empty gutter + content
                        let mut spans = vec![Span::raw("  ")]; // 2-char gutter
                        spans.extend(content_line.spans.into_iter());
                        lines.push(Line::from(spans));
                    }
                }
                lines
            }
            ClaudeMessage::Thinking { content } => {
                let reasoning_visible = is_reasoning_visible();
                let fully_shown = expanded && reasoning_visible;
                if fully_shown {
                    let mut lines = Vec::new();
                    // First line: thinking indicator + content
                    lines.push(Line::from(vec![
                        Span::styled(
                            "∴",
                            Style::default()
                                .fg(theme.warning.to_ratatui_color())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            "Thinking…",
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                    // Subsequent lines: empty gutter + content
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
                } else {
                    // Collapsed placeholder
                    let hint = if reasoning_visible {
                        "(ctrl+o to expand)"
                    } else {
                        "(hidden — /reasoning to show)"
                    };
                    vec![Line::from(vec![
                        Span::styled(
                            "∴ Thinking ",
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                        Span::styled(
                            hint,
                            Style::default()
                                .fg(theme.fg_dim.to_ratatui_color())
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ])]
                }
            }
            ClaudeMessage::ToolStart { name, input } => {
                // Tool start: use ▶ indicator in gutter
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
                // Tool progress: use ◐ indicator in gutter
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
                let mut lines = vec![Line::from(vec![
                    Span::styled(
                        "● ",
                        Style::default().fg(theme.accent_primary.to_ratatui_color()),
                    ),
                    Span::styled(
                        "Permission required",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ])];
                if let Some(r) = reason {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            r.clone(),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ),
                    ]));
                }
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        command.clone(),
                        Style::default().add_modifier(Modifier::ITALIC),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Press y to approve, n to deny",
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines
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
                
                // First line: indicator in gutter
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
                
                // Subsequent lines: empty gutter
                let output_lines: Vec<&str> = output.lines().collect();
                let max_lines = if expanded { output_lines.len() } else { 15 };
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
                            format!("({} more lines — ctrl+o to expand)", remaining),
                            Style::default()
                                .fg(theme.fg_dim.to_ratatui_color())
                                .add_modifier(Modifier::ITALIC),
                        ),
                    ]));
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
                let mut lines = vec![format!("● {}", content)];
                for line in content.lines().skip(1) {
                    lines.push(format!("  {}", line));
                }
                lines
            }
            ClaudeMessage::Thinking { content } => {
                let reasoning_visible = is_reasoning_visible();
                let fully_shown = expanded && reasoning_visible;
                if fully_shown {
                    let mut lines = vec!["∴ Thinking…".to_string()];
                    for line in content.lines() {
                        lines.push(format!("    {}", dim(line)));
                    }
                    lines
                } else {
                    let hint = if reasoning_visible {
                        "(ctrl+o to expand)"
                    } else {
                        "(hidden — /reasoning to show)"
                    };
                    vec![format!("∴ Thinking {}", hint)]
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
                let max_lines = if expanded { output_lines.len() } else { 15 };
                for line in output_lines.iter().take(max_lines) {
                    lines.push(format!("    {}", dim(line)));
                }
                if output_lines.len() > max_lines {
                    let remaining = output_lines.len() - max_lines;
                    let line = format!("    ({} more lines — ctrl+o to expand)", remaining);
                    lines.push(dim(&line));
                }
                lines
            }
            ClaudeMessage::CompactBoundary => {
                if expanded {
                    vec!["✻ Conversation compacted (ctrl+o for history)".to_string()]
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
        self.messages.push(msg);
        // Auto-scroll to bottom on new message
        self.scroll_offset = 0;
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
        self.messages.iter().skip(idx).filter(|m| {
            matches!(m, ClaudeMessage::Assistant { .. } | ClaudeMessage::Thinking { .. })
        }).count()
    }

    pub(crate) fn render_ratatui(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let theme = current_theme();

        // If not expanded, show only the last few messages or a compact summary
        if !self.expanded && self.messages.len() > 5 {
            lines.push(Line::from(vec![
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
            ]));
            lines.push(Line::from(""));
            for msg in self.messages.iter().skip(self.messages.len() - 3) {
                lines.extend(msg.to_ratatui_lines(self.expanded));
                lines.push(Line::from(""));
            }
        } else {
            // Count thinking blocks for filtering (only show last one in normal mode)
            let thinking_count = self.messages.iter().filter(|m| matches!(m, ClaudeMessage::Thinking { .. })).count();
            let mut thinking_seen = 0usize;
            
            let mut i = 0usize;
            while i < self.messages.len() {
                let msg = &self.messages[i];
                
                // Skip non-last thinking blocks when not expanded
                if let ClaudeMessage::Thinking { .. } = msg {
                    thinking_seen += 1;
                    if !self.expanded && thinking_seen < thinking_count {
                        i += 1;
                        continue;
                    }
                }
                
                // Add blank line before assistant messages (Claude Code spacing)
                if let ClaudeMessage::Assistant { .. } = msg {
                    if !lines.is_empty() && !lines.last().map(|l| l.spans.is_empty()).unwrap_or(true) {
                        lines.push(Line::from(""));
                    }
                }
                
                if !self.expanded {
                    let batch_kind = match self.messages.get(i) {
                        Some(ClaudeMessage::ToolStart { name, .. })
                            if name == "read" || name == "search" =>
                        {
                            Some("read/search")
                        }
                        Some(ClaudeMessage::ToolStart { name, .. }) if name == "shell" => {
                            Some("shell")
                        }
                        _ => None,
                    };
                    if let Some(kind) = batch_kind {
                        let mut j = i;
                        let mut count = 0usize;
                        while j < self.messages.len() {
                            match &self.messages[j] {
                                ClaudeMessage::ToolStart { name, .. }
                                | ClaudeMessage::ToolResult { name, .. }
                                    if (kind == "read/search"
                                        && (name == "read" || name == "search"))
                                        || (kind == "shell" && name == "shell") =>
                                {
                                    count += 1;
                                    j += 1;
                                }
                                ClaudeMessage::ToolProgress { name, .. }
                                    if (kind == "read/search"
                                        && (name == "read" || name == "search"))
                                        || (kind == "shell" && name == "shell") =>
                                {
                                    j += 1;
                                }
                                _ => break,
                            }
                        }
                        if count > 1 {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    "  ● ",
                                    Style::default().fg(theme.accent_secondary.to_ratatui_color()),
                                ),
                                Span::styled(
                                    format!("{} batch ({} items)", kind, count),
                                    Style::default().fg(theme.fg_dim.to_ratatui_color()),
                                ),
                                Span::styled(
                                    " (ctrl+o to expand)",
                                    Style::default()
                                        .fg(theme.fg_dim.to_ratatui_color())
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                            lines.push(Line::from(""));
                            i = j;
                            continue;
                        }
                    }
                }

                lines.extend(self.messages[i].to_ratatui_lines(self.expanded));
                lines.push(Line::from(""));
                i += 1;
            }
        }
        lines
    }

    pub(crate) fn render(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut i = 0usize;
        while i < self.messages.len() {
            if !self.expanded {
                let batch_kind = match self.messages.get(i) {
                    Some(ClaudeMessage::ToolStart { name, .. })
                        if name == "read" || name == "search" =>
                    {
                        Some("read/search")
                    }
                    Some(ClaudeMessage::ToolStart { name, .. }) if name == "shell" => Some("shell"),
                    _ => None,
                };
                if let Some(kind) = batch_kind {
                    let mut j = i;
                    let mut count = 0usize;
                    while j < self.messages.len() {
                        match &self.messages[j] {
                            ClaudeMessage::ToolStart { name, .. }
                            | ClaudeMessage::ToolResult { name, .. }
                                if (kind == "read/search"
                                    && (name == "read" || name == "search"))
                                    || (kind == "shell" && name == "shell") =>
                            {
                                count += 1;
                                j += 1;
                            }
                            ClaudeMessage::ToolProgress { name, .. }
                                if (kind == "read/search"
                                    && (name == "read" || name == "search"))
                                    || (kind == "shell" && name == "shell") =>
                            {
                                j += 1;
                            }
                            _ => break,
                        }
                    }
                    if count > 1 {
                        lines.push(format!(
                            "  ● {}",
                            dim(&format!(
                                "{} batch ({} items) (ctrl+o to expand)",
                                kind, count
                            ))
                        ));
                        lines.push(String::new());
                        i = j;
                        continue;
                    }
                }
            }

            lines.extend(self.messages[i].to_lines(self.expanded));
            lines.push(String::new());
            i += 1;
        }
        lines
    }
}

// ============================================================================
// Footer Hints (Claude Code-style)
// ============================================================================

pub(crate) const FOOTER_HINTS: &[&str] = &[
    "ctrl+o: transcript",
    "ctrl+t: tasks",
    "ctrl+c: interrupt",
    "enter: send",
    "esc: cancel",
];

pub(crate) fn render_footer_hints() -> String {
    FOOTER_HINTS
        .iter()
        .map(|s| dim(s))
        .collect::<Vec<_>>()
        .join("  ")
}
