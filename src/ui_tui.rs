//! @efficiency-role: ui-component
//!
//! Raw ANSI Terminal UI — No framework, no layout bugs
//!
//! Uses crossterm for raw mode + events, ANSI codes for colors.
//! Direct screen rendering — text can't overlap if we place it correctly.
//!
//! Architecture (matches Claude Code):
//! ┌──────────────────────────────────────┐
//! │                                      │
//! │  > user message                      │
//! │  ● assistant response                │
//! │  ✓ shell (ls -ltr)                   │
//! │  ✓ completed                         │
//! │    [tool output]                     │
//! │  ● assistant final answer            │
//! │                                      │
//! ├──────────────────────────────────────┤
//! │ > type here    │ model · 38% ctx    │
//! └──────────────────────────────────────┘

use crate::ui_colors::*;
use crossterm::{
    cursor::{MoveTo, MoveToNextLine, Show, Hide},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType, size},
};
use std::io::{self, Write};

// ============================================================================
// Color constants (ANSI 24-bit RGB)
// ============================================================================

fn fg(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}
fn fg_bold(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[1;38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}
fn dim(text: &str) -> String {
    format!("\x1b[2m{}\x1b[0m", text)
}
fn bold(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

// Catppuccin Mocha colors
const MAUVE: (u8,u8,u8) = (203, 166, 247);
const TEAL: (u8,u8,u8) = (148, 226, 213);
const RED: (u8,u8,u8) = (243, 139, 168);
const YELLOW: (u8,u8,u8) = (250, 189, 47);
const GREEN: (u8,u8,u8) = (184, 187, 38);
const FG: (u8,u8,u8) = (235, 219, 178);
const GRAY: (u8,u8,u8) = (146, 131, 116);
const BLUE: (u8,u8,u8) = (137, 180, 250); // Catppuccin blue for bullets
const PURPLE: (u8,u8,u8) = (203, 166, 247);

const DOT: &str = "●";
const CHECK: &str = "✓";
const CROSS: &str = "✗";
const BULLET: &str = "•";
const BLOCKQUOTE_BAR: &str = "▎";
const HR: &str = "─";
const MIDDOT: &str = "·";

// ============================================================================
// Data Types
// ============================================================================

#[derive(Clone)]
pub(crate) struct UIMessage {
    pub(crate) role: MessageRole,
    pub(crate) content: String,
}

#[derive(Clone)]
pub(crate) enum MessageRole {
    User,
    Assistant,
    Tool { name: String, command: String },
    ToolResult { name: String, success: bool, output: String },
    Thinking,
    System,
}

#[derive(Clone, Default)]
pub(crate) struct StatusBarData {
    pub(crate) model: String,
    pub(crate) context_current: u64,
    pub(crate) context_max: u64,
    pub(crate) tokens_in: u64,
    pub(crate) tokens_out: u64,
    pub(crate) effort: String,
}

// ============================================================================
// TerminalUI
// ============================================================================

pub(crate) struct TerminalUI {
    messages: Vec<UIMessage>,
    status: StatusBarData,
    input_buffer: String,
    raw_mode: bool,
    scroll_offset: usize,
}

impl TerminalUI {
    pub(crate) fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self {
            messages: Vec::new(),
            status: StatusBarData::default(),
            input_buffer: String::new(),
            raw_mode: true,
            scroll_offset: 0,
        })
    }

    pub(crate) fn add_message(&mut self, role: MessageRole, content: String) {
        self.messages.push(UIMessage { role, content });
        self.scroll_offset = 0; // Auto-scroll to bottom
        let _ = self.draw();
    }

    pub(crate) fn update_status(
        &mut self,
        model: String,
        ctx_current: u64,
        ctx_max: u64,
        tokens_in: u64,
        tokens_out: u64,
        effort: String,
    ) {
        self.status.model = model;
        self.status.context_current = ctx_current;
        self.status.context_max = ctx_max;
        self.status.tokens_in = tokens_in;
        self.status.tokens_out = tokens_out;
        self.status.effort = effort;
        let _ = self.draw();
    }

    pub(crate) fn estimate_tokens(text: &str) -> u64 {
        text.len() as u64 / 4
    }

    pub(crate) fn run_input_loop(&mut self) -> io::Result<Option<String>> {
        loop {
            self.draw()?;
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                    if kind != KeyEventKind::Press { continue; }
                    match code {
                        KeyCode::Enter => {
                            let input = self.input_buffer.clone();
                            self.input_buffer.clear();
                            if input.trim().is_empty() {
                                continue;
                            }
                            return Ok(Some(input));
                        }
                        KeyCode::Char(c) => {
                            self.input_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            self.input_buffer.pop();
                        }
                        KeyCode::PageUp => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(5);
                        }
                        KeyCode::PageDown | KeyCode::End => {
                            self.scroll_offset = 0;
                        }
                        KeyCode::Up => {
                            if self.scroll_offset == 0 {
                                self.scroll_offset = 1;
                            } else {
                                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Esc => {
                            return Ok(None);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub(crate) fn cleanup(&mut self) -> io::Result<()> {
        if self.raw_mode {
            execute!(io::stdout(), Show, LeaveAlternateScreen)?;
            terminal::disable_raw_mode()?;
            io::stdout().flush()?;
            self.raw_mode = false;
        }
        Ok(())
    }

    pub(crate) fn draw(&mut self) -> io::Result<()> {
        let (cols, rows) = size()?;
        let cols = cols as usize;
        let rows = rows as usize;

        // Build all message lines
        let mut all_lines: Vec<String> = Vec::new();
        for msg in &self.messages {
            all_lines.extend(self.render_message(msg, cols));
            all_lines.push(String::new()); // blank line between messages
        }
        if all_lines.is_empty() {
            all_lines.push(dim("  Type a message to begin..."));
        }

        // Calculate how many content rows we have
        let content_rows = rows.saturating_sub(1); // 1 row for bottom bar

        // Calculate scroll offset (0 = show bottom)
        if all_lines.len() > content_rows {
            let max_scroll = all_lines.len() - content_rows;
            if self.scroll_offset > max_scroll {
                self.scroll_offset = max_scroll;
            }
        } else {
            self.scroll_offset = 0;
        }

        // Get visible lines
        let start = if all_lines.len() > content_rows {
            all_lines.len() - content_rows - self.scroll_offset
        } else {
            0
        };
        let visible: &[String] = &all_lines[start..];

        // Clear screen and render
        let mut out = io::stdout();
        execute!(out, Clear(ClearType::All), MoveTo(0, 0))?;

        // Render visible content
        for (i, line) in visible.iter().enumerate() {
            if i < content_rows {
                writeln!(out, "{}", line)?;
            }
        }
        // Fill remaining rows
        for _ in visible.len()..content_rows {
            writeln!(out)?;
        }

        // Render bottom bar
        let bottom_row = (rows - 1) as u16;
        execute!(out, MoveTo(0, bottom_row))?;

        // Separator line
        write!(out, "\x1b[38;2;146;131;116m{}\x1b[0m", HR.repeat(cols))?;
        execute!(out, MoveToNextLine(1))?;

        // Prompt + status
        let prompt_text = format!("> {}", self.input_buffer);
        let prompt_colored = fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, &prompt_text);

        let pct = if self.status.context_max > 0 {
            self.status.context_current as f64 / self.status.context_max as f64 * 100.0
        } else {
            0.0
        };
        let status_text = format!(
            "{} {} {:.1}% {} {}/{} {} {}",
            self.status.model,
            MIDDOT,
            pct,
            MIDDOT,
            format_tokens(self.status.tokens_in),
            format_tokens(self.status.tokens_out),
            MIDDOT,
            self.status.effort,
        );
        let status_colored = dim(&status_text);

        let combined = format!("{}  {}", prompt_colored, status_colored);
        write!(out, " {}", combined)?;

        // Clear rest of line
        execute!(out, Clear(ClearType::UntilNewLine))?;

        // Position cursor after "> "
        let cursor_col = (prompt_text.chars().count() + 2) as u16;
        execute!(out, MoveTo(cursor_col, bottom_row + 1), Show)?;
        out.flush()?;

        Ok(())
    }

    /// Render a single message into multiple display lines.
    fn render_message(&self, msg: &UIMessage, cols: usize) -> Vec<String> {
        let mut lines = Vec::new();

        match &msg.role {
            MessageRole::User => {
                let text = dim(&format!("> {}", msg.content));
                lines.extend(wrap_text(&text, cols));
            }
            MessageRole::Assistant => {
                // Render markdown
                for content_line in msg.content.lines() {
                    let rendered = render_md_line(content_line);
                    lines.extend(wrap_text(&rendered, cols));
                }
            }
            MessageRole::Tool { name, command } => {
                let text = format!(
                    " {} {} {}",
                    fg(GRAY.0, GRAY.1, GRAY.2, DOT),
                    fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, name),
                    dim(&format!("({})", command)),
                );
                lines.extend(wrap_text(&text, cols));
            }
            MessageRole::ToolResult { name, success, output } => {
                let (icon, color) = if *success {
                    (CHECK, GREEN)
                } else {
                    (CROSS, RED)
                };
                let header = format!(
                    "  {} {}",
                    fg(color.0, color.1, color.2, icon),
                    fg(FG.0, FG.1, FG.2, name),
                );
                lines.extend(wrap_text(&header, cols));

                // Show output (capped at 20 lines)
                if !output.is_empty() {
                    let line_count = output.lines().count();
                    let display_lines: Vec<&str> = output.lines().take(20).collect();
                    for oline in &display_lines {
                        let prefixed = format!("    {}", oline);
                        lines.extend(wrap_text(&prefixed, cols));
                    }
                    if line_count > 20 {
                        let note = dim(&format!(
                            "    ... ({} more lines in session logs)",
                            line_count - 20
                        ));
                        lines.extend(wrap_text(&note, cols));
                    }
                }
            }
            MessageRole::Thinking => {
                let text = dim(&format!("  {} {}", DOT, msg.content));
                lines.extend(wrap_text(&text, cols));
            }
            MessageRole::System => {
                let text = dim(&msg.content);
                lines.extend(wrap_text(&text, cols));
            }
        }

        lines
    }

    pub(crate) fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        let _ = self.draw();
    }
}

// ============================================================================
// Markdown rendering → ANSI strings
// ============================================================================

fn render_md_line(line: &str) -> String {
    let trimmed = line.trim();

    if trimmed.starts_with("```") {
        let lang = trimmed.strip_prefix("```").unwrap_or("").trim();
        if lang.is_empty() {
            return fg(GRAY.0, GRAY.1, GRAY.2, &HR.repeat(50));
        }
        return fg(GRAY.0, GRAY.1, GRAY.2, &format!("── {} ──", lang));
    }

    if let Some(rest) = trimmed.strip_prefix("### ") {
        return fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, rest);
    }
    if let Some(rest) = trimmed.strip_prefix("## ") {
        return fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, &format!("## {}", rest));
    }
    if let Some(rest) = trimmed.strip_prefix("# ") {
        return fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, &format!("# {}", rest));
    }

    if let Some(rest) = trimmed.strip_prefix("- ") {
        let bullet = fg(BLUE.0, BLUE.1, BLUE.2, &format!("{} ", BULLET));
        return format!("{}{}", bullet, parse_inline_md(rest));
    }

    if let Some(rest) = trimmed.strip_prefix("> ") {
        return dim(&format!("{} {}", BLOCKQUOTE_BAR, rest));
    }

    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return fg(GRAY.0, GRAY.1, GRAY.2, &HR.repeat(50));
    }

    parse_inline_md(trimmed)
}

fn parse_inline_md(text: &str) -> String {
    let mut result = String::new();
    let mut current = String::new();
    let mut in_code = false;
    let mut in_bold = false;
    let mut in_italic = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '`' if !in_code => {
                if !current.is_empty() {
                    result.push_str(&style_text(&current, in_bold, in_italic));
                    current.clear();
                }
                in_code = true;
            }
            '`' if in_code => {
                if !current.is_empty() {
                    result.push_str(&fg(PURPLE.0, PURPLE.1, PURPLE.2, &current));
                    current.clear();
                }
                in_code = false;
            }
            '*' if !in_code && chars.peek() == Some(&'*') => {
                chars.next();
                if !current.is_empty() {
                    result.push_str(&style_text(&current, in_bold, in_italic));
                    current.clear();
                }
                in_bold = !in_bold;
            }
            '_' if !in_code && chars.peek() == Some(&'_') => {
                chars.next();
                if !current.is_empty() {
                    result.push_str(&style_text(&current, in_bold, in_italic));
                    current.clear();
                }
                in_italic = !in_italic;
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        result.push_str(&style_text(&current, in_bold, in_italic));
    }

    if result.is_empty() {
        result
    } else {
        result
    }
}

fn style_text(text: &str, bold: bool, italic: bool) -> String {
    let mut s = String::new();
    if bold { s.push_str("\x1b[1m"); }
    if italic { s.push_str("\x1b[3m"); }
    s.push_str(text);
    if bold || italic { s.push_str("\x1b[0m"); }
    s
}

/// Wrap an ANSI-encoded string to fit terminal width.
/// Respects ANSI escape sequences (doesn't break them).
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut display_width = 0;
    let mut in_escape = false;
    let mut pending_escape = String::new();

    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
            pending_escape.push(ch);
            continue;
        }
        if in_escape {
            pending_escape.push(ch);
            if ch == 'm' {
                in_escape = false;
                // Escape sequence complete — append to current
                current.push_str(&pending_escape);
                pending_escape.clear();
            }
            continue;
        }

        // Regular character
        display_width += if ch == '\t' { 4 } else { 1 };

        if display_width > max_width {
            // Wrap: close any open escapes, start new line
            if !pending_escape.is_empty() {
                current.push_str(&pending_escape);
                pending_escape.clear();
            }
            // Close current line's formatting
            current.push_str("\x1b[0m");
            lines.push(current);

            // Start new line with pending formatting
            // For simplicity, start fresh (ANSI state resets per line)
            current = pending_escape.clone();
            pending_escape.clear();
            display_width = if ch == '\t' { 4 } else { 1 };
        }
        current.push(ch);
    }

    // Flush pending escape
    if !pending_escape.is_empty() {
        current.push_str(&pending_escape);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn format_tokens(count: u64) -> String {
    if count < 1000 { count.to_string() }
    else if count < 1_000_000 { format!("{:.1}k", count as f64 / 1000.0) }
    else { format!("{:.1}M", count as f64 / 1_000_000.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = UIMessage { role: MessageRole::Assistant, content: "Hello".to_string() };
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(4096), "4.1k");
    }

    #[test]
    fn test_wrap_text() {
        let lines = wrap_text("hello world", 5);
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_special_chars() {
        assert_eq!(DOT, "●");
        assert_eq!(CHECK, "✓");
        assert_eq!(CROSS, "✗");
    }
}
