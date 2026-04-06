//! @efficiency-role: ui-component
//!
//! Structured Layout — Claude Code-Inspired
//!
//! Design (from Claude Code study):
//! - `●` dot prefix for assistant messages (no heavy borders)
//! - Tool display: [dot] TOOL_NAME (details)
//! - Status line: dim text with `·` separators
//! - No response-level borders — content speaks for itself
//! - Full output always shown — never truncated

use crate::ui_colors::*;
use std::io::IsTerminal;

/// Terminal width.
pub(crate) fn term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// Blockquote bar character.
pub(crate) const BLOCKQUOTE_BAR: &str = "▎";

/// Render a horizontal separator (for markdown `---`).
pub(crate) fn render_hr() -> String {
    meta_comment(&"─".repeat(term_width().min(80)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hr() {
        let output = render_hr();
        assert!(output.contains("─"));
    }
}
