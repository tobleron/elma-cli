//! @efficiency-role: ui-component
//!
//! Elma Visual Design System — Gruvbox Dark Hard Color Palette
//!
//! Exact RGB values from the Gruvbox Dark Hard theme.
//! All UI modules reference this file for color consistency.
//! See: https://github.com/morhetz/gruvbox

// ============================================================================
// Gruvbox Dark Hard Palette (ANSI 24-bit RGB)
// ============================================================================

// --- Primary palette ---

/// Rgb(29, 32, 33) — Background (hard contrast dark)
pub(crate) const BG_HARD: (u8, u8, u8) = (29, 32, 33);
/// Rgb(235, 219, 178) — Primary foreground text
pub(crate) const FG: (u8, u8, u8) = (235, 219, 178);
/// Rgb(251, 73, 52) — Red: errors, failures, destructive markers
pub(crate) const RED: (u8, u8, u8) = (251, 73, 52);
/// Rgb(184, 187, 38) — Green: success, confirmations, safe operations
pub(crate) const GREEN: (u8, u8, u8) = (184, 187, 38);
/// Rgb(250, 189, 47) — Yellow: warnings, prompts, tool names, headers
pub(crate) const YELLOW: (u8, u8, u8) = (250, 189, 47);
/// Rgb(131, 165, 152) — Blue: tool execution info, informational messages
pub(crate) const BLUE: (u8, u8, u8) = (131, 165, 152);
/// Rgb(211, 134, 155) — Purple: accent, inline code, Elma prefix
pub(crate) const PURPLE: (u8, u8, u8) = (211, 134, 155);
/// Rgb(142, 192, 124) — Aqua: secondary accent, tool badges
pub(crate) const AQUA: (u8, u8, u8) = (142, 192, 124);
/// Rgb(254, 128, 25) — Orange: highlights, warnings, important markers
pub(crate) const ORANGE: (u8, u8, u8) = (254, 128, 25);
/// Rgb(146, 131, 116) — Gray: metadata, dim text, separators
pub(crate) const GRAY: (u8, u8, u8) = (146, 131, 116);

// --- Derived / semantic aliases (Gruvbox-mapped) ---

/// Border/separator gray — Gruvbox bg3 (#665c54)
pub(crate) const BORDER_GRAY: (u8, u8, u8) = (102, 92, 84);
/// Prompt prefix gray — Gruvbox gray (#928374)
pub(crate) const PROMPT_GRAY: (u8, u8, u8) = (146, 131, 116);
/// Very dark gray — Gruvbox bg1 (#3c3836)
pub(crate) const VERY_DARK_GRAY: (u8, u8, u8) = (60, 56, 54);
/// Steel blue equivalent — Gruvbox blue (#83a598)
pub(crate) const STEEL_BLUE: (u8, u8, u8) = (131, 165, 152);
/// Pane green equivalent — Gruvbox green (#b8bb26)
pub(crate) const PANE_GREEN: (u8, u8, u8) = (184, 187, 38);
/// Teal equivalent — Gruvbox aqua (#8ec07c)
pub(crate) const TEAL: (u8, u8, u8) = (142, 192, 124);
/// Selection highlight background — Gruvbox bg2 (#504945)
pub(crate) const SELECT_BG: (u8, u8, u8) = (80, 73, 69);
/// User message background — Gruvbox bg1 (#3c3836)
pub(crate) const USER_BG: (u8, u8, u8) = (60, 56, 54);
/// System message yellow — Gruvbox yellow (#fabd2f)
pub(crate) const SYSTEM_YELLOW: (u8, u8, u8) = (250, 189, 47);

// Named colors for semantic use (Gruvbox-mapped)
/// Cyan equivalent → Gruvbox blue
pub(crate) const CYAN: (u8, u8, u8) = (131, 165, 152);
/// White → Gruvbox fg
pub(crate) const WHITE: (u8, u8, u8) = (235, 219, 178);
/// Dark gray → Gruvbox gray
pub(crate) const DARK_GRAY: (u8, u8, u8) = (146, 131, 116);
/// Black → Gruvbox bg hard
pub(crate) const BLACK: (u8, u8, u8) = (29, 32, 33);

/// ANSI escape code for 24-bit RGB foreground.
pub(crate) fn ansi_24bit(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

// ============================================================================
// Semantic Color Functions
// ============================================================================

/// Elma accent — purple (Gruvbox Purple #d3869b).
pub(crate) fn elma_accent(s: &str) -> String {
    ansi_24bit(PURPLE.0, PURPLE.1, PURPLE.2, s)
}

/// Info text — blue (Gruvbox Blue #83a598).
pub(crate) fn info_cyan(s: &str) -> String {
    ansi_24bit(BLUE.0, BLUE.1, BLUE.2, s)
}

/// Errors — red (Gruvbox Red #fb4934).
pub(crate) fn error_red(s: &str) -> String {
    ansi_24bit(RED.0, RED.1, RED.2, s)
}

/// Warnings — yellow (Gruvbox Yellow #fabd2f).
pub(crate) fn warn_yellow(s: &str) -> String {
    ansi_24bit(YELLOW.0, YELLOW.1, YELLOW.2, s)
}

/// Success — green (Gruvbox Green #b8bb26).
pub(crate) fn success_green(s: &str) -> String {
    ansi_24bit(GREEN.0, GREEN.1, GREEN.2, s)
}

/// Primary text — fg (Gruvbox Fg #ebdbb2).
pub(crate) fn text_white(s: &str) -> String {
    ansi_24bit(FG.0, FG.1, FG.2, s)
}

/// Metadata / dim text — gray (Gruvbox Gray #928374).
pub(crate) fn meta_comment(s: &str) -> String {
    ansi_24bit(GRAY.0, GRAY.1, GRAY.2, s)
}

/// Orange markers (Gruvbox Orange #fe8019).
pub(crate) fn gruvbox_orange(s: &str) -> String {
    ansi_24bit(ORANGE.0, ORANGE.1, ORANGE.2, s)
}

/// Aqua accent (Gruvbox Aqua #8ec07c).
pub(crate) fn gruvbox_aqua(s: &str) -> String {
    ansi_24bit(AQUA.0, AQUA.1, AQUA.2, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_escape_codes() {
        let result = ansi_24bit(251, 73, 52, "test");
        assert!(result.contains("\x1b[38;2;251;73;52m"));
        assert!(result.contains("test"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_all_color_functions() {
        // elma_accent = purple
        assert!(elma_accent("x").contains("\x1b[38;2;211;134;155m"));
        // info_cyan = blue
        assert!(info_cyan("x").contains("\x1b[38;2;131;165;152m"));
        // error_red = red
        assert!(error_red("x").contains("\x1b[38;2;251;73;52m"));
        // warn_yellow = yellow
        assert!(warn_yellow("x").contains("\x1b[38;2;250;189;47m"));
        // success_green = green
        assert!(success_green("x").contains("\x1b[38;2;184;187;38m"));
        // text_white = fg
        assert!(text_white("x").contains("\x1b[38;2;235;219;178m"));
        // meta_comment = gray
        assert!(meta_comment("x").contains("\x1b[38;2;146;131;116m"));
    }

    #[test]
    fn test_gruvbox_constants() {
        assert_eq!(RED, (251, 73, 52));
        assert_eq!(GREEN, (184, 187, 38));
        assert_eq!(YELLOW, (250, 189, 47));
        assert_eq!(BLUE, (131, 165, 152));
        assert_eq!(PURPLE, (211, 134, 155));
        assert_eq!(AQUA, (142, 192, 124));
        assert_eq!(ORANGE, (254, 128, 25));
        assert_eq!(GRAY, (146, 131, 116));
        assert_eq!(FG, (235, 219, 178));
        assert_eq!(BG_HARD, (29, 32, 33));
    }

    #[test]
    fn test_gruvbox_derived_aliases() {
        assert_eq!(BORDER_GRAY, (102, 92, 84));
        assert_eq!(PROMPT_GRAY, (146, 131, 116));
        assert_eq!(STEEL_BLUE, (131, 165, 152));
        assert_eq!(PANE_GREEN, (184, 187, 38));
        assert_eq!(TEAL, (142, 192, 124));
        assert_eq!(SYSTEM_YELLOW, (250, 189, 47));
    }
}
