//! @efficiency-role: ui-component
//!
//! Elma Visual Design System — Gruvbox Dark Hard
//!
//! Gruvbox Dark Hard (extra contrast) — the ONLY color palette used.
//! https://github.com/morhetz/gruvbox
//!
//! Warm tones, high contrast, easy on the eyes during long sessions.
//! All UI modules reference this file for color consistency.

use ratatui::style::Color;

// ============================================================================
// Gruvbox Dark Hard Palette
// ============================================================================

pub(crate) mod colors {
    use super::Color;

    /// #fabd2f — Yellow (prompts, tool names, warnings)
    pub(crate) const YELLOW: Color = Color::Rgb(250, 189, 47);
    /// #fb4934 — Red (errors, failures, destructive blocks)
    pub(crate) const RED: Color = Color::Rgb(251, 73, 52);
    /// #b8bb26 — Green (success, confirmations, safe operations)
    pub(crate) const GREEN: Color = Color::Rgb(184, 187, 38);
    /// #83a598 — Blue (tool execution, informational messages)
    pub(crate) const BLUE: Color = Color::Rgb(131, 165, 152);
    /// #d3869b — Purple (accent, inline code, Elma prefix)
    pub(crate) const PURPLE: Color = Color::Rgb(211, 134, 155);
    /// #8ec07c — Aqua (secondary accent, tool badges)
    pub(crate) const AQUA: Color = Color::Rgb(142, 192, 124);
    /// #fe8019 — Orange (highlights, warnings, important markers)
    pub(crate) const ORANGE: Color = Color::Rgb(254, 128, 25);
    /// #ebdbb2 — Fg (primary text, normal output)
    pub(crate) const FG: Color = Color::Rgb(235, 219, 178);
    /// #928374 — Gray (metadata, timestamps, dim text)
    pub(crate) const GRAY: Color = Color::Rgb(146, 131, 116);
    /// #1d2021 — Bg Hard (background — terminal provides this)
    #[allow(dead_code)]
    pub(crate) const BG_HARD: Color = Color::Rgb(29, 32, 33);
}

/// ANSI escape code for a Gruvbox color.
pub(crate) fn ansi_24bit(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

// ============================================================================
// Public Color Functions (used throughout the codebase)
// ============================================================================

/// Elma accent / primary — Purple (for `●` prefix, inline code)
pub(crate) fn elma_accent(s: &str) -> String {
    ansi_24bit(211, 134, 155, s)
}

/// Tool execution / info — Blue
pub(crate) fn info_cyan(s: &str) -> String {
    ansi_24bit(131, 165, 152, s)
}

/// Errors / failures — Red
pub(crate) fn error_red(s: &str) -> String {
    ansi_24bit(251, 73, 52, s)
}

/// Warnings / caution / prompts — Yellow
pub(crate) fn warn_yellow(s: &str) -> String {
    ansi_24bit(250, 189, 47, s)
}

/// Success / confirmations — Green
pub(crate) fn success_green(s: &str) -> String {
    ansi_24bit(184, 187, 38, s)
}

/// Primary text — Fg
pub(crate) fn text_white(s: &str) -> String {
    ansi_24bit(235, 219, 178, s)
}

/// Metadata / dim text — Gray
pub(crate) fn meta_comment(s: &str) -> String {
    ansi_24bit(146, 131, 116, s)
}

/// Orange — highlights and important markers
pub(crate) fn gruvbox_orange(s: &str) -> String {
    ansi_24bit(254, 128, 25, s)
}

/// Aqua — secondary accent
pub(crate) fn gruvbox_aqua(s: &str) -> String {
    ansi_24bit(142, 192, 124, s)
}

// ============================================================================
// ratatui Color Mappings
// ============================================================================

pub(crate) fn ratatui_accent() -> Color { colors::PURPLE }
pub(crate) fn ratatui_info() -> Color { colors::BLUE }
pub(crate) fn ratatui_error() -> Color { colors::RED }
pub(crate) fn ratatui_warning() -> Color { colors::YELLOW }
pub(crate) fn ratatui_success() -> Color { colors::GREEN }
pub(crate) fn ratatui_text() -> Color { colors::FG }
pub(crate) fn ratatui_dim() -> Color { colors::GRAY }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_escape_codes() {
        let result = elma_accent("test");
        assert!(result.contains("\x1b[38;2;211;134;155m"));
        assert!(result.contains("test"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_all_color_functions() {
        assert!(info_cyan("x").contains("\x1b[38;2;131"));
        assert!(error_red("x").contains("\x1b[38;2;251"));
        assert!(warn_yellow("x").contains("\x1b[38;2;250"));
        assert!(success_green("x").contains("\x1b[38;2;184"));
        assert!(text_white("x").contains("\x1b[38;2;235"));
        assert!(meta_comment("x").contains("\x1b[38;2;146"));
    }

    #[test]
    fn test_ratatui_colors() {
        assert_eq!(ratatui_accent(), colors::PURPLE);
        assert_eq!(ratatui_info(), colors::BLUE);
        assert_eq!(ratatui_error(), colors::RED);
        assert_eq!(ratatui_warning(), colors::YELLOW);
        assert_eq!(ratatui_success(), colors::GREEN);
        assert_eq!(ratatui_text(), colors::FG);
        assert_eq!(ratatui_dim(), colors::GRAY);
    }
}
