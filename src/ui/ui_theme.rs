//! @efficiency-role: ui-component
//!
//! Tokenized Theme System — Pink/Cyan default theme for Claude Code parity.
//! Replaces hardcoded Gruvbox colors with semantic tokens.

use std::sync::OnceLock;

// ============================================================================
// Theme Token System (Claude Code Parity)
// ============================================================================

/// A semantic color token in the theme system.
/// Tokens map to RGB values that can be swapped per theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ColorToken(pub u8, pub u8, pub u8);

impl ColorToken {
    pub(crate) fn to_ratatui_color(self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.0, self.1, self.2)
    }
}

/// The complete theme with semantic color tokens.
/// Future themes can swap Pink for Orange, etc., without changing renderers.
#[derive(Clone, Debug)]
pub(crate) struct Theme {
    /// Primary text color (white/off-white)
    pub fg: ColorToken,
    /// Dim/secondary text, metadata, separators (grey)
    pub fg_dim: ColorToken,
    /// Primary accent (Pink in default theme)
    pub accent_primary: ColorToken,
    /// Complementary accent (Cyan in default theme)
    pub accent_secondary: ColorToken,
    /// Success states (green)
    pub success: ColorToken,
    /// Error states (red)
    pub error: ColorToken,
    /// Warning states (yellow)
    pub warning: ColorToken,
    /// Border/separator color (grey)
    pub border: ColorToken,
    /// Background color (dark grey, not pure black)
    pub bg: ColorToken,
    /// Footer background (slightly lighter than bg)
    pub bg_footer: ColorToken,
}

/// Claude Code default theme: black/white/grey + Pink/Cyan
pub(crate) fn default_theme() -> Theme {
    Theme {
        fg: ColorToken(255, 255, 255),             // White
        fg_dim: ColorToken(128, 128, 128),         // Grey
        accent_primary: ColorToken(255, 105, 180), // Hot Pink
        accent_secondary: ColorToken(0, 255, 255), // Cyan
        success: ColorToken(0, 255, 0),            // Green
        error: ColorToken(255, 0, 0),              // Red
        warning: ColorToken(255, 255, 0),          // Yellow
        border: ColorToken(64, 64, 64),            // Dark grey
        bg: ColorToken(28, 28, 30),                // Very dark grey background
        bg_footer: ColorToken(36, 36, 40),         // Slightly lighter footer bg
    }
}

/// Global theme instance (lazy-initialized to default)
static THEME: OnceLock<Theme> = OnceLock::new();

/// Get the current theme (defaults to Pink/Cyan)
pub(crate) fn current_theme() -> &'static Theme {
    THEME.get_or_init(default_theme)
}

/// Set a custom theme (for future theme switching)
#[allow(dead_code)]
pub(crate) fn set_theme(theme: Theme) {
    let _ = THEME.set(theme);
}

// ============================================================================
// Unicode Symbols (Theme-agnostic)
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
// ANSI 24-bit Helpers (Theme-aware)
// ============================================================================

/// Apply foreground color from theme token to text.
pub(crate) fn fg_token(token: ColorToken, text: &str) -> String {
    format!(
        "\x1b[38;2;{};{};{}m{}\x1b[0m",
        token.0, token.1, token.2, text
    )
}

/// Apply foreground + bold from theme token to text.
pub(crate) fn fg_bold_token(token: ColorToken, text: &str) -> String {
    format!(
        "\x1b[1;38;2;{};{};{}m{}\x1b[0m",
        token.0, token.1, token.2, text
    )
}

/// Apply foreground + background from theme tokens to text.
pub(crate) fn fg_bg_token(fg_token: ColorToken, bg_token: ColorToken, text: &str) -> String {
    format!(
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg_token.0, fg_token.1, fg_token.2, bg_token.0, bg_token.1, bg_token.2, text
    )
}

/// Apply foreground + background + bold from theme tokens to text.
pub(crate) fn fg_bg_bold_token(fg_token: ColorToken, bg_token: ColorToken, text: &str) -> String {
    format!(
        "\x1b[1;38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg_token.0, fg_token.1, fg_token.2, bg_token.0, bg_token.1, bg_token.2, text
    )
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

// ============================================================================
// Backward-compatible Semantic Wrappers (Theme-mapped)
// ============================================================================

/// Elma accent → accent_primary (Pink)
pub(crate) fn elma_accent(text: &str) -> String {
    fg_token(current_theme().accent_primary, text)
}

pub(crate) fn accent_primary(text: &str) -> String {
    fg_token(current_theme().accent_primary, text)
}

/// Info cyan → accent_secondary (Cyan)
pub(crate) fn info_cyan(text: &str) -> String {
    fg_token(current_theme().accent_secondary, text)
}

/// Error red → error (red)
pub(crate) fn error_red(text: &str) -> String {
    fg_token(current_theme().error, text)
}

/// Warn yellow → warning (yellow)
pub(crate) fn warn_yellow(text: &str) -> String {
    fg_token(current_theme().warning, text)
}

/// Success green → success (green)
pub(crate) fn success_green(text: &str) -> String {
    fg_token(current_theme().success, text)
}

/// Primary text → fg (white)
pub(crate) fn text_white(text: &str) -> String {
    fg_token(current_theme().fg, text)
}

/// Meta comment → fg_dim (grey)
pub(crate) fn meta_comment(text: &str) -> String {
    fg_token(current_theme().fg_dim, text)
}

/// Border color → border (grey)
pub(crate) fn border_color(text: &str) -> String {
    fg_token(current_theme().border, text)
}

// ============================================================================
// Legacy Gruvbox Functions (Deprecated, kept for compatibility)
// ============================================================================

/// DEPRECATED: Use fg_token instead. Legacy: Apply foreground 24-bit RGB color.
#[deprecated(note = "Use fg_token with theme tokens instead")]
pub(crate) fn fg(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

/// DEPRECATED: Use fg_bold_token instead. Legacy: Apply foreground + bold 24-bit RGB.
#[deprecated(note = "Use fg_bold_token with theme tokens instead")]
pub(crate) fn fg_bold(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[1;38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

/// DEPRECATED: Use fg_bg_token instead. Legacy: Apply fg + bg 24-bit RGB.
#[deprecated(note = "Use fg_bg_token with theme tokens instead")]
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

/// DEPRECATED: Use fg_bg_bold_token instead. Legacy: Apply fg + bg + bold 24-bit RGB.
#[deprecated(note = "Use fg_bg_bold_token with theme tokens instead")]
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

/// DEPRECATED: Use theme tokens instead. Legacy: Background 24-bit RGB.
#[deprecated(note = "Use theme tokens instead")]
pub(crate) fn bg(r: u8, g: u8, b: u8, text: &str) -> String {
    format!("\x1b[48;2;{};{};{}m{}\x1b[0m", r, g, b, text)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_tokens() {
        let theme = default_theme();
        assert_eq!(theme.fg, ColorToken(255, 255, 255)); // White
        assert_eq!(theme.fg_dim, ColorToken(128, 128, 128)); // Grey
        assert_eq!(theme.accent_primary, ColorToken(255, 105, 180)); // Hot Pink
        assert_eq!(theme.accent_secondary, ColorToken(0, 255, 255)); // Cyan
        assert_eq!(theme.success, ColorToken(0, 255, 0)); // Green
        assert_eq!(theme.error, ColorToken(255, 0, 0)); // Red
        assert_eq!(theme.warning, ColorToken(255, 255, 0)); // Yellow
        assert_eq!(theme.border, ColorToken(64, 64, 64)); // Dark grey
        assert_eq!(theme.bg, ColorToken(28, 28, 30));
        assert_eq!(theme.bg_footer, ColorToken(36, 36, 40));
    }

    #[test]
    fn test_theme_functions_use_tokens() {
        // elma_accent should use accent_primary (Pink)
        let result = elma_accent("test");
        assert!(result.contains("\x1b[38;2;255;105;180m")); // Hot Pink
        assert!(result.contains("test"));
        assert!(result.ends_with("\x1b[0m"));

        // info_cyan should use accent_secondary (Cyan)
        let result = info_cyan("test");
        assert!(result.contains("\x1b[38;2;0;255;255m")); // Cyan
        assert!(result.contains("test"));
        assert!(result.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_current_theme_returns_default() {
        let theme = current_theme();
        assert_eq!(theme.accent_primary, ColorToken(255, 105, 180));
    }
}
