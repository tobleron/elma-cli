//! @efficiency-role: ui-component
//!
//! Task 110: Inquire Interaction Integration
//!
//! Interactive selection menus using inquire crate:
//! - Model selection from API
//! - Profile switching
//! - Tool confirmation prompts
//!
//! Design: Gruvbox Dark Hard themed, vim-mode (j/k), graceful TTY fallback.

use inquire::{Confirm, Select, Text};
use std::io::IsTerminal;

/// Theme configuration using Gruvbox Dark Hard colors.
fn theme() -> inquire::ui::RenderConfig<'static> {
    inquire::ui::RenderConfig::default()
        .with_prompt_prefix(inquire::ui::Styled::new("?").with_fg(inquire::ui::Color::LightYellow))
        .with_answered_prompt_prefix(
            inquire::ui::Styled::new("✓").with_fg(inquire::ui::Color::LightGreen),
        )
        .with_highlighted_option_prefix(
            inquire::ui::Styled::new("→").with_fg(inquire::ui::Color::LightCyan),
        )
}

/// Select one item from a list. Returns None on cancel.
/// Falls back to first option if not a TTY.
pub(crate) fn select_from_list<T: std::fmt::Display + Clone + 'static>(
    message: &str,
    options: Vec<T>,
) -> Option<T> {
    if !std::io::stderr().is_terminal() {
        eprintln!(
            "  [select] {}: {} options (non-interactive, selecting first)",
            message,
            options.len()
        );
        return options.first().cloned();
    }

    if options.is_empty() {
        return None;
    }

    let display_options: Vec<String> = options.iter().map(|o| format!("{}", o)).collect();

    Select::new(message, display_options.clone())
        .with_render_config(theme())
        .with_vim_mode(true)
        .prompt()
        .ok()
        .and_then(|selected| {
            let idx = display_options.iter().position(|o| o == &selected)?;
            options.get(idx).cloned()
        })
}

/// Simple text input with a prompt. Returns None on cancel/empty.
pub(crate) fn prompt_text(message: &str) -> Option<String> {
    if !std::io::stderr().is_terminal() {
        return None;
    }

    Text::new(message)
        .with_render_config(theme())
        .prompt()
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// Yes/No confirmation prompt.
pub(crate) fn confirm(message: &str) -> bool {
    if !std::io::stderr().is_terminal() {
        eprintln!(
            "  [confirm] {}: defaulting to 'no' (non-interactive)",
            message
        );
        return false;
    }

    Confirm::new(message)
        .with_render_config(theme())
        .with_default(false)
        .prompt()
        .unwrap_or(false)
}

/// Fuzzy-searchable model selector.
pub(crate) fn select_model(models: Vec<String>) -> Option<String> {
    if models.len() == 1 {
        return models.first().cloned();
    }
    select_from_list("Select model:", models)
}

/// Profile selector.
pub(crate) fn select_profile(profiles: Vec<String>) -> Option<String> {
    if profiles.len() == 1 {
        return profiles.first().cloned();
    }
    select_from_list("Select profile:", profiles)
}

/// Tool execution confirmation for destructive commands.
pub(crate) fn confirm_tool_execution(command: &str) -> bool {
    if !std::io::stderr().is_terminal() {
        return false;
    }

    eprintln!();
    eprintln!("  ⚠️  Destructive command detected:");
    eprintln!("     {}", command);
    eprintln!();

    confirm("Execute this command?")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let t = theme();
        let debug = format!("{:?}", t.prompt_prefix);
        assert!(debug.contains("?"));
        assert!(debug.contains("LightYellow"));
    }

    #[test]
    fn test_select_from_list_empty() {
        let result: Option<String> = select_from_list("test", vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_from_list_single() {
        if std::io::stderr().is_terminal() {
            return;
        }
        let result = select_from_list("test", vec!["only".to_string()]);
        assert_eq!(result, Some("only".to_string()));
    }

    #[test]
    fn test_confirm_non_tty_defaults_false() {
        if std::io::stderr().is_terminal() {
            return;
        }
        let result = confirm("Test confirmation?");
        assert!(!result);
    }

    #[test]
    fn test_prompt_text_non_tty() {
        if std::io::stderr().is_terminal() {
            return;
        }
        let result = prompt_text("Enter value:");
        assert!(result.is_none());
    }

    #[test]
    fn test_select_model_single() {
        let result = select_model(vec!["model1".to_string()]);
        assert_eq!(result, Some("model1".to_string()));
    }

    #[test]
    fn test_select_profile_single() {
        let result = select_profile(vec!["default".to_string()]);
        assert_eq!(result, Some("default".to_string()));
    }
}
