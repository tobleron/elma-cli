//! @efficiency-role: ui-component
//!
//! Terminal I/O Layer — crossterm raw mode, alternate screen, cursor, resize.
//!
//! This is the public interface that the chat loop talks to.
//! It wraps the UIState model and handles all terminal I/O.

use crate::ui_input::TextInput;
use crate::ui_render::{self, ScreenBuffer};
use crate::ui_state::*;
use crate::ui_theme::*;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::path::PathBuf;

// ============================================================================
// Backward-compatible MessageRole (maps to TranscriptItem internally)
// ============================================================================

#[derive(Clone)]
pub(crate) enum MessageRole {
    User,
    Assistant,
    Tool {
        name: String,
        command: String,
    },
    ToolResult {
        name: String,
        success: bool,
        output: String,
    },
    Thinking,
    System,
}

// ============================================================================
// TerminalUI
// ============================================================================

pub(crate) struct TerminalUI {
    state: UIState,
    input: TextInput,
    raw_mode: bool,
    pending_draw: bool,
}

impl TerminalUI {
    /// Initialize the terminal UI: enter raw mode, alternate screen, hide cursor.
    pub(crate) fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self {
            state: UIState::new(),
            input: TextInput::new(10), // max 10 lines
            raw_mode: true,
            pending_draw: true,
        })
    }

    /// Load input history from a file. Call after construction.
    pub(crate) fn load_history(&mut self, path: &PathBuf) {
        self.input.load_history(path);
    }

    /// Save input history to a file. Call before cleanup.
    pub(crate) fn save_history(&self, path: &PathBuf) {
        self.input.save_history(path);
    }

    /// Apply an autocomplete suggestion by replacing the input content.
    fn apply_autocomplete(&mut self, label: &str) {
        if self.state.autocomplete.is_emoji {
            // For emoji, insert the emoji character at the current cursor position.
            // The label is like ":smile:" — we need to find the actual emoji.
            if let Some(suggestion) = self
                .state
                .autocomplete
                .matches
                .iter()
                .find(|s| s.label == label)
            {
                // Insert the emoji description (the actual emoji char).
                for c in suggestion.description.chars() {
                    self.input.insert_char(c);
                }
            }
        } else {
            // For slash commands, replace the entire input.
            self.input.set_content(label);
            self.input.move_end();
        }
    }

    // --- Backward-compatible add_message ---

    /// Add a message to the transcript (backward-compatible API).
    pub(crate) fn add_message(&mut self, role: MessageRole, content: String) {
        match role {
            MessageRole::User => {
                self.state.push_user_message(&content);
            }
            MessageRole::Assistant => {
                self.state.push_assistant_markdown(&content);
            }
            MessageRole::Tool { name, command } => {
                self.state.push_tool_start(&name, &command);
            }
            MessageRole::ToolResult {
                name,
                success,
                output,
            } => {
                self.state.push_tool_finish(&name, success, &output, None);
            }
            MessageRole::Thinking => {
                self.state.push_thinking(&content);
            }
            MessageRole::System => {
                self.state.push_system(&content);
            }
        }
        self.pending_draw = true;
    }

    // --- Status update (backward-compatible API) ---

    /// Update footer metrics (backward-compatible API).
    pub(crate) fn update_status(
        &mut self,
        model: String,
        ctx_current: u64,
        ctx_max: u64,
        tokens_in: u64,
        tokens_out: u64,
        effort: String,
    ) {
        self.state.footer.model = model;
        self.state.footer.context_current = ctx_current;
        self.state.footer.context_max = ctx_max;
        self.state.footer.tokens_in = tokens_in;
        self.state.footer.tokens_out = tokens_out;
        self.state.footer.effort = effort;
        self.pending_draw = true;
    }

    // --- New push_* methods ---

    pub(crate) fn push_meta_event(&mut self, category: &str, message: &str) {
        self.state.push_meta_event(category, message);
        self.pending_draw = true;
    }

    pub(crate) fn push_tool_start(&mut self, name: &str, command: &str) {
        self.state.push_tool_start(name, command);
        self.pending_draw = true;
    }

    pub(crate) fn push_tool_finish(
        &mut self,
        name: &str,
        success: bool,
        output: &str,
        duration_ms: Option<u64>,
    ) {
        self.state
            .push_tool_finish(name, success, output, duration_ms);
        self.pending_draw = true;
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        self.state.push_warning(message);
        self.pending_draw = true;
    }

    // --- Activity rail ---

    pub(crate) fn set_activity(&mut self, label: &str, message: &str) {
        self.state.set_activity(label, message);
        self.pending_draw = true;
    }

    pub(crate) fn clear_activity(&mut self) {
        self.state.clear_activity();
        self.pending_draw = true;
    }

    // --- Header info ---

    pub(crate) fn set_header_info(&mut self, header: HeaderInfo) {
        self.state.header = header;
        self.pending_draw = true;
    }

    // --- Footer metrics ---

    pub(crate) fn set_footer_metrics(&mut self, metrics: FooterMetrics) {
        self.state.set_footer_metrics(metrics);
        self.pending_draw = true;
    }

    // --- Modal ---

    pub(crate) fn set_modal(&mut self, modal: ModalState) {
        self.state.set_modal(modal);
        self.pending_draw = true;
    }

    pub(crate) fn clear_modal(&mut self) {
        self.state.clear_modal();
        self.pending_draw = true;
    }

    // --- Clear / Reset ---

    pub(crate) fn clear_messages(&mut self) {
        self.state.reset();
        self.pending_draw = true;
    }

    /// Estimate tokens from text (rough: ~4 chars per token).
    #[allow(dead_code)]
    pub(crate) fn estimate_tokens(text: &str) -> u64 {
        text.len() as u64 / 4
    }

    // --- Drawing ---

    fn draw(&mut self) -> io::Result<()> {
        if !self.pending_draw {
            return Ok(());
        }
        self.pending_draw = false;

        let (cols, rows) = size()?;
        let cols = cols as usize;
        let rows = rows as usize;

        if cols == 0 || rows == 0 {
            return Ok(());
        }

        let screen = ui_render::render_screen(&self.state, cols, rows, &self.input);

        // Check if modal is active — if so, overlay.
        if let Some(ref modal) = self.state.modal {
            self.draw_with_modal(&screen, modal, cols, rows)?;
        } else {
            self.draw_normal(&screen)?;
        }

        Ok(())
    }

    fn draw_normal(&self, screen: &ScreenBuffer) -> io::Result<()> {
        let mut out = io::stdout();
        execute!(out, Clear(ClearType::All), MoveTo(0, 0))?;

        let (cols, rows) = size()?;
        let rows = rows as usize;

        // Write lines using \r\n (raw mode needs explicit \r).
        // The last line uses write! (no trailing newline) to prevent terminal scroll.
        let total = screen.lines.len().min(rows);
        for (i, line) in screen.lines.iter().enumerate() {
            if i >= rows {
                break;
            }
            if i == total - 1 {
                // Last line — no trailing newline to avoid scroll
                write!(out, "{}", line)?;
            } else {
                write!(out, "{}\r\n", line)?;
            }
        }

        // Position cursor in the input area.
        let cursor_row = screen.cursor_row.min((rows - 1) as u16);
        let cursor_col = screen.cursor_col.min((cols - 1) as u16);
        execute!(out, MoveTo(cursor_col, cursor_row), Show)?;
        out.flush()?;

        Ok(())
    }

    fn draw_with_modal(
        &self,
        _screen: &ScreenBuffer,
        modal: &ModalState,
        cols: usize,
        rows: usize,
    ) -> io::Result<()> {
        // Draw the normal screen first.
        let mut out = io::stdout();
        execute!(out, Clear(ClearType::All), MoveTo(0, 0))?;

        // Render modal overlay.
        let modal_lines = crate::ui_modal::render_modal(modal, cols, rows);
        for line in &modal_lines {
            writeln!(out, "{}", line)?;
        }

        // Position cursor at bottom-center of modal for visual balance.
        let cursor_row = (rows / 2) as u16;
        let cursor_col = (cols / 2) as u16;
        execute!(out, MoveTo(cursor_col, cursor_row), Show)?;
        out.flush()?;

        Ok(())
    }

    // --- Input Loop ---

    /// Run the interactive input loop.
    /// Returns Some(input) when Enter is pressed, None when Esc is pressed.
    pub(crate) fn run_input_loop(&mut self) -> io::Result<Option<String>> {
        loop {
            self.draw()?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(KeyEvent {
                    code,
                    kind,
                    modifiers,
                    ..
                }) = event::read()?
                {
                    if kind != KeyEventKind::Press {
                        continue;
                    }

                    // If modal is active, handle modal-specific keys.
                    if self.state.modal.is_some() {
                        match code {
                            KeyCode::Esc => {
                                self.state.clear_modal();
                                self.pending_draw = true;
                                continue;
                            }
                            KeyCode::Enter => {
                                // For confirm modals, treat Enter as confirmation.
                                if let Some(ModalState::Confirm { .. }) = self.state.modal {
                                    self.state.clear_modal();
                                    self.pending_draw = true;
                                    let input = self.input.content_trimmed();
                                    self.input.clear();
                                    if input.is_empty() {
                                        continue;
                                    }
                                    return Ok(Some(input));
                                }
                                // For tool approval, accept selected option.
                                if let Some(ModalState::ToolApproval { selected, .. }) =
                                    &self.state.modal
                                {
                                    match selected {
                                        0 => { /* Yes — proceed once */ }
                                        1 => { /* Always — auto-approve */ }
                                        _ => { /* No — deny */ }
                                    }
                                    self.state.clear_modal();
                                    self.pending_draw = true;
                                }
                                continue;
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                // D denies tool approval.
                                if let Some(ModalState::ToolApproval { .. }) = &self.state.modal {
                                    self.state.clear_modal();
                                    self.pending_draw = true;
                                }
                            }
                            KeyCode::Left => {
                                if let Some(ModalState::ToolApproval { selected, .. }) =
                                    &mut self.state.modal
                                {
                                    if *selected > 0 {
                                        *selected -= 1;
                                        self.pending_draw = true;
                                    }
                                }
                            }
                            KeyCode::Right => {
                                if let Some(ModalState::ToolApproval { selected, .. }) =
                                    &mut self.state.modal
                                {
                                    *selected = (*selected + 1).min(2);
                                    self.pending_draw = true;
                                }
                            }
                            KeyCode::Char(c) => {
                                self.input.insert_char(c);
                                self.pending_draw = true;
                            }
                            KeyCode::Backspace => {
                                self.input.backspace();
                                self.pending_draw = true;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Normal input handling.
                    match code {
                        KeyCode::Enter if modifiers.contains(KeyModifiers::CONTROL) => {
                            // Ctrl+Enter: submit (or accept autocomplete first)
                            if self.state.autocomplete.active {
                                if let Some(label) = self.state.autocomplete.selected_label() {
                                    self.apply_autocomplete(&label);
                                    self.state.autocomplete.deactivate();
                                    self.pending_draw = true;
                                    continue;
                                }
                            }
                            let input = self.input.content_trimmed();
                            if input.is_empty() {
                                continue;
                            }
                            self.input.push_to_history();
                            self.input.clear();
                            return Ok(Some(input));
                        }
                        KeyCode::Enter => {
                            // Accept autocomplete or submit.
                            if self.state.autocomplete.active {
                                if let Some(label) = self.state.autocomplete.selected_label() {
                                    self.apply_autocomplete(&label);
                                    self.state.autocomplete.deactivate();
                                    self.pending_draw = true;
                                    continue;
                                }
                            }
                            let input = self.input.content_trimmed();
                            if input.is_empty() {
                                continue;
                            }
                            self.input.push_to_history();
                            self.input.clear();
                            return Ok(Some(input));
                        }
                        KeyCode::Tab => {
                            // Cycle autocomplete suggestions.
                            if self.state.autocomplete.active
                                && !self.state.autocomplete.matches.is_empty()
                            {
                                self.state.autocomplete.select_down();
                                self.pending_draw = true;
                            }
                        }
                        KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                            // Ctrl+J: newline (multi-line input)
                            self.input.insert_newline();
                            self.pending_draw = true;
                        }
                        KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => {
                            match c {
                                'c' => {
                                    // Ctrl+C: clear input (first press), quit (second handled by signal)
                                    if !self.input.is_empty() {
                                        self.input.clear();
                                        self.pending_draw = true;
                                    }
                                }
                                'u' => {
                                    self.input.delete_to_line_start();
                                    self.pending_draw = true;
                                }
                                'w' => {
                                    self.input.delete_word_before();
                                    self.pending_draw = true;
                                }
                                'l' => {
                                    // Ctrl+L: open sessions modal
                                    self.state.set_modal(ModalState::Select {
                                        title: "Sessions".to_string(),
                                        options: vec![
                                            "N — New session".to_string(),
                                            "Esc — Back to chat".to_string(),
                                        ],
                                    });
                                    self.pending_draw = true;
                                }
                                'a' => {
                                    // Ctrl+A: home
                                    self.input.move_home();
                                    self.pending_draw = true;
                                }
                                'e' => {
                                    // Ctrl+E: end
                                    self.input.move_end();
                                    self.pending_draw = true;
                                }
                                'b' => {
                                    // Ctrl+B: left
                                    self.input.move_left();
                                    self.pending_draw = true;
                                }
                                'f' => {
                                    // Ctrl+F: right
                                    self.input.move_right();
                                    self.pending_draw = true;
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Char(c) if modifiers.contains(KeyModifiers::ALT) => {
                            match c {
                                // Alt+Left/Right handled via KeyCode::Left/Right with ALT below
                                _ => {
                                    self.input.insert_char(c);
                                    self.pending_draw = true;
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            self.input.insert_char(c);
                            // Update autocomplete state.
                            let content = self.input.content();
                            if content.starts_with('/') {
                                self.state.autocomplete.update_slash(&content);
                            } else if content.starts_with(':') {
                                self.state.autocomplete.update_emoji(&content);
                            } else {
                                self.state.autocomplete.deactivate();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Backspace => {
                            if modifiers.contains(KeyModifiers::ALT)
                                || modifiers.contains(KeyModifiers::CONTROL)
                            {
                                self.input.delete_word_before();
                            } else {
                                self.input.backspace();
                            }
                            // Update autocomplete after backspace.
                            let content = self.input.content();
                            if content.starts_with('/') {
                                self.state.autocomplete.update_slash(&content);
                            } else if content.starts_with(':') {
                                self.state.autocomplete.update_emoji(&content);
                            } else {
                                self.state.autocomplete.deactivate();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Delete => {
                            self.input.delete();
                            let content = self.input.content();
                            if content.starts_with('/') {
                                self.state.autocomplete.update_slash(&content);
                            } else if content.starts_with(':') {
                                self.state.autocomplete.update_emoji(&content);
                            } else {
                                self.state.autocomplete.deactivate();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Left => {
                            if modifiers.contains(KeyModifiers::CONTROL)
                                || modifiers.contains(KeyModifiers::ALT)
                            {
                                self.input.move_word_left();
                            } else {
                                self.input.move_left();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Right => {
                            if modifiers.contains(KeyModifiers::CONTROL)
                                || modifiers.contains(KeyModifiers::ALT)
                            {
                                self.input.move_word_right();
                            } else {
                                self.input.move_right();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Home => {
                            self.input.move_home();
                            self.pending_draw = true;
                        }
                        KeyCode::End => {
                            self.state.scroll_to_bottom();
                            self.input.move_end();
                            self.pending_draw = true;
                        }
                        KeyCode::PageUp => {
                            self.state.scroll_up(5);
                            self.pending_draw = true;
                        }
                        KeyCode::PageDown => {
                            self.state.scroll_down(5);
                            self.pending_draw = true;
                        }
                        KeyCode::Up => {
                            if self.state.autocomplete.active {
                                self.state.autocomplete.select_up();
                            } else if self.input.is_in_history() {
                                self.input.history_up();
                            } else if self.state.viewport.scroll_offset > 0 {
                                self.state.scroll_up(1);
                            } else {
                                self.input.history_up();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Down => {
                            if self.state.autocomplete.active {
                                self.state.autocomplete.select_down();
                            } else if self.input.is_in_history() {
                                self.input.history_down();
                            } else {
                                self.state.scroll_down(1);
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Esc => {
                            if self.state.autocomplete.active {
                                self.state.autocomplete.deactivate();
                                self.pending_draw = true;
                            } else {
                                return Ok(None);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // --- Cleanup ---

    /// Restore terminal state: leave alternate screen, disable raw mode, show cursor.
    pub(crate) fn cleanup(&mut self) -> io::Result<()> {
        if self.raw_mode {
            // Save input history before exiting.
            let history_path = std::env::current_dir()
                .ok()
                .map(|d| d.join("sessions").join("history.txt"));
            if let Some(ref path) = history_path {
                self.save_history(path);
            }
            execute!(io::stdout(), Show, LeaveAlternateScreen)?;
            terminal::disable_raw_mode()?;
            io::stdout().flush()?;
            self.raw_mode = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(TerminalUI::estimate_tokens("hello"), 1);
        assert_eq!(TerminalUI::estimate_tokens("hello world"), 2);
    }

    #[test]
    fn test_message_role_mapping() {
        // Just verify the enum variants exist and compile.
        let _ = MessageRole::User;
        let _ = MessageRole::Assistant;
        let _ = MessageRole::Tool {
            name: "x".to_string(),
            command: "y".to_string(),
        };
        let _ = MessageRole::ToolResult {
            name: "x".to_string(),
            success: true,
            output: "ok".to_string(),
        };
        let _ = MessageRole::Thinking;
        let _ = MessageRole::System;
    }
}
