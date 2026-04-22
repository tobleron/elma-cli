//! @efficiency-role: ui-component
//!
//! Modal overlay rendering — centered boxes for confirmations, help, and selections.
//!
//! Simple, robust, keyboard-first. No fake translucency, no shadows.
//! Thin border, theme-token colors.

use crate::ui_colors::*;
use crate::ui_state::ModalState;
use crate::ui_theme::*;
use crate::ui_theme::{current_theme, fg_bold_token, fg_token};
use crate::ui_wrap::{display_width, wrap_ansi};

/// Render a modal overlay into display lines.
///
/// Returns a Vec of lines that should be drawn centered on the screen.
/// `screen_width` and `screen_height` are the terminal dimensions.
pub(crate) fn render_modal(
    modal: &ModalState,
    screen_width: usize,
    screen_height: usize,
) -> Vec<String> {
    let content_lines = match modal {
        ModalState::Confirm { title, message } => render_confirm_box(title, message, screen_width),
        ModalState::Help { content } => render_help_box(content, screen_width),
        ModalState::Select { title, options } => render_select_box(title, options, screen_width),
        ModalState::Settings { content } => render_settings_box(content, screen_width),
        ModalState::Usage { content } => render_usage_box(content, screen_width),
        ModalState::ToolApproval {
            tool_name,
            description,
            selected,
        } => render_tool_approval(tool_name, description, *selected, screen_width),
        ModalState::PermissionGate {
            command,
            risk_level,
            selected,
        } => render_permission_gate(command, risk_level, *selected, screen_width),
        ModalState::PlanProgress {
            title,
            current,
            total,
            steps,
        } => render_plan_progress(title, *current, *total, steps, screen_width),
        ModalState::Notification { message, level } => {
            render_notification(message, level, screen_width)
        }
        ModalState::Splash { content } => render_splash(content, screen_width),
    };

    // Center vertically: calculate padding
    let box_height = content_lines.len() + 2; // +2 for borders
    let top_pad = if screen_height > box_height + 2 {
        (screen_height - box_height) / 2
    } else {
        0
    };

    let mut all_lines: Vec<String> = Vec::new();

    // Top padding
    for _ in 0..top_pad {
        all_lines.push(String::new());
    }

    // Top border
    let first_line = &content_lines[0];
    let box_width = display_width(first_line) + 2; // +2 for side borders
    all_lines.push(center_box_line(box_width, None));

    // Content lines with side borders
    for line in &content_lines {
        all_lines.push(wrap_in_borders(line, box_width));
    }

    // Bottom border
    all_lines.push(center_box_line(box_width, None));

    // Bottom padding to fill screen
    while all_lines.len() < screen_height {
        all_lines.push(String::new());
    }

    all_lines
}

/// Render a confirmation dialog box.
fn render_confirm_box(title: &str, message: &str, _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Title centered
    let title_line = format!(" {} ", fg_bold_token(current_theme().warning, title));
    lines.push(title_line);

    lines.push(String::new()); // spacer

    // Message text — wrap if needed
    let max_msg_width = 60;
    for msg_line in message.lines() {
        let wrapped = wrap_ansi(msg_line, max_msg_width);
        for wline in wrapped {
            lines.push(format!("  {}", wline));
        }
    }

    lines.push(String::new()); // spacer

    // Action hints
    let hint = format!(
        "{} to confirm · {} to cancel",
        fg_bold_token(current_theme().success, "Enter"),
        dim("Esc"),
    );
    lines.push(format!("  {}", hint));

    lines
}

/// Render a help / reference box.
fn render_help_box(content: &str, _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Title
    let title_line = format!(" {} ", fg_bold(AQUA.0, AQUA.1, AQUA.2, "Commands"));
    lines.push(title_line);

    lines.push(String::new()); // spacer

    // Content — typically slash command reference
    let max_width = 60;
    for content_line in content.lines() {
        let wrapped = wrap_ansi(content_line, max_width);
        for wline in wrapped {
            lines.push(format!("  {}", wline));
        }
    }

    lines.push(String::new()); // spacer

    let hint = format!("{} to close", dim("Esc"));
    lines.push(format!("  {}", hint));

    lines
}

/// Render a selection box.
fn render_select_box(title: &str, options: &[String], _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Title
    let title_line = format!(
        " {} ",
        fg_bold_token(current_theme().accent_secondary, title)
    );
    lines.push(title_line);

    lines.push(String::new()); // spacer

    // Options
    for (i, opt) in options.iter().enumerate() {
        let prefix = if i == 0 {
            fg_bold_token(current_theme().accent_secondary, "▸")
        } else {
            dim(" ")
        };
        lines.push(format!("  {} {}", prefix, opt));
    }

    lines.push(String::new()); // spacer

    let hint = format!(
        "{} to navigate · {} to select · {} to cancel",
        dim("↑/↓"),
        dim("Enter"),
        dim("Esc"),
    );
    lines.push(format!("  {}", hint));

    lines
}

/// Render a settings display box.
fn render_settings_box(content: &str, _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let title_line = format!(" {} ", fg_bold(AQUA.0, AQUA.1, AQUA.2, "Settings"));
    lines.push(title_line);
    lines.push(String::new());
    let max_width = 60;
    for content_line in content.lines() {
        let wrapped = wrap_ansi(content_line, max_width);
        for wline in wrapped {
            lines.push(format!("  {}", wline));
        }
    }
    lines.push(String::new());
    lines.push(format!("  {}", dim("Esc to close")));
    lines
}

/// Render a usage/stats dialog.
fn render_usage_box(content: &str, _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let title_line = format!(" {} ", fg_bold(YELLOW.0, YELLOW.1, YELLOW.2, "Usage"));
    lines.push(title_line);
    lines.push(String::new());
    let max_width = 60;
    for content_line in content.lines() {
        let wrapped = wrap_ansi(content_line, max_width);
        for wline in wrapped {
            lines.push(format!("  {}", wline));
        }
    }
    lines.push(String::new());
    lines.push(format!("  {}", dim("Esc to close")));
    lines
}

/// Render a tool approval dialog with Yes/Always/No.
fn render_tool_approval(
    tool_name: &str,
    description: &str,
    selected: usize,
    _screen_width: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let title_line = format!(
        " {} {} ",
        fg_token(current_theme().warning, "⚡"),
        fg_bold_token(current_theme().warning, tool_name),
    );
    lines.push(title_line);
    lines.push(String::new());
    let max_width = 60;
    for desc_line in description.lines().take(5) {
        let wrapped = wrap_ansi(desc_line, max_width);
        for wline in wrapped {
            lines.push(format!("  {}", wline));
        }
    }
    lines.push(String::new());
    let options = ["Yes", "Always", "No"];
    for (i, opt) in options.iter().enumerate() {
        let is_selected = i == selected;
        let (opt_text, token) = match i {
            0 => (opt.to_string(), current_theme().success),
            1 => (opt.to_string(), current_theme().warning),
            2 => (opt.to_string(), current_theme().error),
            _ => (opt.to_string(), current_theme().fg_dim),
        };
        let prefix = if is_selected {
            fg_bold_token(token, "▸")
        } else {
            dim(" ")
        };
        lines.push(format!(
            "  {} {}",
            prefix,
            if is_selected {
                fg_bold_token(token, &opt_text)
            } else {
                dim(&opt_text)
            }
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} select · {} confirm · {} deny",
        dim("←/→"),
        dim("Enter"),
        dim("D"),
    ));
    lines
}

/// Render a permission gate dialog for destructive commands.
fn render_permission_gate(
    command: &str,
    risk_level: &str,
    selected: usize,
    _screen_width: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let title_line = format!(
        " {} {} ",
        fg_token(current_theme().warning, "🚫"),
        fg_bold_token(current_theme().warning, "Permission Required"),
    );
    lines.push(title_line);
    lines.push(String::new());
    let risk_line = format!("Risk: {}", fg_bold_token(current_theme().error, risk_level));
    lines.push(format!("  {}", risk_line));
    lines.push(String::new());
    let cmd_line = format!("Command: {}", fg_bold_token(current_theme().fg, command));
    lines.push(format!("  {}", cmd_line));
    lines.push(String::new());
    let options = ["Yes", "Always", "No"];
    for (i, opt) in options.iter().enumerate() {
        let is_selected = i == selected;
        let (opt_text, token) = match i {
            0 => (opt.to_string(), current_theme().success),
            1 => (opt.to_string(), current_theme().warning),
            2 => (opt.to_string(), current_theme().error),
            _ => (opt.to_string(), current_theme().fg_dim),
        };
        let prefix = if is_selected {
            fg_bold_token(token, "▸")
        } else {
            dim(" ")
        };
        lines.push(format!(
            "  {} {}",
            prefix,
            if is_selected {
                fg_bold_token(token, &opt_text)
            } else {
                dim(&opt_text)
            }
        ));
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} select · {} confirm · {} deny · {} always",
        dim("←/→"),
        dim("Enter/Y"),
        dim("N"),
        dim("A"),
    ));
    lines
}

/// Render plan progress widget.
fn render_plan_progress(
    title: &str,
    current: usize,
    total: usize,
    steps: &[String],
    _screen_width: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let title_line = format!(
        " {} {}  {}/{}",
        fg(BLUE.0, BLUE.1, BLUE.2, "◆"),
        fg_bold(BLUE.0, BLUE.1, BLUE.2, title),
        current,
        total,
    );
    lines.push(title_line);
    let bar_width = 30;
    let filled = if total > 0 {
        (current * bar_width) / total
    } else {
        0
    };
    let bar = format!("  {}{}", "█".repeat(filled), "░".repeat(bar_width - filled));
    let pct = if total > 0 {
        (current * 100) / total
    } else {
        0
    };
    lines.push(format!("{}  {}%", bar, pct));
    lines.push(String::new());
    for (i, step) in steps.iter().enumerate().take(6) {
        let prefix = if i < current {
            fg(GREEN.0, GREEN.1, GREEN.2, "✓")
        } else if i == current {
            fg(ORANGE.0, ORANGE.1, ORANGE.2, "▶")
        } else {
            dim("·")
        };
        lines.push(format!("  {} {}", prefix, dim(step)));
    }
    if steps.len() > 6 {
        lines.push(format!(
            "  {}",
            dim(&format!("... ({} more)", steps.len() - 6))
        ));
    }
    lines
}

/// Render a notification.
fn render_notification(message: &str, level: &str, _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let (icon, color) = match level {
        "error" => ("✗", (RED.0, RED.1, RED.2)),
        "warning" => ("⚠", (YELLOW.0, YELLOW.1, YELLOW.2)),
        _ => ("ℹ", (BLUE.0, BLUE.1, BLUE.2)),
    };
    let title = format!(" {} {}", fg(color.0, color.1, color.2, icon), message);
    lines.push(format!("  {}", title));
    lines
}

/// Render a splash screen.
fn render_splash(content: &str, _screen_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for content_line in content.lines() {
        lines.push(format!("  {}", content_line));
    }
    lines.push(String::new());
    lines.push(format!("  {}", dim("Press any key to continue...")));
    lines
}

/// Create a centered border line for the top or bottom of the modal box.
fn center_box_line(width: usize, _label: Option<&str>) -> String {
    let inner = width.saturating_sub(2);
    format!(
        "{}{}",
        fg(
            BORDER_GRAY.0,
            BORDER_GRAY.1,
            BORDER_GRAY.2,
            &"─".repeat(inner)
        ),
        fg(BORDER_GRAY.0, BORDER_GRAY.1, BORDER_GRAY.2, "")
    )
}

/// Wrap a content line in left and right border characters.
fn wrap_in_borders(content: &str, total_width: usize) -> String {
    let inner_width = total_width.saturating_sub(2);
    let content_dw = display_width(content);
    let right_pad = if inner_width > content_dw {
        " ".repeat(inner_width - content_dw)
    } else {
        String::new()
    };
    format!(
        "{}{}{}{}",
        fg(BORDER_GRAY.0, BORDER_GRAY.1, BORDER_GRAY.2, "│"),
        content,
        right_pad,
        fg(BORDER_GRAY.0, BORDER_GRAY.1, BORDER_GRAY.2, "│")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_confirm_box() {
        let lines = render_confirm_box("Confirm", "Are you sure?", 80);
        assert!(!lines.is_empty());
        // Should have title, spacer, message, spacer, hint
        assert!(lines.len() >= 5);
    }

    #[test]
    fn test_render_help_box() {
        let lines = render_help_box("/exit — quit\n/reset — clear history", 80);
        assert!(lines.len() >= 4);
        assert!(lines.iter().any(|l| l.contains("Commands")));
    }

    #[test]
    fn test_render_select_box() {
        let lines = render_select_box("Select", &["opt1".to_string(), "opt2".to_string()], 80);
        assert!(lines.iter().any(|l| l.contains("opt1")));
        assert!(lines.iter().any(|l| l.contains("opt2")));
    }

    #[test]
    fn test_render_modal_returns_screen_lines() {
        let modal = ModalState::Confirm {
            title: "Delete".to_string(),
            message: "This cannot be undone.".to_string(),
        };
        let lines = render_modal(&modal, 80, 24);
        // Should fill the screen height with padding
        assert!(lines.len() >= 24);
    }

    #[test]
    fn test_wrap_in_borders() {
        let line = wrap_in_borders("hello", 20);
        assert!(line.contains("│"));
        assert!(line.contains("hello"));
    }
}
