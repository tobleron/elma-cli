//! @efficiency-role: ui-component
//!
//! ANSI-safe text wrapping.
//!
//! - Does not count ANSI escape sequences toward display width.
//! - Does not break inside escape sequences.
//! - Preserves active formatting across wrapped lines.
//! - Uses unicode-width for accurate cell-width calculation.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Wrap an ANSI-encoded string to fit the given display width.
///
/// Returns a Vec of display lines, each no wider than `max_width`
/// in visible cells. ANSI escape sequences are preserved and
/// reopened on each new line so formatting continues correctly.
pub(crate) fn wrap_ansi(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut display_width: usize = 0;

    // Track pending ANSI sequence for current formatting state.
    // When we wrap, we close the current line with "\x1b[0m" and
    // prepend the pending style to the next line.
    let mut pending_style = String::new();

    let mut in_escape = false;
    let mut escape_buf = String::new();

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if ch == '\x1b' {
            in_escape = true;
            escape_buf.clear();
            escape_buf.push(ch);
            i += 1;
            continue;
        }

        if in_escape {
            escape_buf.push(ch);
            if ch == 'm' {
                in_escape = false;
                // This escape sequence is complete.
                // If we're in the middle of the line, apply it.
                // If we're at a line boundary, stash it as pending.
                if display_width == 0 && current_line.is_empty() {
                    // At the start of a new logical line — just append.
                    current_line.push_str(&escape_buf);
                } else {
                    // We're mid-line. Check if this is a style change.
                    // Stash the latest SGR code (the last \x1b[...m sequence).
                    pending_style = escape_buf.clone();
                    current_line.push_str(&escape_buf);
                }
                escape_buf.clear();
            }
            i += 1;
            continue;
        }

        // Regular character.
        let char_width = ch.width().unwrap_or(1);

        // Check if adding this character would exceed the width.
        if display_width + char_width > max_width && display_width > 0 {
            // Wrap: close current line, start new one.
            if !current_line.ends_with("\x1b[0m") {
                current_line.push_str("\x1b[0m");
            }
            lines.push(current_line);
            current_line = String::new();

            // Reopen pending style on new line.
            if !pending_style.is_empty() {
                current_line.push_str(&pending_style);
            }

            display_width = 0;
        }

        // Special: hard newline in source — force line break.
        if ch == '\n' {
            if !current_line.ends_with("\x1b[0m") {
                current_line.push_str("\x1b[0m");
            }
            lines.push(current_line);
            current_line = String::new();
            if !pending_style.is_empty() {
                current_line.push_str(&pending_style);
            }
            display_width = 0;
            i += 1;
            continue;
        }

        current_line.push(ch);
        display_width += char_width;
        i += 1;
    }

    // Flush pending escape buffer.
    if !escape_buf.is_empty() {
        current_line.push_str(&escape_buf);
    }

    // Flush last line.
    if !current_line.is_empty() {
        // Ensure line is properly terminated.
        if !current_line.ends_with("\x1b[0m") {
            // Don't add reset if the line ends mid-escape (shouldn't happen)
            // or if it's already styled (the style will carry).
            // We add reset to be safe.
            if !in_escape {
                current_line.push_str("\x1b[0m");
            }
        }
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Compute the display width of a string, ignoring ANSI escape sequences.
pub(crate) fn display_width(text: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;

    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
            continue;
        }
        width += ch.width().unwrap_or(1);
    }

    width
}

/// Strip all ANSI escape sequences from a string, returning plain text.
pub(crate) fn strip_ansi(text: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
            continue;
        }
        result.push(ch);
    }

    result
}

/// Pad a string (with ANSI escapes) to the given display width.
/// The padding is added after the visible content.
pub(crate) fn pad_to_width(text: &str, target_width: usize) -> String {
    let current = display_width(text);
    if current >= target_width {
        return text.to_string();
    }
    let padding = " ".repeat(target_width - current);
    format!("{}{}", text, padding)
}

/// Truncate a string (with ANSI escapes) to the given display width.
/// Preserves the last active style.
pub(crate) fn truncate_ansi(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }

    let mut result = String::new();
    let mut width = 0;
    let mut in_escape = false;
    let mut escape_buf = String::new();

    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
            escape_buf.clear();
            escape_buf.push(ch);
            continue;
        }
        if in_escape {
            escape_buf.push(ch);
            if ch == 'm' {
                in_escape = false;
                result.push_str(&escape_buf);
                escape_buf.clear();
            }
            continue;
        }

        let char_width = ch.width().unwrap_or(1);
        if width + char_width > max_width {
            break;
        }
        result.push(ch);
        width += char_width;
    }

    // Close styling.
    if !result.ends_with("\x1b[0m") {
        result.push_str("\x1b[0m");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_plain_text() {
        let lines = wrap_ansi("hello world this is a test", 10);
        assert!(lines.len() >= 3);
        for line in &lines {
            assert!(display_width(line) <= 10);
        }
    }

    #[test]
    fn test_wrap_ansi_preserves_formatting() {
        let text = "\x1b[1;38;2;250;189;47mhello world\x1b[0m";
        let lines = wrap_ansi(text, 5);
        assert!(lines.len() >= 3);
        // Each line should be properly terminated
        for line in &lines {
            assert!(line.ends_with("\x1b[0m"));
            assert!(display_width(line) <= 5);
        }
    }

    #[test]
    fn test_display_width_ignores_ansi() {
        let text = "\x1b[1;38;2;250;189;47mhello\x1b[0m";
        assert_eq!(display_width(text), 5);
    }

    #[test]
    fn test_display_width_unicode() {
        // Emoji typically has width 2
        assert_eq!(display_width("🚀"), 2);
        assert_eq!(display_width("hi"), 2);
    }

    #[test]
    fn test_strip_ansi() {
        let text = "\x1b[1mhello\x1b[0m";
        assert_eq!(strip_ansi(text), "hello");
    }

    #[test]
    fn test_wrap_empty() {
        let lines = wrap_ansi("", 80);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_empty());
    }

    #[test]
    fn test_wrap_zero_width() {
        let lines = wrap_ansi("hello", 0);
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn test_pad_to_width() {
        let text = "hello";
        let padded = pad_to_width(text, 10);
        assert_eq!(display_width(&padded), 10);
        assert!(padded.starts_with("hello"));
    }

    #[test]
    fn test_truncate_ansi() {
        let text = "\x1b[1mhello world\x1b[0m";
        let truncated = truncate_ansi(text, 5);
        assert_eq!(display_width(&truncated), 5);
        assert!(truncated.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_wrap_long_ansi_sequence() {
        // Multiple style changes in one string
        let text = "\x1b[1mbold\x1b[0m and \x1b[38;2;250;189;47myellow\x1b[0m";
        let lines = wrap_ansi(text, 8);
        assert!(lines.len() >= 2);
        // Verify each line is within width
        for line in &lines {
            assert!(display_width(line) <= 8, "line '{}' too wide", line);
        }
    }
}
