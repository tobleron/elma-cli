//! @efficiency-role: ui-component
//!
//! Claude Code-style Input and Slash Picker
//!
//! Implements:
//! - Prompt input at bottom with "> " prefix
//! - Slash command picker (/cmd)
//! - File mention picker (@file)
//! - Command modes (!shell, &background)
//! - Key bindings (Ctrl+C, Ctrl+D, Esc, Ctrl+O, etc.)

use crate::ui_theme::*;

// ============================================================================
// Slash Commands
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub second_action: Option<&'static str>,
}

pub(crate) const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "/compact",
        description: "Summarize conversation",
        second_action: None,
    },
    SlashCommand {
        name: "/exit",
        description: "End the session",
        second_action: None,
    },
    SlashCommand {
        name: "/clear",
        description: "Clear conversation",
        second_action: None,
    },
    SlashCommand {
        name: "/help",
        description: "Show available commands",
        second_action: None,
    },
    SlashCommand {
        name: "/retry",
        description: "Retry last assistant response",
        second_action: None,
    },
    SlashCommand {
        name: "/reject",
        description: "Reject last tool call",
        second_action: None,
    },
    SlashCommand {
        name: "/resume",
        description: "Resume previous session",
        second_action: None,
    },
];

// ============================================================================
// Picker State
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) enum PickerState {
    #[default]
    None,
    Slash {
        query: String,
        selected: usize,
    },
    File {
        query: String,
        selected: usize,
    },
}

impl PickerState {
    pub(crate) fn is_active(&self) -> bool {
        !matches!(self, PickerState::None)
    }

    pub(crate) fn filter_commands(&self, query: &str) -> Vec<&SlashCommand> {
        if query.is_empty() {
            SLASH_COMMANDS.iter().collect()
        } else {
            SLASH_COMMANDS
                .iter()
                .filter(|c| c.name.contains(query) || c.description.contains(query))
                .collect()
        }
    }

    pub(crate) fn select_next(&mut self, max: usize) {
        if let PickerState::Slash {
            ref mut selected, ..
        } = self
        {
            *selected = (*selected + 1).min(max.saturating_sub(1));
        }
    }

    pub(crate) fn select_prev(&mut self, max: usize) {
        if let PickerState::Slash {
            ref mut selected, ..
        } = self
        {
            *selected = selected.saturating_sub(1);
        }
    }
}

// ============================================================================
// Input Mode
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InputMode {
    Chat,
    Multiline,
    Bash,       // ! prefix
    Background, // & suffix
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::Chat
    }
}

// ============================================================================
// Input State
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) struct InputState {
    pub text: String,
    pub cursor: usize,
    pub mode: InputMode,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub picker: PickerState,
    pub multiline: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            mode: InputMode::Chat,
            history: Vec::new(),
            history_index: None,
            picker: PickerState::None,
            multiline: false,
        }
    }
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn start_slash_picker(&mut self) {
        self.picker = PickerState::Slash {
            query: String::new(),
            selected: 0,
        };
    }

    pub(crate) fn start_file_picker(&mut self) {
        self.picker = PickerState::File {
            query: String::new(),
            selected: 0,
        };
    }

    pub(crate) fn close_picker(&mut self) {
        self.picker = PickerState::None;
    }

    pub(crate) fn is_picker_active(&self) -> bool {
        self.picker.is_active()
    }

    pub(crate) fn add_to_history(&mut self) {
        if !self.text.is_empty() && !self.text.starts_with('!') {
            self.history.push(self.text.clone());
            self.history_index = None;
        }
    }

    pub(crate) fn history_up(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }
        let new_index =
            self.history_index
                .map_or(self.history.len() - 1, |i| if i > 0 { i - 1 } else { 0 });
        self.history_index = Some(new_index);
        Some(&self.history[new_index])
    }

    pub(crate) fn history_down(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }
        let new_index = match self.history_index {
            Some(i) if i < self.history.len() - 1 => i + 1,
            Some(_) => return None,
            None => return None,
        };
        self.history_index = Some(new_index);
        Some(&self.history[new_index])
    }
}

// ============================================================================
// Key Binding Helpers
// ============================================================================

pub(crate) fn format_key(key: &str) -> String {
    elma_accent(key)
}

pub(crate) const FOOTER_HINTS: &[(&str, &str)] = &[
    ("ctrl+o", "transcript"),
    ("ctrl+t", "tasks"),
    ("ctrl+c", "interrupt"),
    ("enter", "send"),
    ("esc", "cancel"),
];

pub(crate) fn render_footer() -> String {
    FOOTER_HINTS
        .iter()
        .map(|(key, desc)| format!("{}: {}", meta_comment(key), dim(desc)))
        .collect::<Vec<_>>()
        .join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_commands() {
        let cmds = SLASH_COMMANDS;
        assert!(!cmds.is_empty());
        assert!(cmds.iter().any(|c| c.name == "/compact"));
    }

    #[test]
    fn test_picker_filter() {
        let picker = PickerState::None;
        assert!(!picker.is_active());
    }

    #[test]
    fn test_input_history() {
        let mut input = InputState::new();
        input.text = "hello".to_string();
        input.add_to_history();
        assert_eq!(input.history.len(), 1);
    }

    #[test]
    fn test_input_mode_default() {
        let input = InputState::new();
        assert_eq!(input.mode, InputMode::Chat);
    }
}
