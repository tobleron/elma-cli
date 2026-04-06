//! @efficiency-role: ui-component
//!
//! Gruvbox Dark Hard Color Palette — exact RGB values + ANSI helpers.
//! Re-exports constants from ui_colors and adds rendering utilities.

// Re-export all colors from ui_colors.
pub(crate) use crate::ui_colors::*;

// ============================================================================
// Unicode Symbols (Gruvbox-styled)
// ============================================================================

pub(crate) const ASSISTANT_DOT: &str = "\u{25CF}"; // ● black circle
pub(crate) const USER_ARROW: &str = "\u{276F}"; // ❯ heavy rightwards arrow
pub(crate) const CHECK: &str = "✓";
pub(crate) const CROSS: &str = "✗";
pub(crate) const BULLET: &str = "•";
pub(crate) const BLOCKQUOTE_BAR: &str = "▎";
pub(crate) const HR: &str = "─";
pub(crate) const MIDDOT: &str = "·";
pub(crate) const LIGHTNING: &str = "⚡";
pub(crate) const LOCK: &str = "🔒";
pub(crate) const CRAB: &str = "🦀";
pub(crate) const HOURGLASS: &str = "⏳";
pub(crate) const CODE_HEADER_LEFT: &str = "├─";
pub(crate) const CODE_HEADER_RIGHT: &str = "─┤";
pub(crate) const CODE_FOOTER: &str = "╰";
pub(crate) const LINE_SEPARATOR: &str = "│";
pub(crate) const EXPAND_ARROW_RIGHT: &str = "\u{25B8}"; // ▸
pub(crate) const EXPAND_ARROW_DOWN: &str = "\u{25BE}"; // ▾
pub(crate) const BLOCK_CURSOR: &str = "\u{2588}"; // █

// Spinner frames (Braille, 10-frame cycle)
pub(crate) const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ============================================================================
// ANSI 24-bit Helpers
// ============================================================================

/// Apply foreground 24-bit RGB color to text.
pub(crate) fn fg(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

/// Apply foreground 24-bit RGB + bold to text.
pub(crate) fn fg_bold(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[1;38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

/// Apply foreground + background 24-bit RGB to text.
pub(crate) fn fg_bg(
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
    text: &str,
) -> String {
    format!(
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, text
    )
}

/// Apply foreground + background 24-bit RGB + bold to text.
pub(crate) fn fg_bg_bold(
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
    text: &str,
) -> String {
    format!(
        "\x1b[1;38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, text
    )
}

/// Apply background 24-bit RGB to text (foreground reset).
pub(crate) fn bg(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[48;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

/// Bold text (ANSI 1).
pub(crate) fn bold(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

/// Dim text (ANSI 2).
pub(crate) fn dim(text: &str) -> String {
    format!("\x1b[2m{}\x1b[0m", text)
}

/// Bold + dim text.
#[allow(dead_code)]
pub(crate) fn bold_dim(text: &str) -> String {
    format!("\x1b[1;2m{}\x1b[0m", text)
}

/// Italic text (ANSI 3).
pub(crate) fn italic(text: &str) -> String {
    format!("\x1b[3m{}\x1b[0m", text)
}

/// Underline text (ANSI 4).
#[allow(dead_code)]
pub(crate) fn underline(text: &str) -> String {
    format!("\x1b[4m{}\x1b[0m", text)
}

/// Strikethrough text (ANSI 9).
#[allow(dead_code)]
pub(crate) fn strikethrough(text: &str) -> String {
    format!("\x1b[9m{}\x1b[0m", text)
}

/// Named color: white → Gruvbox fg (#ebdbb2).
pub(crate) fn white(text: &str) -> String {
    fg(FG.0, FG.1, FG.2, text)
}

/// Named color: black → Gruvbox bg hard (#1d2021).
#[allow(dead_code)]
pub(crate) fn black(text: &str) -> String {
    fg(BG_HARD.0, BG_HARD.1, BG_HARD.2, text)
}

/// Named color: red → Gruvbox red (#fb4934).
pub(crate) fn red(text: &str) -> String {
    fg(RED.0, RED.1, RED.2, text)
}

/// Named color: green → Gruvbox green (#b8bb26).
pub(crate) fn green(text: &str) -> String {
    fg(GREEN.0, GREEN.1, GREEN.2, text)
}

/// Named color: cyan → Gruvbox blue (#83a598).
pub(crate) fn cyan(text: &str) -> String {
    fg(BLUE.0, BLUE.1, BLUE.2, text)
}

/// Named color: yellow → Gruvbox yellow (#fabd2f).
pub(crate) fn yellow(text: &str) -> String {
    fg(YELLOW.0, YELLOW.1, YELLOW.2, text)
}

/// Named color: purple → Gruvbox purple (#d3869b).
pub(crate) fn purple(text: &str) -> String {
    fg(PURPLE.0, PURPLE.1, PURPLE.2, text)
}

/// Named color: orange → Gruvbox orange (#fe8019).
pub(crate) fn orange(text: &str) -> String {
    fg(ORANGE.0, ORANGE.1, ORANGE.2, text)
}

/// Named color: dark gray → Gruvbox gray (#928374).
pub(crate) fn dark_gray(text: &str) -> String {
    fg(GRAY.0, GRAY.1, GRAY.2, text)
}

/// Fill a line to a given display width with background color.
pub(crate) fn fill_to_width(
    current: &str,
    target_width: usize,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
) -> String {
    use crate::ui_wrap::display_width;
    let current_width = display_width(current);
    if current_width < target_width {
        let spaces = " ".repeat(target_width.saturating_sub(current_width));
        format!("{}{}", current, bg(bg_r, bg_g, bg_b, &spaces))
    } else {
        current.to_string()
    }
}

// ============================================================================
// Semantic wrappers (Gruvbox-mapped, backward compatibility)
// ============================================================================

/// Elma accent → Gruvbox purple (#d3869b).
#[allow(dead_code)]
pub(crate) fn elma_accent(text: &str) -> String {
    fg(PURPLE.0, PURPLE.1, PURPLE.2, text)
}

/// Info cyan → Gruvbox blue (#83a598).
#[allow(dead_code)]
pub(crate) fn info_cyan(text: &str) -> String {
    fg(BLUE.0, BLUE.1, BLUE.2, text)
}

/// Error red → Gruvbox red (#fb4934).
#[allow(dead_code)]
pub(crate) fn error_red(text: &str) -> String {
    fg(RED.0, RED.1, RED.2, text)
}

/// Warn yellow → Gruvbox yellow (#fabd2f).
#[allow(dead_code)]
pub(crate) fn warn_yellow(text: &str) -> String {
    fg(YELLOW.0, YELLOW.1, YELLOW.2, text)
}

/// Success green → Gruvbox green (#b8bb26).
#[allow(dead_code)]
pub(crate) fn success_green(text: &str) -> String {
    fg(GREEN.0, GREEN.1, GREEN.2, text)
}

/// Text white → Gruvbox fg (#ebdbb2).
#[allow(dead_code)]
pub(crate) fn text_white(text: &str) -> String {
    fg(FG.0, FG.1, FG.2, text)
}

/// Meta comment → Gruvbox gray (#928374).
#[allow(dead_code)]
pub(crate) fn meta_comment(text: &str) -> String {
    fg(GRAY.0, GRAY.1, GRAY.2, text)
}

/// Gruvbox orange (#fe8019).
#[allow(dead_code)]
pub(crate) fn gruvbox_orange(text: &str) -> String {
    fg(ORANGE.0, ORANGE.1, ORANGE.2, text)
}

/// Gruvbox aqua (#8ec07c).
#[allow(dead_code)]
pub(crate) fn gruvbox_aqua(text: &str) -> String {
    fg(AQUA.0, AQUA.1, AQUA.2, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gruvbox_orange() {
        assert_eq!(ORANGE, (254, 128, 25));
    }

    #[test]
    fn test_gruvbox_border_gray() {
        assert_eq!(BORDER_GRAY, (102, 92, 84));
    }

    #[test]
    fn test_gruvbox_prompt_gray() {
        assert_eq!(PROMPT_GRAY, (146, 131, 116));
    }

    #[test]
    fn test_gruvbox_steel_blue() {
        assert_eq!(STEEL_BLUE, (131, 165, 152));
    }

    #[test]
    fn test_gruvbox_pane_green() {
        assert_eq!(PANE_GREEN, (184, 187, 38));
    }

    #[test]
    fn test_gruvbox_teal() {
        assert_eq!(TEAL, (142, 192, 124));
    }

    #[test]
    fn test_gruvbox_select_bg() {
        assert_eq!(SELECT_BG, (80, 73, 69));
    }

    #[test]
    fn test_gruvbox_user_bg() {
        assert_eq!(USER_BG, (60, 56, 54));
    }

    #[test]
    fn test_gruvbox_system_yellow() {
        assert_eq!(SYSTEM_YELLOW, (250, 189, 47));
    }

    #[test]
    fn test_semantic_functions_use_gruvbox() {
        // elma_accent should use purple
        assert!(elma_accent("x").contains("211;134;155"));
        // warn_yellow should use yellow, not orange
        assert!(warn_yellow("x").contains("250;189;47"));
        // info_cyan should use blue
        assert!(info_cyan("x").contains("131;165;152"));
        // error_red should use Gruvbox red
        assert!(error_red("x").contains("251;73;52"));
        // success_green should use Gruvbox green
        assert!(success_green("x").contains("184;187;38"));
    }
}
