//! @efficiency-role: logic
//!
//! InputController — owns text buffer, cursor, history, picker state, and
//! command-mode parsing.  Emits InputAction events instead of mutating UI
//! state directly.
//!
//! Task 637: Separate prompt editing, slash commands, file mentions,
//! shell/background modes, and keybinding chords into an input controller
//! that emits typed commands.

use crate::claude_ui::{InputMode, PickerState, SessionCommand};
use crate::ui_runtime_event::UiRuntimeEvent;
use crate::ui_wrap::display_width;
use std::path::PathBuf;

/// Actions the InputController can emit.
#[derive(Clone, Debug)]
pub(crate) enum InputAction {
    /// Submit the current buffer as a chat message.
    SubmitChat(String),
    /// Submit the current buffer as a shell command.
    SubmitShell(String),
    /// Submit the current buffer as a background job.
    SubmitBackground(String),
    /// Open a modal dialog by name.
    OpenModal(&'static str),
    /// Switch model.
    SwitchModel,
    /// Toggle reasoning visibility.
    ToggleReasoning,
    /// Cancel the current action / close picker.
    Cancel,
    /// Open search panel.
    OpenSearch,
    /// Close the application.
    Exit,
    /// Confirm exit (double-press).
    ConfirmExit,
}

/// Slash command descriptor.
#[derive(Clone, Debug)]
pub(crate) struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub action: InputAction,
}

/// Built-in slash command registry.
pub(crate) const SLASH_COMMAND_REGISTRY: &[SlashCommand] = &[
    SlashCommand {
        name: "/help",
        description: "Show help",
        action: InputAction::OpenModal("help"),
    },
    SlashCommand {
        name: "/models",
        description: "Switch model",
        action: InputAction::SwitchModel,
    },
    SlashCommand {
        name: "/reasoning",
        description: "Toggle reasoning visibility",
        action: InputAction::ToggleReasoning,
    },
    SlashCommand {
        name: "/clear",
        description: "Clear transcript",
        action: InputAction::Cancel,
    },
    SlashCommand {
        name: "/exit",
        description: "Exit Elma",
        action: InputAction::Exit,
    },
    SlashCommand {
        name: "/search",
        description: "Search transcript",
        action: InputAction::OpenSearch,
    },
];

/// InputController owns the text buffer, cursor, history, picker state,
/// and command-mode parsing.
#[derive(Clone, Debug)]
pub(crate) struct InputController {
    /// Lines of the current input buffer.
    lines: Vec<String>,
    /// Cursor row (0-based).
    cursor_row: usize,
    /// Cursor column — byte offset within the current line.
    cursor_col: usize,
    /// Input mode.
    mode: InputMode,
    /// Picker state (slash commands, file mentions).
    picker: PickerState,
    /// File mention matches.
    file_matches: Vec<String>,
    /// Maximum visible lines.
    max_lines: usize,
}

impl InputController {
    pub(crate) fn new(max_lines: usize) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            mode: InputMode::Chat,
            picker: PickerState::None,
            file_matches: Vec::new(),
            max_lines: max_lines.max(1),
        }
    }

    // ── Text buffer ────────────────────────────────────────────────────

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub(crate) fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub(crate) fn mode(&self) -> &InputMode {
        &self.mode
    }

    pub(crate) fn picker(&self) -> &PickerState {
        &self.picker
    }

    pub(crate) fn file_matches(&self) -> &[String] {
        &self.file_matches
    }

    pub(crate) fn set_content(&mut self, text: &str) {
        self.lines = vec![text.to_string()];
        self.cursor_row = 0;
        self.cursor_col = text.len();
    }

    pub(crate) fn insert_char(&mut self, c: char) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[self.cursor_row];
        let byte_pos = self.cursor_col.min(line.len());
        line.insert(byte_pos, c);
        self.cursor_col += c.len_utf8();
    }

    pub(crate) fn delete_char(&mut self) {
        if self.lines.is_empty() {
            return;
        }
        let line = &mut self.lines[self.cursor_row];
        if self.cursor_col > 0 && self.cursor_col <= line.len() {
            let byte_pos = self.cursor_col;
            let prev = line[..byte_pos]
                .chars()
                .next_back()
                .map(|c| byte_pos - c.len_utf8())
                .unwrap_or(0);
            line.drain(prev..byte_pos);
            self.cursor_col = prev;
        }
    }

    pub(crate) fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            let byte_pos = self.cursor_col.min(line.len());
            let prev = line[..byte_pos]
                .chars()
                .next_back()
                .map(|c| byte_pos - c.len_utf8())
                .unwrap_or(0);
            self.cursor_col = prev;
        }
    }

    pub(crate) fn move_cursor_right(&mut self) {
        let line = &self.lines[self.cursor_row];
        let byte_pos = self.cursor_col;
        if byte_pos < line.len() {
            let next = byte_pos + line[byte_pos..].chars().next().map_or(0, |c| c.len_utf8());
            self.cursor_col = next;
        }
    }

    pub(crate) fn move_cursor_home(&mut self) {
        self.cursor_col = 0;
    }

    pub(crate) fn move_cursor_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    /// Get the full text content as a single string.
    pub(crate) fn text(&self) -> String {
        self.lines.join("\n")
    }

    // ── Mode ───────────────────────────────────────────────────────────

    pub(crate) fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    /// Detect mode from the current input prefix.
    pub(crate) fn detect_mode(&self) -> InputMode {
        let text = self.text();
        if text.starts_with('!') && !text.starts_with("!/") {
            InputMode::Bash
        } else if text.ends_with('&') && text.len() > 1 {
            InputMode::Background
        } else {
            InputMode::Chat
        }
    }

    // ── Picker ─────────────────────────────────────────────────────────

    pub(crate) fn open_slash_picker(&mut self, query: String) {
        let selected = match &self.picker {
            PickerState::Slash { selected, .. } => *selected,
            _ => 0,
        };
        self.picker = PickerState::Slash { query, selected };
    }

    pub(crate) fn open_file_picker(&mut self, query: String, workdir: &PathBuf) {
        self.file_matches = discover_workspace_files(workdir, &query);
        self.picker = PickerState::File { query, selected: 0 };
    }

    pub(crate) fn close_picker(&mut self) {
        self.file_matches.clear();
        self.picker = PickerState::None;
    }

    pub(crate) fn picker_select_down(&mut self) {
        let max = match &self.picker {
            PickerState::Slash { query, .. } => filtered_slash_commands(query).len(),
            PickerState::File { .. } => self.file_matches.len(),
            PickerState::None => 0,
        };
        self.picker.select_next(max);
    }

    pub(crate) fn picker_select_up(&mut self) {
        let max = match &self.picker {
            PickerState::Slash { query, .. } => filtered_slash_commands(query).len(),
            PickerState::File { .. } => self.file_matches.len(),
            PickerState::None => 0,
        };
        self.picker.select_prev(max);
    }

    pub(crate) fn is_picker_active(&self) -> bool {
        self.picker.is_active()
    }

    pub(crate) fn selected_slash_action(&self) -> Option<InputAction> {
        if let PickerState::Slash { query, selected } = &self.picker {
            let filtered = filtered_slash_commands(query);
            filtered.get(*selected).map(|c| c.action.clone())
        } else {
            None
        }
    }

    pub(crate) fn selected_file(&self) -> Option<String> {
        if let PickerState::File { selected, .. } = &self.picker {
            self.file_matches.get(*selected).cloned()
        } else {
            None
        }
    }

    /// Resolve the current input state into an InputAction and UiRuntimeEvent.
    pub(crate) fn resolve_submit(&self) -> (InputAction, UiRuntimeEvent) {
        let text = self.text();

        // Check for slash commands
        if text.starts_with('/') {
            let cmd_name = text.split_whitespace().next().unwrap_or("");
            if let Some(cmd) = SLASH_COMMAND_REGISTRY.iter().find(|c| c.name == cmd_name) {
                return (
                    cmd.action.clone(),
                    UiRuntimeEvent::InputModeChanged("chat".into()),
                );
            }
            // Unrecognized slash → submit as chat
        }

        match self.mode {
            InputMode::Chat => {
                if text.starts_with('!') && !text.starts_with("!/") {
                    let cmd = text.trim_start_matches('!').to_string();
                    (
                        InputAction::SubmitShell(cmd.clone()),
                        UiRuntimeEvent::UserSubmitted(cmd),
                    )
                } else if text.ends_with('&') && text.len() > 1 {
                    let cmd = text.trim_end_matches('&').trim().to_string();
                    (
                        InputAction::SubmitBackground(cmd.clone()),
                        UiRuntimeEvent::UserSubmitted(cmd),
                    )
                } else {
                    (
                        InputAction::SubmitChat(text.clone()),
                        UiRuntimeEvent::UserSubmitted(text),
                    )
                }
            }
            InputMode::Bash => {
                let cmd = text.trim_start_matches('!').to_string();
                (
                    InputAction::SubmitShell(cmd.clone()),
                    UiRuntimeEvent::UserSubmitted(cmd),
                )
            }
            InputMode::Background => {
                let cmd = text.trim_end_matches('&').trim().to_string();
                (
                    InputAction::SubmitBackground(cmd.clone()),
                    UiRuntimeEvent::UserSubmitted(cmd),
                )
            }
            InputMode::Multiline => (
                InputAction::SubmitChat(text.clone()),
                UiRuntimeEvent::UserSubmitted(text),
            ),
        }
    }
}

/// Filter slash commands by query.
fn filtered_slash_commands(query: &str) -> Vec<&SlashCommand> {
    if query.is_empty() {
        return SLASH_COMMAND_REGISTRY.iter().collect();
    }
    let q = query.to_lowercase();
    let mut exact: Vec<&SlashCommand> = Vec::new();
    let mut prefix: Vec<&SlashCommand> = Vec::new();
    for cmd in SLASH_COMMAND_REGISTRY.iter() {
        let name = cmd.name.trim_start_matches('/').to_lowercase();
        if name == q {
            exact.push(cmd);
        } else if name.starts_with(&q) {
            prefix.push(cmd);
        }
    }
    exact.extend(prefix);
    exact
}

/// Discover workspace files matching a prefix.
fn discover_workspace_files(workdir: &PathBuf, query: &str) -> Vec<String> {
    // Simplified file discovery using git ls-files
    let output = std::process::Command::new("git")
        .args(["ls-files", "--others", "--cached", "--exclude-standard"])
        .current_dir(workdir)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let mut matches: Vec<String> = output
        .lines()
        .filter(|f| f.to_lowercase().contains(&query.to_lowercase()))
        .map(|f| f.to_string())
        .collect();
    matches.sort();
    matches.truncate(50);
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_delete() {
        let mut ctrl = InputController::new(10);
        ctrl.insert_char('h');
        ctrl.insert_char('i');
        assert_eq!(ctrl.text(), "hi");
        ctrl.delete_char();
        assert_eq!(ctrl.text(), "h");
    }

    #[test]
    fn test_cursor_movement() {
        let mut ctrl = InputController::new(10);
        ctrl.set_content("hello");
        ctrl.move_cursor_home();
        assert_eq!(ctrl.cursor_col, 0);
        ctrl.move_cursor_right();
        assert_eq!(ctrl.cursor_col, 1);
        ctrl.move_cursor_end();
        assert_eq!(ctrl.cursor_col, 5);
        ctrl.move_cursor_left();
        assert_eq!(ctrl.cursor_col, 4);
    }

    #[test]
    fn test_mode_detection() {
        let mut ctrl = InputController::new(10);
        assert_eq!(ctrl.detect_mode(), InputMode::Chat);
        ctrl.set_content("!ls -la");
        assert_eq!(ctrl.detect_mode(), InputMode::Bash);
        ctrl.set_content("build &");
        assert_eq!(ctrl.detect_mode(), InputMode::Background);
    }

    #[test]
    fn test_slash_command_registry() {
        let cmds = filtered_slash_commands("");
        assert!(cmds.len() >= 6);
        assert!(cmds.iter().any(|c| c.name == "/help"));
    }

    #[test]
    fn test_slash_command_filtering() {
        let cmds = filtered_slash_commands("help");
        assert!(cmds.iter().any(|c| c.name == "/help"));
    }

    #[test]
    fn test_resolve_submit_chat() {
        let mut ctrl = InputController::new(10);
        ctrl.set_content("hello");
        let (action, event) = ctrl.resolve_submit();
        assert!(matches!(action, InputAction::SubmitChat(_)));
        assert!(matches!(event, UiRuntimeEvent::UserSubmitted(_)));
    }

    #[test]
    fn test_resolve_submit_shell() {
        let mut ctrl = InputController::new(10);
        ctrl.set_content("!ls");
        let (action, event) = ctrl.resolve_submit();
        assert!(matches!(action, InputAction::SubmitShell(_)));
        assert!(matches!(event, UiRuntimeEvent::UserSubmitted(_)));
    }

    #[test]
    fn test_resolve_submit_slash() {
        let mut ctrl = InputController::new(10);
        ctrl.set_content("/help");
        let (action, _event) = ctrl.resolve_submit();
        assert!(matches!(action, InputAction::OpenModal(_)));
    }

    #[test]
    fn test_picker_navigation() {
        let mut ctrl = InputController::new(10);
        ctrl.open_slash_picker("".into());
        assert!(ctrl.is_picker_active());
        ctrl.picker_select_down();
        ctrl.picker_select_up();
        ctrl.close_picker();
        assert!(!ctrl.is_picker_active());
    }
}
