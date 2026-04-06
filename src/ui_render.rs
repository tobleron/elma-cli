//! @efficiency-role: ui-component
//!
//! Full-screen rendering — Gruvbox Dark Hard layout.
//!
//! Layout order (bottom to top):
//!   [Status bar — 1 row, bottom of terminal]
//!   [Input box — bordered, above status bar]
//!   [Autocomplete dropdown — bordered box, above input when active]
//!   [Transcript area — fills all remaining space at top]

use crate::ui_autocomplete::AutocompleteState;
use crate::ui_input::TextInput;
use crate::ui_markdown::render_markdown;
use crate::ui_state::*;
use crate::ui_theme::*;
use crate::ui_wrap::{display_width, wrap_ansi};

/// Result of rendering a full screen.
pub(crate) struct ScreenBuffer {
    pub lines: Vec<String>,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

/// Render the full screen from UIState.
pub(crate) fn render_screen(
    state: &UIState,
    width: usize,
    height: usize,
    input: &TextInput,
) -> ScreenBuffer {
    if height == 0 || width == 0 {
        return ScreenBuffer {
            lines: vec![],
            cursor_row: 0,
            cursor_col: 0,
        };
    }

    // === Layout calculation (bottom-up) ===
    // Bottom: Status bar (1 row)
    let status_rows = 1;

    // Above status: Input box (content lines + 2 borders)
    let input_content_rows = input.line_count().max(1).min(3);
    let input_box_rows = input_content_rows + 2; // top + bottom border

    // Above input: Autocomplete dropdown (when active, bordered box)
    let dropdown_rows = if state.autocomplete.active && !state.autocomplete.matches.is_empty() {
        let items = state.autocomplete.matches.len().min(8);
        items + 4 // items + top/bottom border + 2 padding lines
    } else {
        0
    };

    // Above dropdown: Activity rail / streaming state (1-2 rows)
    let activity_rows = if state.streaming.kind != StreamingKind::Idle {
        2 // streaming state needs space
    } else if let ActivityState::Active { .. } = &state.activity {
        1 // activity rail
    } else {
        0
    };

    // Top: Transcript fills remaining space
    let bottom_reserved = status_rows + input_box_rows + dropdown_rows + activity_rows;
    let transcript_rows = if height > bottom_reserved {
        height - bottom_reserved
    } else {
        1
    };

    let mut screen: Vec<String> = Vec::with_capacity(height);

    // ===== FRAME 1: Transcript =====
    let transcript = render_transcript(
        &state.transcript,
        width,
        transcript_rows,
        state.viewport.scroll_offset,
        state.show_thinking,
    );
    screen.extend(transcript);
    while screen.len() < transcript_rows {
        screen.push(String::new());
    }

    // ===== FRAME 2: Activity rail / Streaming state (reserved space above transcript) =====
    if state.streaming.kind != StreamingKind::Idle {
        let streaming_lines = render_streaming_state(&state.streaming, width);
        for line in &streaming_lines {
            screen.push(line.clone());
        }
        while screen.len() < transcript_rows + activity_rows {
            screen.push(String::new());
        }
    } else if let ActivityState::Active { label, message } = &state.activity {
        let activity_line = format!(
            "  {} {} {}",
            fg(GRAY.0, GRAY.1, GRAY.2, &SPINNER_FRAMES[0]),
            fg_bold(AQUA.0, AQUA.1, AQUA.2, label),
            dim(message),
        );
        screen.push(activity_line);
        while screen.len() < transcript_rows + activity_rows {
            screen.push(String::new());
        }
    }

    // ===== FRAME 4: Autocomplete dropdown (bordered box above input) =====
    // This renders BEFORE the input box, in the reserved dropdown_rows space
    if state.autocomplete.active && !state.autocomplete.matches.is_empty() {
        let dropdown_lines = render_autocomplete_dropdown(&state.autocomplete, width);
        screen.extend(dropdown_lines);
    }

    // ===== FRAME 5: Input box (borders + content) =====
    let input_lines = render_input_box(input, width);
    screen.extend(input_lines);

    // ===== FRAME 6: Status bar =====
    let status_line = render_status_bar(&state.footer, &state.header, width);
    screen.push(status_line);

    // Ensure exactly `height` lines
    while screen.len() < height {
        screen.push(String::new());
    }
    screen.truncate(height);

    // Cursor: positioned in the input content area (skip top border = +1)
    let input_content_start = transcript_rows + activity_rows + dropdown_rows + 1; // +1 for top border
    let cursor_row = (input_content_start + input.cursor_row()) as u16;
    let cursor_col = (2 + input.display_col()) as u16; // after "❯ "

    ScreenBuffer {
        lines: screen,
        cursor_row,
        cursor_col,
    }
}

// ============================================================================
// FRAME 1: Transcript
// ============================================================================

fn render_transcript(
    items: &[TranscriptItem],
    width: usize,
    visible_rows: usize,
    scroll_offset: usize,
    show_thinking: bool,
) -> Vec<String> {
    // Reserve 2 chars on right edge for potential scroll indicator
    let content_width = width.saturating_sub(4);

    let mut all_lines: Vec<String> = Vec::new();

    for (msg_idx, item) in items.iter().enumerate() {
        // Add spacing between messages (except before first)
        if !all_lines.is_empty() {
            all_lines.push(String::new());
        }
        let item_lines = render_message(item, content_width, msg_idx, show_thinking);
        all_lines.extend(item_lines);
    }

    if all_lines.is_empty() {
        all_lines.push(dim("  Type a message to begin..."));
        all_lines.push(String::new());
    }

    // Remove trailing empty lines
    while all_lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        all_lines.pop();
    }

    let total = all_lines.len();
    let max_scroll = if total > visible_rows {
        total - visible_rows
    } else {
        0
    };
    let clamped_scroll = scroll_offset.min(max_scroll);
    let start = if total > visible_rows {
        total - visible_rows - clamped_scroll
    } else {
        0
    };

    let visible: Vec<String> = visible_lines_with_scroll_indicator(
        &all_lines,
        start,
        visible_rows,
        total,
        max_scroll,
        clamped_scroll,
    );

    let mut result: Vec<String> = visible.into_iter().take(visible_rows).collect();
    while result.len() < visible_rows {
        result.push(String::new());
    }
    result
}

/// Add a subtle scroll indicator on the right edge when content is scrollable
fn visible_lines_with_scroll_indicator(
    all_lines: &[String],
    start: usize,
    visible_rows: usize,
    total_lines: usize,
    max_scroll: usize,
    scroll_offset: usize,
) -> Vec<String> {
    let has_scroll = max_scroll > 0;
    let visible: Vec<String> = all_lines[start..].to_vec();

    if !has_scroll {
        return visible;
    }

    // Calculate scrollbar position
    let scroll_progress = if max_scroll > 0 {
        scroll_offset as f64 / max_scroll as f64
    } else {
        0.0
    };

    visible
        .into_iter()
        .enumerate()
        .take(visible_rows)
        .map(|(i, line)| {
            let line_pos_in_visible = i as f64 / visible_rows as f64;
            let is_at_scrollbar = (line_pos_in_visible - scroll_progress).abs() < 0.02;

            if is_at_scrollbar {
                format!("{}{}", line, fg(GRAY.0, GRAY.1, GRAY.2, "▏"))
            } else {
                line
            }
        })
        .collect()
}

fn render_message(
    item: &TranscriptItem,
    content_width: usize,
    _msg_idx: usize,
    show_thinking: bool,
) -> Vec<String> {
    match item {
        TranscriptItem::User { content } => render_user_message(content, content_width),
        TranscriptItem::Assistant { content } => render_assistant_message(content, content_width),
        TranscriptItem::ToolStart { name, command } => {
            render_tool_start(name, command, content_width)
        }
        TranscriptItem::ToolResult {
            name,
            success,
            output,
            duration_ms,
        } => render_tool_result(name, *success, output, *duration_ms, content_width),
        TranscriptItem::MetaEvent { category, message } => {
            render_meta_event(category, message, content_width)
        }
        TranscriptItem::Warning { message } => render_warning_msg(message, content_width),
        TranscriptItem::Thinking { content } => {
            if show_thinking {
                render_thinking(content, content_width)
            } else {
                vec![format!(
                    "  {} {} {}",
                    dim(EXPAND_ARROW_RIGHT),
                    dark_gray("Thinking"),
                    dark_gray("(ctrl+o to expand)"),
                )]
            }
        }
        TranscriptItem::System { content } => render_system_message(content, content_width),
    }
}

fn render_user_message(content: &str, content_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for (i, raw_line) in content.lines().enumerate() {
        let prefix = if i == 0 {
            format!(
                "{} {}",
                fg(PROMPT_GRAY.0, PROMPT_GRAY.1, PROMPT_GRAY.2, USER_ARROW),
                white(raw_line)
            )
        } else {
            format!("  {}", white(raw_line))
        };
        let wrapped = wrap_ansi(&prefix, content_width);
        lines.extend(wrapped);
    }
    lines
}

fn render_assistant_message(content: &str, content_width: usize) -> Vec<String> {
    let mut all_lines: Vec<String> = Vec::new();
    let rendered = render_markdown(content);
    let rendered_lines: Vec<&str> = rendered.lines().collect();

    for (i, line) in rendered_lines.iter().enumerate() {
        let prefix = if i == 0 {
            format!(
                "{} {}",
                fg_bold(PURPLE.0, PURPLE.1, PURPLE.2, ASSISTANT_DOT),
                line,
            )
        } else {
            format!("  {}", line)
        };
        let wrapped = wrap_ansi(&prefix, content_width);
        all_lines.extend(wrapped);
    }
    if all_lines.is_empty() {
        all_lines.push(format!(
            "{} {}",
            fg_bold(PURPLE.0, PURPLE.1, PURPLE.2, ASSISTANT_DOT),
            dim("(empty)"),
        ));
    }
    all_lines
}

fn render_tool_start(name: &str, command: &str, content_width: usize) -> Vec<String> {
    let text = format!(
        "  {} {} {}",
        fg(GRAY.0, GRAY.1, GRAY.2, "◦"),
        fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, name),
        dim(command),
    );
    wrap_ansi(&text, content_width)
}

fn render_tool_result(
    name: &str,
    success: bool,
    output: &str,
    duration_ms: Option<u64>,
    content_width: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let (icon, color) = if success {
        (CHECK, GREEN)
    } else {
        (CROSS, RED)
    };
    let duration_str = duration_ms
        .map(|ms| {
            if ms < 1000 {
                format!("{}ms", ms)
            } else {
                format!("{:.1}s", ms as f64 / 1000.0)
            }
        })
        .unwrap_or_default();

    let header = if duration_str.is_empty() {
        format!(
            "  {} {}",
            fg(color.0, color.1, color.2, icon),
            fg(GRAY.0, GRAY.1, GRAY.2, name),
        )
    } else {
        format!(
            "  {} {} {}",
            fg(color.0, color.1, color.2, icon),
            fg(GRAY.0, GRAY.1, GRAY.2, name),
            dim(&format!("({})", duration_str)),
        )
    };
    lines.extend(wrap_ansi(&header, content_width));

    if !output.is_empty() {
        let output_lines: Vec<&str> = output.lines().take(20).collect();
        for oline in &output_lines {
            let prefixed = format!("    {}", oline);
            lines.extend(wrap_ansi(&prefixed, content_width));
        }
        let total = output.lines().count();
        if total > 20 {
            lines.push(dim(&format!("    ... ({} more lines)", total - 20)));
        }
    }
    lines
}

fn render_meta_event(category: &str, message: &str, content_width: usize) -> Vec<String> {
    let styled = match category {
        "PLAN" => fg(
            BLUE.0,
            BLUE.1,
            BLUE.2,
            &format!("[{}] {}", category, message),
        ),
        "CLASSIFY" => fg(
            GRAY.0,
            GRAY.1,
            GRAY.2,
            &format!("[{}] {}", category, message),
        ),
        "REFLECT" => fg(
            PURPLE.0,
            PURPLE.1,
            PURPLE.2,
            &format!("[{}] {}", category, message),
        ),
        _ => dim(&format!("[{}] {}", category, message)),
    };
    wrap_ansi(&styled, content_width)
}

fn render_warning_msg(message: &str, content_width: usize) -> Vec<String> {
    let text = format!("  {} {}", red("⚠"), red(message));
    wrap_ansi(&text, content_width)
}

fn render_thinking(content: &str, content_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "  {} {} {}",
        dim(EXPAND_ARROW_DOWN),
        dark_gray("Thinking"),
        dark_gray("(ctrl+o to collapse)"),
    ));
    lines.push(String::new());
    for raw_line in content.lines() {
        let prefixed = format!("    {}", dark_gray(raw_line));
        lines.extend(wrap_ansi(&prefixed, content_width));
    }
    lines
}

fn render_system_message(content: &str, content_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for (i, raw_line) in content.lines().enumerate() {
        let prefixed = if i == 0 {
            format!(
                "  {} {}",
                fg(SYSTEM_YELLOW.0, SYSTEM_YELLOW.1, SYSTEM_YELLOW.2, LIGHTNING),
                italic(raw_line)
            )
        } else {
            format!("    {}", italic(raw_line))
        };
        lines.extend(wrap_ansi(&prefixed, content_width));
    }
    lines
}

// ============================================================================
// FRAME 2: Streaming state
// ============================================================================

fn render_streaming_state(streaming: &StreamingState, width: usize) -> Vec<String> {
    let content_width = width.saturating_sub(2);
    let spinner = SPINNER_FRAMES[streaming.animation_frame % 10];
    let spinner_char = fg_bold(GRAY.0, GRAY.1, GRAY.2, spinner);

    let line = match streaming.kind {
        StreamingKind::Processing => {
            let text = fg(YELLOW.0, YELLOW.1, YELLOW.2, "Elma is thinking...");
            let meta = if streaming.elapsed_s > 0 || streaming.tokens > 0 {
                let elapsed_str = if streaming.elapsed_s > 0 {
                    format!(" {}s", streaming.elapsed_s)
                } else {
                    String::new()
                };
                let tok_str = if streaming.tokens > 0 {
                    format!(" · {} tok", streaming.tokens)
                } else {
                    String::new()
                };
                dim(&format!(" {}{}", elapsed_str, tok_str))
            } else {
                String::new()
            };
            format!("  {} {}{}", spinner_char, text, meta)
        }
        StreamingKind::Responding => {
            let crab = fg_bold(FG.0, FG.1, FG.2, CRAB);
            let text = fg(YELLOW.0, YELLOW.1, YELLOW.2, "Elma is responding...");
            let meta = if streaming.elapsed_s > 0 || streaming.tokens > 0 {
                let elapsed_str = if streaming.elapsed_s > 0 {
                    format!(" {}s", streaming.elapsed_s)
                } else {
                    String::new()
                };
                let tok_str = if streaming.tokens > 0 {
                    format!(" · {} tok", streaming.tokens)
                } else {
                    String::new()
                };
                dark_gray(&format!(" {}{}", elapsed_str, tok_str))
            } else {
                String::new()
            };
            format!("  {} {} {}{}", spinner_char, crab, text, meta)
        }
        StreamingKind::Idle => String::new(),
    };

    if line.is_empty() {
        vec![]
    } else {
        wrap_ansi(&line, content_width + 2)
    }
}

// ============================================================================
// FRAME 4: Autocomplete dropdown — Gruvbox styled
// Full-width bordered box with fg-on-bg selected items
// ============================================================================

fn render_autocomplete_dropdown(state: &AutocompleteState, width: usize) -> Vec<String> {
    if state.matches.is_empty() {
        return vec![];
    }

    let max_items = state.matches.len().min(8);
    let mut lines: Vec<String> = Vec::new();

    // Top border — full terminal width, bold gray visible on dark terminals
    lines.push(fg(
        BORDER_GRAY.0,
        BORDER_GRAY.1,
        BORDER_GRAY.2,
        &"─".repeat(width),
    ));

    // Padding line — full width to clear terminal
    lines.push(fg(
        BORDER_GRAY.0,
        BORDER_GRAY.1,
        BORDER_GRAY.2,
        &" ".repeat(width),
    ));

    for i in 0..max_items {
        let suggestion = &state.matches[i];
        let is_selected = i == state.selected;

        let content = if state.is_emoji {
            if is_selected {
                // Gruvbox fg on bg2 — high contrast, warm tones
                format!(
                    "  {}  {}",
                    fg_bg_bold(
                        FG.0,
                        FG.1,
                        FG.2,
                        SELECT_BG.0,
                        SELECT_BG.1,
                        SELECT_BG.2,
                        &suggestion.description
                    ),
                    fg_bg(
                        FG.0,
                        FG.1,
                        FG.2,
                        SELECT_BG.0,
                        SELECT_BG.1,
                        SELECT_BG.2,
                        &suggestion.label
                    ),
                )
            } else {
                format!(
                    "  {}  {}",
                    suggestion.description,
                    dark_gray(&suggestion.label),
                )
            }
        } else {
            // Slash command: "  {name:<10} {description}"
            let name_padded = format!("{:<10}", suggestion.label);
            if is_selected {
                // Gruvbox fg on bg2 — high contrast, warm tones
                format!(
                    "  {}  {}",
                    fg_bg_bold(
                        FG.0,
                        FG.1,
                        FG.2,
                        SELECT_BG.0,
                        SELECT_BG.1,
                        SELECT_BG.2,
                        &name_padded
                    ),
                    fg_bg(
                        FG.0,
                        FG.1,
                        FG.2,
                        SELECT_BG.0,
                        SELECT_BG.1,
                        SELECT_BG.2,
                        &suggestion.description
                    ),
                )
            } else {
                format!("  {}  {}", name_padded, dark_gray(&suggestion.description),)
            }
        };
        // CRITICAL: every line fills to terminal width so terminal clears old content
        lines.push(fill_to_width(
            &content,
            width,
            BORDER_GRAY.0,
            BORDER_GRAY.1,
            BORDER_GRAY.2,
        ));
    }

    // Padding line — full width
    lines.push(fg(
        BORDER_GRAY.0,
        BORDER_GRAY.1,
        BORDER_GRAY.2,
        &" ".repeat(width),
    ));

    // Bottom border — full terminal width
    lines.push(fg(
        BORDER_GRAY.0,
        BORDER_GRAY.1,
        BORDER_GRAY.2,
        &"─".repeat(width),
    ));

    lines
}

// ============================================================================
// FRAME 5: Input box — Gruvbox styled
// Top/bottom borders, ❯ prompt, inverted cursor (fg on border)
// ============================================================================

fn render_input_box(input: &TextInput, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Top border — full terminal width, visible on dark terminals
    lines.push(fg(
        BORDER_GRAY.0,
        BORDER_GRAY.1,
        BORDER_GRAY.2,
        &"─".repeat(width),
    ));

    // Content lines with prompt
    let cursor_row = input.cursor_row();
    let cursor_col = input.display_col();

    for (i, line) in input.lines().iter().enumerate() {
        let prefix = if i == 0 {
            fg(
                PROMPT_GRAY.0,
                PROMPT_GRAY.1,
                PROMPT_GRAY.2,
                &format!("{} ", USER_ARROW),
            )
        } else {
            "  ".to_string()
        };

        // Cursor: Gruvbox fg on border gray — visible on dark terminals
        let line_with_cursor = if i == cursor_row {
            let before = substring_to_display_width(line, cursor_col);
            let after_byte = byte_col_for_display_width(line, cursor_col);
            let after = &line[after_byte..];
            // Gruvbox fg block cursor on bg3
            format!(
                "{}{}{}",
                before,
                fg_bg_bold(
                    FG.0,
                    FG.1,
                    FG.2,
                    BORDER_GRAY.0,
                    BORDER_GRAY.1,
                    BORDER_GRAY.2,
                    BLOCK_CURSOR
                ),
                after,
            )
        } else {
            line.to_string()
        };

        lines.push(format!("{}{}", prefix, line_with_cursor));
    }

    // Bottom border — full terminal width
    lines.push(fg(
        BORDER_GRAY.0,
        BORDER_GRAY.1,
        BORDER_GRAY.2,
        &"─".repeat(width),
    ));

    lines
}

// ============================================================================
// FRAME 6: Status bar — Gruvbox Dark Hard
// ============================================================================

fn render_status_bar(footer: &FooterMetrics, header: &HeaderInfo, width: usize) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Session name removed — clutter reduction

    // Provider/model — Gruvbox blue
    let mut model_parts: Vec<String> = Vec::new();
    if !header.model.is_empty() {
        model_parts.push(fg(BLUE.0, BLUE.1, BLUE.2, &header.model));
    }
    if !header.endpoint.is_empty() {
        let short_endpoint = if let Some(host) = header.endpoint.split("://").nth(1) {
            host
        } else {
            &header.endpoint
        };
        model_parts.push(fg(BLUE.0, BLUE.1, BLUE.2, short_endpoint));
    }
    if !model_parts.is_empty() {
        parts.push(model_parts.join(&format!(" {} ", dark_gray(MIDDOT))));
    }

    // Workspace
    if !header.workspace.is_empty() {
        parts.push(fg(BLUE.0, BLUE.1, BLUE.2, &header.workspace));
    }

    // Approval policy
    if !footer.approval_policy.is_empty() {
        let policy_text = match footer.approval_policy.as_str() {
            "yolo" => format!("{} {}", LIGHTNING, red("yolo")),
            "auto" => format!("{} {}", LIGHTNING, fg(ORANGE.0, ORANGE.1, ORANGE.2, "auto")),
            _ => format!("{} {}", LOCK, dark_gray("approve")),
        };
        parts.push(policy_text);
    }

    // Context usage — bold, color-coded with token counts
    if footer.context_max > 0 {
        let pct = footer.context_current as f64 / footer.context_max as f64 * 100.0;
        let ctx_color = if pct > 90.0 {
            fg(RED.0, RED.1, RED.2, &format!("{:.0}% ctx", pct))
        } else if pct > 70.0 {
            fg(YELLOW.0, YELLOW.1, YELLOW.2, &format!("{:.0}% ctx", pct))
        } else {
            fg(GREEN.0, GREEN.1, GREEN.2, &format!("{:.0}% ctx", pct))
        };
        parts.push(bold(&ctx_color));

        // Token counts — dim gray
        if footer.tokens_in > 0 || footer.tokens_out > 0 {
            let token_fmt = format!(
                "↑{} ↓{}",
                format_tokens(footer.tokens_in),
                format_tokens(footer.tokens_out)
            );
            parts.push(dim(&token_fmt));
        }
    }

    // Effort timer — if active
    if !footer.effort.is_empty() {
        parts.push(footer.effort.clone());
    }

    if header.verbose {
        parts.push(fg(ORANGE.0, ORANGE.1, ORANGE.2, "[verbose]"));
    }

    let status = parts.join(&format!(" {} ", dark_gray(MIDDOT)));

    let status_dw = display_width(&status);
    if status_dw >= width {
        truncate_to_display_width(&status, width - 1)
    } else {
        status
    }
}

// ============================================================================
// Display-width helpers
// ============================================================================

/// Format token counts with k suffix for large numbers
fn format_tokens(count: u64) -> String {
    if count >= 1000 {
        format!("{:.1}k", count as f64 / 1000.0)
    } else {
        count.to_string()
    }
}

fn substring_to_display_width(s: &str, target_display_width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;
    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) as usize;
        if current_width + char_width > target_display_width {
            break;
        }
        result.push(c);
        current_width += char_width;
    }
    result
}

fn byte_col_for_display_width(s: &str, target_display_col: usize) -> usize {
    let mut byte_pos = 0;
    let mut display_pos = 0;
    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) as usize;
        if display_pos >= target_display_col {
            break;
        }
        if display_pos + char_width > target_display_col {
            break;
        }
        byte_pos += c.len_utf8();
        display_pos += char_width;
    }
    byte_pos
}

fn truncate_to_display_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;
    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) as usize;
        if current_width + char_width > max_width {
            break;
        }
        result.push(c);
        current_width += char_width;
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_autocomplete::AutocompleteSuggestion;

    #[test]
    fn test_render_user_message() {
        let lines = render_user_message("hello world", 78);
        assert!(!lines.is_empty());
        assert!(lines[0].contains(USER_ARROW));
    }

    #[test]
    fn test_render_assistant_message() {
        let lines = render_assistant_message("Hello **bold** world", 78);
        assert!(!lines.is_empty());
        assert!(lines[0].contains(ASSISTANT_DOT));
    }

    #[test]
    fn test_render_status_bar() {
        let footer = FooterMetrics {
            model: "qwen3:4b".to_string(),
            context_current: 4096,
            context_max: 8192,
            tokens_in: 500,
            tokens_out: 200,
            effort: String::new(),
            route: String::new(),
            approval_policy: "auto".to_string(),
        };
        let header = HeaderInfo {
            model: "qwen3:4b".to_string(),
            endpoint: "localhost:8080".to_string(),
            route: String::new(),
            workspace: "elma-cli".to_string(),
            session: "default".to_string(),
            verbose: false,
        };
        let line = render_status_bar(&footer, &header, 80);
        // Session name removed from footer for cleaner UI
        assert!(line.contains("qwen3"));
        assert!(line.contains("50% ctx"));
        assert!(line.contains("↑500"));
        assert!(line.contains("↓200"));
    }

    #[test]
    fn test_render_screen_80x24() {
        let state = UIState::new();
        let input = TextInput::new(10);
        let result = render_screen(&state, 80, 24, &input);
        assert!(result.lines.len() >= 24);
    }

    #[test]
    fn test_empty_transcript() {
        let state = UIState::new();
        let input = TextInput::new(10);
        let result = render_screen(&state, 80, 24, &input);
        assert!(result.lines.iter().any(|l| l.contains("Type a message")));
    }

    #[test]
    fn test_autocomplete_dropdown_full_width() {
        let mut ac = AutocompleteState::new();
        ac.active = true;
        ac.is_emoji = false;
        ac.matches.push(AutocompleteSuggestion {
            label: "/help".to_string(),
            description: "Show help screen".to_string(),
        });
        ac.selected = 0;
        let lines = render_autocomplete_dropdown(&ac, 170);
        assert!(!lines.is_empty());
        assert!(lines.first().map(|l| l.contains("─")).unwrap_or(false));
        assert!(lines.last().map(|l| l.contains("─")).unwrap_or(false));
    }
}
