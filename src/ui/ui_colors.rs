//! @efficiency-role: ui-component
//!
//! Elma Visual Design System — Tokenized Theme (Legacy Compatibility)
//!
//! This file provides backward compatibility for old Gruvbox constant names.
//! New code should use ui_theme.rs tokens directly.
//! Active interactive UI must not import Gruvbox constants.

use crate::ui_theme::{
    current_theme, elma_accent, error_red, info_cyan, meta_comment, success_green, text_white,
    warn_yellow, ColorToken,
};

// ============================================================================
// Theme-based RGB Constants (Legacy Gruvbox Compatibility)
// ============================================================================

// --- Primary palette (mapped to theme tokens) ---

/// Background (hard contrast dark) — now black
pub(crate) fn bg_hard() -> (u8, u8, u8) {
    (0, 0, 0)
}
/// Primary foreground text — theme fg
pub(crate) fn fg_color() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().fg;
    (r, g, b)
}
/// Red: errors, failures — theme error
pub(crate) fn red() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().error;
    (r, g, b)
}
/// Green: success — theme success
pub(crate) fn green() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().success;
    (r, g, b)
}
/// Yellow: warnings — theme warning
pub(crate) fn yellow() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().warning;
    (r, g, b)
}
/// Blue: tool execution info — theme accent_secondary (Cyan)
pub(crate) fn blue() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().accent_secondary;
    (r, g, b)
}
/// Purple: accent — theme accent_primary (Pink)
pub(crate) fn purple() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().accent_primary;
    (r, g, b)
}
/// Aqua: secondary accent — theme accent_secondary (Cyan)
pub(crate) fn aqua() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().accent_secondary;
    (r, g, b)
}
/// Orange: highlights — theme accent_primary (Pink) for now
pub(crate) fn orange() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().accent_primary;
    (r, g, b)
}
/// Gray: metadata — theme fg_dim
pub(crate) fn gray() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().fg_dim;
    (r, g, b)
}

// Legacy constants for backward compatibility
pub(crate) const BG_HARD: (u8, u8, u8) = (29, 32, 33);
pub(crate) const FG: (u8, u8, u8) = (235, 219, 178);
pub(crate) const RED: (u8, u8, u8) = (251, 73, 52); // error token
pub(crate) const GREEN: (u8, u8, u8) = (184, 187, 38); // success token
pub(crate) const YELLOW: (u8, u8, u8) = (250, 189, 47); // warning token
pub(crate) const BLUE: (u8, u8, u8) = (131, 165, 152); // accent_secondary token
pub(crate) const PURPLE: (u8, u8, u8) = (211, 134, 155); // accent_primary token
pub(crate) const PINK: (u8, u8, u8) = (255, 120, 180); // primary accent (pink)
pub(crate) const AQUA: (u8, u8, u8) = (142, 192, 124); // accent_secondary alt
pub(crate) const ORANGE: (u8, u8, u8) = (254, 128, 25); // accent_primary alt
pub(crate) const GRAY: (u8, u8, u8) = (146, 131, 116); // fg_dim token

// --- Derived / semantic aliases (theme-mapped) ---

/// Border/separator gray — theme border
pub(crate) fn border_gray() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().border;
    (r, g, b)
}
/// Prompt prefix gray — theme fg_dim
pub(crate) fn prompt_gray() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().fg_dim;
    (r, g, b)
}
/// Very dark gray — theme border (darker)
pub(crate) fn very_dark_gray() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().border;
    (r, g, b)
}
/// Steel blue equivalent — theme accent_secondary
pub(crate) fn steel_blue() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().accent_secondary;
    (r, g, b)
}
/// Pane green equivalent — theme success
pub(crate) fn pane_green() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().success;
    (r, g, b)
}
/// Teal equivalent — theme accent_secondary
pub(crate) fn teal() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().accent_secondary;
    (r, g, b)
}
/// Selection highlight background — theme border
pub(crate) fn select_bg() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().border;
    (r, g, b)
}
/// User message background — theme border
pub(crate) fn user_bg() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().border;
    (r, g, b)
}
/// System message yellow — theme warning
pub(crate) fn system_yellow() -> (u8, u8, u8) {
    let ColorToken(r, g, b) = current_theme().warning;
    (r, g, b)
}

// Legacy constants for backward compatibility
pub(crate) const BORDER_GRAY: (u8, u8, u8) = (102, 92, 84); // border token
pub(crate) const PROMPT_GRAY: (u8, u8, u8) = (146, 131, 116); // fg_dim token
pub(crate) const VERY_DARK_GRAY: (u8, u8, u8) = (102, 92, 84); // same as border
pub(crate) const STEEL_BLUE: (u8, u8, u8) = (131, 165, 152); // accent_secondary token
pub(crate) const PANE_GREEN: (u8, u8, u8) = (184, 187, 38); // success token
pub(crate) const TEAL: (u8, u8, u8) = (142, 192, 124); // accent_secondary alt
pub(crate) const SELECT_BG: (u8, u8, u8) = (102, 92, 84); // border token
pub(crate) const USER_BG: (u8, u8, u8) = (102, 92, 84); // border token
pub(crate) const SYSTEM_YELLOW: (u8, u8, u8) = (250, 189, 47); // warning token

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
    fn test_all_color_functions_use_theme_tokens() {
        // These now use theme tokens from ui_theme.rs (Pink/Cyan default)
        // elma_accent = pink (primary accent)
        assert!(elma_accent("x").contains("\x1b[38;2;255;105;180m"));
        // info_cyan = cyan (secondary accent)
        assert!(info_cyan("x").contains("\x1b[38;2;0;255;255m"));
        // error_red = red
        assert!(error_red("x").contains("\x1b[38;2;255;0;0m"));
        // warn_yellow = yellow
        assert!(warn_yellow("x").contains("\x1b[38;2;255;255;0m"));
        // success_green = green
        assert!(success_green("x").contains("\x1b[38;2;0;255;0m"));
        // text_white = white
        assert!(text_white("x").contains("\x1b[38;2;255;255;255m"));
        // meta_comment = grey
        assert!(meta_comment("x").contains("\x1b[38;2;128;128;128m"));
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
