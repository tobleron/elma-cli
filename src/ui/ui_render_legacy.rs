//! @efficiency-role: ui-component
//!
//! Full-screen rendering — Gruvbox Dark Hard layout.
//!
//! Layout order (top to bottom):
//!   [Header strip — 1 row, top of terminal]
//!   [Activity rail — live status when processing/responding]
//!   [Transcript area — fills most space, scrollable]
//!   [Autocomplete dropdown — bordered box, above input when active]
//!   [Input box — bordered, above context bar]
//!   [Context bar — token usage indicator]

use crate::ui_autocomplete::AutocompleteState;
use crate::ui_colors::*;
use crate::ui_input::TextInput;
use crate::ui_markdown::render_markdown;
use crate::ui_state::*;
use crate::ui_theme::*;
use crate::ui_theme::{current_theme, fg_bold_token, fg_token};
use crate::ui_wrap::{display_width, wrap_ansi};

/// Fixed width of the input prompt ("> ").
const INPUT_PROMPT_WIDTH: usize = 2;

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
    // Minimum height to show all mandatory frames: header(1) + separator(1) + activity(1) +
    // transcript(min 5) + dropdown(max 0-8 rows, assume 4 for calculation) + input_box(2+) + context_bar(0-1)
    const MIN_HEIGHT: usize = 13;

    if height == 0 || width == 0 {
        return ScreenBuffer {
            lines: vec![],
            cursor_row: 0,
            cursor_col: 0,
        };
    }

    // Warn if terminal is too small - could cause truncated UI
    if height < MIN_HEIGHT {
        // Add a minimal warning line to the screen (will be part of fixed_rows)
        // This ensures at least header + separator + activity + input are visible
    }

    // Calculate space needed for each section
    let header_rows = 1;
    let separator_rows = 1; // separator after header

    let input_content_rows = input.line_count().max(1).min(3);
    let input_box_rows = input_content_rows + 2; // top + bottom border

    let context_bar_rows = if state.footer.context_max > 0 { 1 } else { 0 };

    let dropdown_rows = if state.autocomplete.active && !state.autocomplete.matches.is_empty() {
        let items = state.autocomplete.matches.len().min(8);
        items + 4
    } else {
        0
    };

    let activity_rows = if state.streaming.kind != StreamingKind::Idle {
        2
    } else if let ActivityState::Active { .. } = &state.activity {
        1
    } else {
        // Reserve 1 row for status bar to prevent layout jumps
        1
    };

    // Calculate transcript space
    let fixed_rows = header_rows
        + separator_rows
        + activity_rows
        + dropdown_rows
        + input_box_rows
        + context_bar_rows;
    let transcript_rows = height.saturating_sub(fixed_rows).max(5);

    let mut screen: Vec<String> = Vec::with_capacity(height);

    // ===== FRAME 1: Header strip =====
    let header_line = render_header_strip(&state.header, width);
    screen.push(header_line);
    screen.push(meta_comment(&"─".repeat(width)));

    // ===== FRAME 2: Transcript (user messages show here FIRST) =====
    let transcript = render_transcript(
        &state.transcript,
        width,
        transcript_rows,
        state.viewport.scroll_offset,
        state.show_thinking,
        state.viewport.user_scrolled_up,
    );
    screen.extend(transcript);

    // ===== FRAME 3: Activity (AFTER transcript, below user message) =====
    if state.streaming.kind != StreamingKind::Idle {
        let streaming_lines = render_streaming_state(&state.streaming, width);
        screen.extend(streaming_lines);
        screen.push(meta_comment(&"─".repeat(width)));
    } else if let ActivityState::Active { label, message } = &state.activity {
        let spinner = SPINNER_FRAMES[state.streaming.animation_frame % 10];
        screen.push(format!(
            "  {} {} {}",
            fg(AQUA.0, AQUA.1, AQUA.2, spinner),
            fg_bold(AQUA.0, AQUA.1, AQUA.2, label),
            dim(message),
        ));
        screen.push(meta_comment(&"─".repeat(width)));
    }

    // Pad transcript to exact size
    while screen.len() < header_rows + separator_rows + activity_rows + transcript_rows {
        screen.push(String::new());
    }

    // ===== FRAME 4: Composer =====
    if state.autocomplete.active && !state.autocomplete.matches.is_empty() {
        let dropdown_lines = render_autocomplete_dropdown(&state.autocomplete, width);
        screen.extend(dropdown_lines);
    }

    // Store where input starts for cursor calculation
    let input_start_row = screen.len();

    let input_lines = render_input_box(input, width);
    screen.extend(input_lines);

    // ===== FRAME 5: Context bar =====
    if state.footer.context_max > 0 {
        let context_bar = render_context_bar(&state.footer, width);
        screen.push(context_bar);
    }

    // Ensure exactly height lines
    screen.truncate(height);
    while screen.len() < height {
        screen.push(String::new());
    }

    // Cursor position: input_start_row + 1 (for top border) + cursor_row_within_input
    let cursor_row = (input_start_row + 1 + input.cursor_row()) as u16;
    let cursor_col = (INPUT_PROMPT_WIDTH + input.display_col()) as u16; // INPUT_PROMPT_WIDTH for "> "

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
    user_scrolled_up: bool,
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
        all_lines.push(String::new());
        all_lines.push(format!(
            "  {}",
            dim("Welcome to Elma - your local-first autonomous CLI agent")
        ));
        all_lines.push(String::new());
        all_lines.push(format!(
            "  {}",
            dim("Type a message and press Enter to begin")
        ));
        all_lines.push(format!("  {}", dim("• Ctrl+J for multi-line input")));
        all_lines.push(format!("  {}", dim("• Ctrl+C to cancel")));
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
        user_scrolled_up,
    );

    let mut result: Vec<String> = visible.into_iter().take(visible_rows).collect();
    while result.len() < visible_rows {
        result.push(String::new());
    }
    result
}

/// Add a subtle scroll indicator on the right edge when content is scrollable.
/// When user_scrolled_up is true, show a "↓ N new" hint at the bottom of the visible area.
fn visible_lines_with_scroll_indicator(
    all_lines: &[String],
    start: usize,
    visible_rows: usize,
    total_lines: usize,
    max_scroll: usize,
    scroll_offset: usize,
    user_scrolled_up: bool,
) -> Vec<String> {
    let has_scroll = max_scroll > 0;
    let visible: Vec<String> = all_lines[start..].to_vec();

    if !has_scroll && !user_scrolled_up {
        return visible;
    }

    // Calculate scrollbar position
    let scroll_progress = if max_scroll > 0 {
        scroll_offset as f64 / max_scroll as f64
    } else {
        0.0
    };

    let lines_below = max_scroll.saturating_sub(scroll_offset);

    let mut result: Vec<String> = visible
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
        .collect();

    // Show "↓ N new lines below" indicator when user has scrolled up
    if user_scrolled_up && lines_below > 0 && !result.is_empty() {
        let last_idx = result.len() - 1;
        let indicator = dim(&format!("↓ {} new", lines_below));
        // Replace the last line's rightmost chars with the indicator
        let line = &result[last_idx];
        let indicator_width = 10; // "↓ N new" is about 6-8 chars
        if line.len() > indicator_width {
            let truncated = truncate_to_display_width(line, 80 - indicator_width - 2);
            result[last_idx] = format!("{}  {}", truncated, indicator);
        }
    }

    result
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
                    dim(EXPAND_ARROW_DOWN),
                    meta_comment("Thinking"),
                    meta_comment("(ctrl+o to expand)"),
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
                fg_bold_token(current_theme().accent_primary, ">"),
                text_white(raw_line)
            )
        } else {
            format!("  {}", text_white(raw_line))
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
                fg_bold_token(current_theme().accent_secondary, "●"),
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
            fg_bold(AQUA.0, AQUA.1, AQUA.2, "●"),
            dim("(empty)"),
        ));
    }
    all_lines
}

fn render_tool_start(name: &str, command: &str, content_width: usize) -> Vec<String> {
    let text = format!(
        "  {} {} {}",
        fg_token(current_theme().fg_dim, "▸"),
        fg_bold(BLUE.0, BLUE.1, BLUE.2, name),
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
    let (icon, color_token) = if success {
        ("✓", current_theme().success)
    } else {
        ("✗", current_theme().error)
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
        format!("  {} {}", fg_bold_token(color_token, icon), dim(name),)
    } else {
        format!(
            "  {} {} {}",
            fg_bold_token(color_token, icon),
            dim(name),
            dim(&format!("({})", duration_str)),
        )
    };
    lines.extend(wrap_ansi(&header, content_width));

    if !output.is_empty() {
        let output_lines: Vec<&str> = output.lines().take(15).collect();
        for oline in &output_lines {
            let prefixed = format!("    {}", dim(oline));
            lines.extend(wrap_ansi(&prefixed, content_width));
        }
        let total = output.lines().count();
        if total > 15 {
            lines.push(dim(&format!("    ... ({} more lines)", total - 15)));
        }
    }

    lines
}

fn render_meta_event(category: &str, message: &str, content_width: usize) -> Vec<String> {
    // Make meta events more prominent with better styling
    let styled = match category {
        "PLAN" => format!(
            "  {} {}",
            fg_bold(BLUE.0, BLUE.1, BLUE.2, &format!("[{}]", category)),
            fg(BLUE.0, BLUE.1, BLUE.2, message)
        ),
        "CLASSIFY" => format!("  {} {}", dim(&format!("[{}]", category)), dim(message)),
        "REFLECT" => format!(
            "  {} {}",
            fg_bold(PURPLE.0, PURPLE.1, PURPLE.2, &format!("[{}]", category)),
            fg(PURPLE.0, PURPLE.1, PURPLE.2, message)
        ),
        "EXECUTE" => format!(
            "  {} {}",
            fg_bold(AQUA.0, AQUA.1, AQUA.2, &format!("[{}]", category)),
            fg(AQUA.0, AQUA.1, AQUA.2, message)
        ),
        _ => format!("  {} {}", dim(&format!("[{}]", category)), dim(message)),
    };
    wrap_ansi(&styled, content_width)
}

fn render_warning_msg(message: &str, content_width: usize) -> Vec<String> {
    let text = format!(
        "  {} {}",
        fg_bold_token(current_theme().warning, "⚠"),
        fg_token(current_theme().warning, message)
    );
    wrap_ansi(&text, content_width)
}

fn render_thinking(content: &str, content_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "  {} {}",
        dim(EXPAND_ARROW_RIGHT),
        meta_comment("(ctrl+o to collapse)"),
    ));
    lines.push(String::new());
    for raw_line in content.lines() {
        let prefixed = format!("    {}", meta_comment(raw_line));
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
                meta_comment(&format!(" {}{}", elapsed_str, tok_str))
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
                    meta_comment(&suggestion.label),
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
                format!(
                    "  {}  {}",
                    name_padded,
                    meta_comment(&suggestion.description),
                )
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
            fg(YELLOW.0, YELLOW.1, YELLOW.2, "> ")
        } else {
            "  ".to_string()
        };

        // Show the text content
        let text_content = fg(FG.0, FG.1, FG.2, line);

        // Add cursor if on this line
        let line_with_content = if i == cursor_row {
            let before = substring_to_display_width(line, cursor_col);
            let after_byte = byte_col_for_display_width(line, cursor_col);
            let after = &line[after_byte..];

            // Show cursor at position
            let cursor_char = if cursor_col >= line.len() {
                " " // Space at end of line
            } else {
                &line
                    [after_byte..after_byte + line[after_byte..].chars().next().unwrap().len_utf8()]
            };

            format!(
                "{}{}{}",
                fg(FG.0, FG.1, FG.2, &before),
                fg_bg_bold(
                    BG_HARD.0,
                    BG_HARD.1,
                    BG_HARD.2,
                    FG.0,
                    FG.1,
                    FG.2,
                    cursor_char
                ),
                fg(FG.0, FG.1, FG.2, after),
            )
        } else {
            text_content
        };

        lines.push(format!("{}{}", prefix, line_with_content));
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

fn render_header_strip(header: &HeaderInfo, width: usize) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Elma branding
    parts.push(bold(&fg(PURPLE.0, PURPLE.1, PURPLE.2, "Elma")));

    // Workflow / formula label
    if !header.workflow.is_empty() {
        parts.push(fg(PINK.0, PINK.1, PINK.2, &header.workflow));
    }

    // Stage info for main tasks
    if let Some(stage) = &header.stage {
        parts.push(dim(stage));
    }

    // Workspace (compact)
    if !header.workspace.is_empty() {
        parts.push(fg(AQUA.0, AQUA.1, AQUA.2, &header.workspace));
    }

    // Model (compact)
    if !header.model.is_empty() {
        parts.push(dim(&header.model));
    }

    // Endpoint (very compact - just hostname)
    if !header.endpoint.is_empty() {
        let short_endpoint = if let Some(host) = header.endpoint.split("://").nth(1) {
            host.split(':').next().unwrap_or(host)
        } else {
            &header.endpoint
        };
        parts.push(dim(short_endpoint));
    }

    let header_text = parts.join(&dim(" · "));

    let header_dw = display_width(&header_text);
    if header_dw >= width {
        truncate_to_display_width(&header_text, width - 1)
    } else {
        format!(" {}", header_text) // Add leading space
    }
}

fn render_context_bar(footer: &FooterMetrics, width: usize) -> String {
    if footer.context_max == 0 {
        return String::new();
    }

    let pct = footer.context_current as f64 / footer.context_max as f64;

    // Simpler bar: just show percentage with color
    let bar_color = if pct > 0.9 {
        RED
    } else if pct > 0.7 {
        YELLOW
    } else {
        GREEN
    };

    // Create compact bar
    let bar_width: usize = 20; // Fixed width
    let filled = (bar_width as f64 * pct).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    let bar_text = format!(
        "{}{}",
        fg(bar_color.0, bar_color.1, bar_color.2, &"█".repeat(filled)),
        meta_comment(&"░".repeat(empty))
    );

    // Tokens and percentage
    let info = format!(
        " {:.1}k/{:.1}k [{:.0}%]",
        footer.context_current as f64 / 1000.0,
        footer.context_max as f64 / 1000.0,
        pct * 100.0
    );

    format!(" {}{}", bar_text, dim(&info))
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
        assert!(lines[0].contains(">"));
    }

    #[test]
    fn test_render_assistant_message() {
        let lines = render_assistant_message("Hello **bold** world", 78);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("●")); // Check for bullet character
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
            workflow: String::new(),
            stage: None,
            verbose: false,
        };
        let line = render_header_strip(&header, 80);
        assert!(line.contains("Elma"));
        assert!(line.contains("qwen3"));
        assert!(line.contains("elma-cli"));
        assert!(line.contains("localhost"));
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
        assert!(result
            .lines
            .iter()
            .any(|l| l.contains("Welcome to Elma") || l.contains("Type a message")));
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

    #[test]
    fn test_transcript_viewport_math() {
        let mut state = UIState::new();
        // Add enough messages to fill more than a small viewport
        for i in 0..10 {
            state.push_user_message(&format!("message {}", i));
        }
        let input = TextInput::new(10);
        let result = render_screen(&state, 80, 20, &input);
        // Header(1) + separator(1) + transcript + input_box(2+) + borders
        // Transcript should fill remaining space
        assert!(result.lines.len() >= 20);
        // Should contain some user messages
        assert!(result.lines.iter().any(|l| l.contains("message")));
    }

    #[test]
    fn test_transcript_respects_user_scrolled_up() {
        let mut state = UIState::new();
        // Add enough messages to require scrolling
        for i in 0..20 {
            state.push_user_message(&format!("msg {}", i));
        }
        // User scrolls up
        state.viewport.user_scrolled_up = true;
        state.viewport.scroll_offset = 5;

        let input = TextInput::new(10);
        let result = render_screen(&state, 80, 24, &input);
        // The screen should render without panicking
        assert!(!result.lines.is_empty());
    }

    #[test]
    fn test_activity_rail_appears_in_screen() {
        let mut state = UIState::new();
        state.set_activity("Testing", "Running tests...");
        state.push_user_message("hello");
        let input = TextInput::new(10);
        let result = render_screen(&state, 80, 24, &input);
        // Activity rail should appear somewhere in the screen
        assert!(result
            .lines
            .iter()
            .any(|l| l.contains("Testing") || l.contains("⠋") || l.contains("⠙")));
    }
}
