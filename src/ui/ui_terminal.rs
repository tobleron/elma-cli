//! @efficiency-role: ui-component
//!
//! Terminal I/O Layer — crossterm raw mode, alternate screen, cursor, resize.
//!
//! This is the public interface that the chat loop talks to.
//! It wraps the UIState model and handles all terminal I/O.

use crate::ui_input::TextInput;
use crate::ui_state::*;
use crate::ui_theme::*;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{self, size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::collections::VecDeque;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ============================================================================
// Backward-compatible MessageRole (maps to TranscriptItem internally)
// ============================================================================

#[derive(Clone)]
pub(crate) enum MessageRole {
    User,
    Assistant,
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
    // Claude parity renderer
    claude: crate::claude_ui::ClaudeRenderer,
    previous_claude_screen: Option<Vec<String>>,
    // Ratatui terminal backend
    terminal: Option<ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>>,
    // Claude-style task list state
    tasks: crate::claude_ui::claude_tasks::TaskList,
    // Task U003: Coordinator status indicator
    coordinator_status: crate::ui_coordinator_status::CoordinatorStatus,
    // Double-key chord state
    esc_armed_until: Option<Instant>,
    ctrl_c_armed_until: Option<Instant>,
    ctrl_d_armed_until: Option<Instant>,
    notification_line: Option<String>,
    notification_expiry: Option<Instant>,
    queued_submissions: VecDeque<String>,
    // Async permission request channel
    permission_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    // Async event channel: background thread reads crossterm events,
    // all TerminalUI methods drain from here. This is the ONLY event source.
    event_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
}

#[cfg(unix)]
fn is_stdin_tty() -> bool {
    atty::is(atty::Stream::Stdin)
}

#[cfg(not(unix))]
fn is_stdin_tty() -> bool {
    false
}

fn is_stdout_tty() -> bool {
    io::stdout().is_terminal()
}

impl TerminalUI {
    /// Initialize the terminal UI: enter raw mode, alternate screen, hide cursor.
    /// Falls back to non-interactive mode if stdin is not a terminal.
    pub(crate) fn new() -> io::Result<Self> {
        let is_interactive = is_stdin_tty() && is_stdout_tty();
        let (cols, rows) = if is_interactive {
            let _ = terminal::enable_raw_mode();
            let _ = execute!(io::stdout(), EnterAlternateScreen, Hide, EnableMouseCapture);
            size().unwrap_or((80, 24))
        } else {
            (80, 24)
        };

        let terminal = if is_interactive {
            let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
            Some(ratatui::Terminal::new(backend)?)
        } else {
            None
        };

        // Spawn dedicated input reader thread — the ONLY place that reads crossterm events.
        // All other TerminalUI methods drain events from the async channel.
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        if is_interactive {
            std::thread::spawn(move || {
                loop {
                    match crossterm::event::poll(Duration::from_millis(50)) {
                        Ok(true) => match crossterm::event::read() {
                            Ok(ev) => {
                                if event_tx.send(ev).is_err() {
                                    break; // Receiver dropped, exit thread
                                }
                            }
                            Err(_) => break,
                        },
                        Ok(false) => continue,
                        Err(_) => break,
                    }
                }
            });
        }

        Ok(Self {
            state: UIState::new(),
            input: TextInput::new(10),
            raw_mode: is_interactive,
            pending_draw: true,
            claude: crate::claude_ui::ClaudeRenderer::new(cols as usize, rows as usize),
            previous_claude_screen: None,
            terminal,
            tasks: crate::claude_ui::TaskList::new(),
            coordinator_status: crate::ui_coordinator_status::CoordinatorStatus {
                task_description: "Initializing...".to_string(),
                is_active: false,
            },
            esc_armed_until: None,
            ctrl_c_armed_until: None,
            ctrl_d_armed_until: None,
            notification_line: None,
            notification_expiry: None,
            queued_submissions: VecDeque::new(),
            permission_tx: None,
            event_rx,
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

    /// Add a message to the transcript (Claude renderer only — Task 190).
    pub(crate) fn add_message(&mut self, role: MessageRole, content: String) {
        use crate::claude_ui::ClaudeMessage;

        match &role {
            MessageRole::User => {
                self.claude.push_message(ClaudeMessage::User {
                    content: content.clone(),
                });
            }
            MessageRole::Assistant => {
                // Avoid double-pushing if streaming already added it
                if self.claude.last_assistant_message() != Some(&content) {
                    self.claude.push_message(ClaudeMessage::Assistant {
                        content: content.clone(),
                    });
                }
            }
            MessageRole::Thinking => {
                self.claude.push_message(ClaudeMessage::Thinking {
                    content: content.clone(),
                });
            }
            MessageRole::System => {
                self.claude.push_message(ClaudeMessage::System {
                    content: content.clone(),
                });
            }
        }
        self.pending_draw = true;
    }

    /// Add a Claude-native message directly.
    pub(crate) fn add_claude_message(&mut self, msg: crate::claude_ui::ClaudeMessage) {
        self.claude.push_message(msg);
        self.pending_draw = true;
    }

    // --- Streaming API (Claude Parity) ---

    pub(crate) fn start_thinking(&mut self) {
        self.claude.start_thinking();
        self.pending_draw = true;
    }

    pub(crate) fn append_thinking(&mut self, text: &str) {
        self.claude.append_thinking(text);
        self.pending_draw = true;
    }

    pub(crate) fn finish_thinking(&mut self) {
        self.claude.finish_thinking();
        self.pending_draw = true;
    }

    pub(crate) fn start_content(&mut self) {
        self.claude.start_content();
        self.pending_draw = true;
    }

    pub(crate) fn append_content(&mut self, text: &str) {
        self.claude.append_content(text);
        self.pending_draw = true;
    }

    pub(crate) fn finish_content(&mut self) {
        self.claude.finish_content();
        self.pending_draw = true;
    }

    /// Primary event handler for Claude-style UI (Task 169)
    pub(crate) fn handle_ui_event(&mut self, event: crate::claude_ui::UiEvent) {
        self.claude.handle_event(event);
        self.pending_draw = true;
    }

    /// Async permission request. Sets up the permission prompt and awaits user response.
    /// Pumps the UI while waiting so the user can see the prompt and respond.
    pub(crate) async fn request_permission(&mut self, command: &str) -> bool {
        use crate::claude_ui::UiEvent;

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.permission_tx = Some(tx);

        self.handle_ui_event(UiEvent::PermissionRequested {
            command: command.to_string(),
        });

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut rx = rx;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break false;
            }

            tokio::select! {
                result = &mut rx => {
                    self.permission_tx = None;
                    let granted = result.unwrap_or(false);
                    self.handle_ui_event(UiEvent::ToolFinished {
                        name: "Permission".to_string(),
                        success: granted,
                        output: if granted { "approved".to_string() } else { "denied".to_string() },
                    });
                    return granted;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                    // Pump UI while waiting for permission response
                    if self.raw_mode {
                        let _ = self.draw();
                        if let Ok(Some(queued)) = self.poll_busy_submission() {
                            self.enqueue_submission(queued);
                        }
                    }
                }
            }
        }
    }

    /// Send permission response (called from input loop when y/n is pressed).
    fn resolve_permission(&mut self, granted: bool) {
        if let Some(tx) = self.permission_tx.take() {
            let _ = tx.send(granted);
        }
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

    pub(crate) fn push_meta_event(&mut self, _category: &str, _message: &str) {
        // Task 190: Meta events are now handled through Claude renderer event system
        self.pending_draw = true;
    }

    pub(crate) fn push_tool_start(&mut self, name: &str, command: &str) {
        self.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
            name: name.to_string(),
            command: command.to_string(),
        });
    }

    pub(crate) fn push_tool_finish(
        &mut self,
        name: &str,
        success: bool,
        output: &str,
        _duration_ms: Option<u64>,
    ) {
        self.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
            name: name.to_string(),
            success,
            output: output.to_string(),
        });
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        self.claude
            .push_message(crate::claude_ui::ClaudeMessage::System {
                content: message.to_string(),
            });
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
        // Also clear Claude renderer transcript
        self.claude.clear_transcript();
        self.pending_draw = true;
    }

    pub(crate) fn todo_add(&mut self, description: String) -> u32 {
        let id = self.tasks.push(description);
        self.tasks.show();
        self.pending_draw = true;
        id
    }

    pub(crate) fn todo_update(&mut self, id: u32, text: String) {
        self.tasks.update_text(id, text);
        self.pending_draw = true;
    }

    pub(crate) fn todo_start(&mut self, id: u32) {
        self.tasks.start(id);
        self.tasks.show();
        self.pending_draw = true;
    }

    pub(crate) fn todo_complete(&mut self, id: u32) {
        self.tasks.complete(id);
        self.pending_draw = true;
    }

    pub(crate) fn todo_block(&mut self, id: u32, reason: Option<String>) {
        self.tasks.block(id, reason);
        self.tasks.show();
        self.pending_draw = true;
    }

    pub(crate) fn todo_remove(&mut self, id: u32) -> bool {
        let removed = self.tasks.remove(id);
        self.pending_draw = true;
        removed
    }

    pub(crate) fn set_coordinator_status(&mut self, description: String, active: bool) {
        self.coordinator_status.task_description = description;
        self.coordinator_status.is_active = active;
        self.pending_draw = true;
    }

    pub(crate) fn todo_render_lines(&self) -> Vec<String> {
        self.tasks.render()
    }

    pub(crate) fn notify(&mut self, message: &str) {
        self.notification_line = Some(message.to_string());
        self.notification_expiry = Some(Instant::now() + Duration::from_secs(5));
        self.pending_draw = true;
    }

    pub(crate) fn enqueue_submission(&mut self, input: String) {
        self.queued_submissions.push_back(input);
        self.notification_line = Some("Queued 1 message (will run after current response)".into());
        self.notification_expiry = Some(Instant::now() + Duration::from_secs(5));
        self.pending_draw = true;
    }

    pub(crate) fn take_queued_submissions(&mut self) -> Vec<String> {
        self.queued_submissions.drain(..).collect()
    }

    /// Estimate tokens from text (rough: ~4 chars per token).
    #[allow(dead_code)]
    pub(crate) fn estimate_tokens(text: &str) -> u64 {
        text.len() as u64 / 4
    }

    // --- Drawing ---

    /// Non-blocking UI pump — call between async steps to keep UI alive.
    /// Forces a redraw if pending, and briefly polls for resize events.
    pub(crate) fn pump_ui(&mut self) -> io::Result<()> {
        if !self.raw_mode {
            return self.draw();
        }
        // Process any pending input events from the channel (resize, scroll, typing)
        if let Ok(Some(queued)) = self.poll_busy_submission() {
            self.enqueue_submission(queued);
        }
        self.draw()
    }

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

        // Update Claude renderer terminal size
        self.claude.terminal_width = cols;
        self.claude.terminal_height = rows;

        // Sync modal state to Claude renderer
        self.claude.set_modal(self.state.modal.clone());

        // PRODUCTION: Always use Claude renderer
        self.draw_claude()?;

        Ok(())
    }

    fn draw_claude(&mut self) -> io::Result<()> {
        // Sync input to Claude renderer
        self.claude.set_input(self.input.lines().to_vec());
        self.claude
            .set_input_cursor(self.input.cursor_row(), self.input.display_col());
        self.claude.set_task_list(self.tasks.clone());

        // Note: scroll offset is managed solely by ClaudeTranscript (self.claude)
        // to avoid dual-source-of-truth bugs. Do NOT sync from UIState.

        // Sync autocomplete state (only when active)
        if self.state.autocomplete.active {
            self.claude
                .set_autocomplete_state(Some(&self.state.autocomplete));
        } else {
            self.claude.set_autocomplete_state(None);
        }

        // Compact status: always show context bar, append activity label when active
        let ctx_pct = if self.state.footer.context_max > 0 {
            ((self.state.footer.context_current * 100 / self.state.footer.context_max) as usize)
                .min(100)
        } else {
            0
        };
        let bar_width = 6;
        let filled = (ctx_pct * bar_width / 100) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));
        let base = format!("{}% {}  {}", ctx_pct, bar, self.state.footer.model);
        let status = match &self.state.activity {
            ActivityState::Active { label, .. } => Some(format!("{}  {}", base, label)),
            ActivityState::Idle => Some(base),
        };
        self.claude.set_status_line(status);

        // Notification with TTL expiry
        if let Some(ref note) = self.notification_line {
            let now = std::time::Instant::now();
            let expired = self.notification_expiry.map(|t| now >= t).unwrap_or(false);
            if expired {
                self.notification_line = None;
                self.notification_expiry = None;
            }
        }
        self.claude
            .set_notification_line(self.notification_line.clone());

        if let Some(terminal) = &mut self.terminal {
            terminal.draw(|f| {
                self.claude.render_ratatui(f);
            })?;
        } else {
            // Fallback for non-interactive or non-ratatui path
            let mut out = io::stdout();
            let (cols, rows) = size()?;
            let rows = rows as usize;

            let screen = self.claude.render();

            // Calculate which lines changed
            let lines_to_draw = if let Some(previous) = &self.previous_claude_screen {
                screen
                    .lines
                    .iter()
                    .take(rows)
                    .enumerate()
                    .filter(|(i, line)| {
                        previous
                            .get(*i)
                            .map_or(true, |prev_line| prev_line != *line)
                    })
                    .collect::<Vec<_>>()
            } else {
                // First draw, render everything
                screen
                    .lines
                    .iter()
                    .take(rows)
                    .enumerate()
                    .collect::<Vec<_>>()
            };

            // Draw only changed lines
            for (i, line) in lines_to_draw {
                execute!(out, MoveTo(0, i as u16))?;
                // Clear line and write new content
                execute!(out, Clear(ClearType::UntilNewLine))?;
                write!(out, "{}", line)?;
            }

            // Position cursor
            let cursor_row = screen.cursor_row.min((rows - 1) as u16);
            let cursor_col = screen.cursor_col.min((cols - 1) as u16);
            execute!(out, MoveTo(cursor_col, cursor_row), Show)?;
            out.flush()?;

            // Store current screen for next comparison
            self.previous_claude_screen = Some(screen.lines.clone());
        }

        Ok(())
    }

    // --- Input Loop ---

    /// Run the interactive input loop.
    /// Returns Some(input) when Enter is pressed, None when Esc is pressed.
    /// In non-interactive mode, reads a single line from stdin.
    pub(crate) async fn run_input_loop(&mut self) -> io::Result<Option<String>> {
        if !self.raw_mode {
            return read_line_non_interactive();
        }
        loop {
            self.draw()?;

            let ev = if let Ok(ev) = self.event_rx.try_recv() {
                ev
            } else {
                match self.event_rx.recv().await {
                    Some(ev) => ev,
                    None => return Ok(None),
                }
            };

            if let Event::Mouse(mouse_event) = ev {
                match mouse_event.kind {
                    MouseEventKind::ScrollDown => {
                        self.claude.scroll_down(3);
                        self.pending_draw = true;
                    }
                    MouseEventKind::ScrollUp => {
                        self.claude.scroll_up(3);
                        self.pending_draw = true;
                    }
                    _ => {}
                }
                continue;
            }

            if let Event::Key(KeyEvent {
                code,
                kind,
                modifiers,
                ..
            }) = ev
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
                            // For permission gate, accept selected option.
                            if let Some(ModalState::PermissionGate {
                                command, selected, ..
                            }) = &self.state.modal
                            {
                                match selected {
                                    0 => {
                                        /* Yes — approve once */
                                        crate::permission_gate::record_approval(command);
                                    }
                                    1 => {
                                        /* Always — approve and cache */
                                        crate::permission_gate::record_approval(command);
                                    }
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
                            // D denies permission gate.
                            if let Some(ModalState::PermissionGate { command, .. }) =
                                &self.state.modal
                            {
                                // Deny without caching
                                self.state.clear_modal();
                                self.pending_draw = true;
                            }
                        }
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            // Y approves permission gate once.
                            if let Some(ModalState::PermissionGate { command, .. }) =
                                &self.state.modal
                            {
                                crate::permission_gate::record_approval(command);
                                self.state.clear_modal();
                                self.pending_draw = true;
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            // A approves permission gate always.
                            if let Some(ModalState::PermissionGate { command, .. }) =
                                &self.state.modal
                            {
                                crate::permission_gate::record_approval(command);
                                self.state.clear_modal();
                                self.pending_draw = true;
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            // N denies permission gate.
                            if let Some(ModalState::PermissionGate { .. }) = &self.state.modal {
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
                            if let Some(ModalState::PermissionGate { selected, .. }) =
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
                            if let Some(ModalState::PermissionGate { selected, .. }) =
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

                // Handle model picker if visible
                if self.claude.model_picker.visible {
                    match code {
                        KeyCode::Esc => {
                            self.claude.hide_model_picker();
                            self.pending_draw = true;
                        }
                        KeyCode::Enter => {
                            // Select model
                            if let Some(_model) = self.claude.model_picker.selected_model() {
                                // TODO: Switch to selected model
                            }
                            self.claude.hide_model_picker();
                            self.pending_draw = true;
                        }
                        KeyCode::Up => {
                            self.claude.model_picker_select_prev();
                            self.pending_draw = true;
                        }
                        KeyCode::Down => {
                            self.claude.model_picker_select_next();
                            self.pending_draw = true;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle search modal if visible
                if self.claude.search_modal.visible {
                    match code {
                        KeyCode::Esc => {
                            self.claude.hide_search();
                            self.pending_draw = true;
                        }
                        KeyCode::Enter => {
                            // Select result
                            if let Some(_result) = self.claude.search_modal.selected_result() {
                                // TODO: Open file or jump to location
                            }
                            self.claude.hide_search();
                            self.pending_draw = true;
                        }
                        KeyCode::Up => {
                            self.claude.search_select_prev();
                            self.pending_draw = true;
                        }
                        KeyCode::Down => {
                            self.claude.search_select_next();
                            self.pending_draw = true;
                        }
                        KeyCode::Char(c) => {
                            let mut query = self.claude.search_modal.query.clone();
                            query.push(c);
                            self.claude.update_search_query(query);
                            self.pending_draw = true;
                        }
                        KeyCode::Backspace => {
                            let mut query = self.claude.search_modal.query.clone();
                            query.pop();
                            self.claude.update_search_query(query);
                            self.pending_draw = true;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle slash/file picker if active
                if self.claude.is_picker_active() {
                    match code {
                        KeyCode::Esc => {
                            self.claude.close_picker();
                            self.pending_draw = true;
                        }
                        KeyCode::Enter => {
                            if let Some(cmd) = self.claude.selected_slash_command() {
                                // Submit slash command immediately (single Enter)
                                self.input.set_content(cmd);
                                self.claude.close_picker();
                                self.input.push_to_history();
                                let input = self.input.content_trimmed();
                                self.input.clear();
                                self.pending_draw = true;
                                return Ok(Some(input));
                            } else if let Some(file) = self.claude.selected_file() {
                                let current = self.input.content();
                                if current.starts_with('@') {
                                    self.input.set_content(&format!("@{}", file));
                                } else {
                                    self.input.set_content(&file);
                                }
                                self.claude.close_picker();
                                self.pending_draw = true;
                            }
                        }
                        KeyCode::Up => {
                            self.claude.picker_select_up();
                            self.pending_draw = true;
                        }
                        KeyCode::Down => {
                            self.claude.picker_select_down();
                            self.pending_draw = true;
                        }
                        KeyCode::Char(c) => {
                            self.input.insert_char(c);
                            let content = self.input.content();
                            if content.starts_with('/') {
                                self.claude.open_slash_picker(content[1..].to_string());
                            } else if content.starts_with('@') && content.len() > 1 {
                                if let Ok(cwd) = std::env::current_dir() {
                                    self.claude.open_file_picker(content[1..].to_string(), &cwd);
                                }
                            } else {
                                self.claude.close_picker();
                            }
                            self.pending_draw = true;
                        }
                        KeyCode::Backspace => {
                            self.input.backspace();
                            let content = self.input.content();
                            if content.starts_with('/') {
                                self.claude.open_slash_picker(content[1..].to_string());
                            } else if content.starts_with('@') && content.len() > 1 {
                                if let Ok(cwd) = std::env::current_dir() {
                                    self.claude.open_file_picker(content[1..].to_string(), &cwd);
                                }
                            } else if content.is_empty() {
                                self.claude.close_picker();
                            }
                            self.pending_draw = true;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle async permission request
                if self.permission_tx.is_some() {
                    match code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            self.resolve_permission(true);
                            self.pending_draw = true;
                            continue;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            self.resolve_permission(false);
                            self.pending_draw = true;
                            continue;
                        }
                        _ => {}
                    }
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
                                // Ctrl+C: first clears input, second within window exits.
                                if !self.input.is_empty() {
                                    self.input.clear();
                                    self.ctrl_c_armed_until =
                                        Some(Instant::now() + Duration::from_millis(1200));
                                    self.notification_line =
                                        Some("Prompt cleared (Ctrl+C again to exit)".to_string());
                                    self.pending_draw = true;
                                } else {
                                    let now = Instant::now();
                                    if self.ctrl_c_armed_until.map(|t| now <= t).unwrap_or(false) {
                                        return Ok(None);
                                    }
                                    self.ctrl_c_armed_until =
                                        Some(now + Duration::from_millis(1200));
                                    self.notification_line =
                                        Some("Press Ctrl+C again to exit".to_string());
                                    self.pending_draw = true;
                                }
                            }
                            'd' => {
                                let now = Instant::now();
                                if self.ctrl_d_armed_until.map(|t| now <= t).unwrap_or(false) {
                                    return Ok(None);
                                }
                                self.ctrl_d_armed_until = Some(now + Duration::from_millis(1200));
                                self.notification_line =
                                    Some("Press Ctrl+D again to exit".to_string());
                                self.pending_draw = true;
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
                            'o' => {
                                // Ctrl+O: toggle transcript expanded mode
                                self.claude.toggle_transcript();
                                self.pending_draw = true;
                            }
                            't' => {
                                self.tasks.toggle();
                                self.pending_draw = true;
                            }
                            'u' => {
                                self.claude.scroll_up(5);
                                self.pending_draw = true;
                            }
                            'd' => {
                                self.claude.scroll_down(5);
                                self.pending_draw = true;
                            }
                            'k' => {
                                self.claude.show_search();
                                self.pending_draw = true;
                            }
                            'm' => {
                                self.claude.show_model_picker();
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
                        let content = self.input.content();
                        if content.starts_with('/') {
                            self.claude.open_slash_picker(content[1..].to_string());
                        } else if content.starts_with('@') && content.len() > 1 {
                            if let Ok(cwd) = std::env::current_dir() {
                                self.claude.open_file_picker(content[1..].to_string(), &cwd);
                            }
                        } else if content == "!" {
                            self.claude
                                .set_input_mode(crate::claude_ui::claude_input::InputMode::Bash);
                        } else {
                            self.claude.close_picker();
                            if self.claude.input_mode()
                                != &crate::claude_ui::claude_input::InputMode::Chat
                            {
                                self.claude.set_input_mode(
                                    crate::claude_ui::claude_input::InputMode::Chat,
                                );
                            }
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
                        let content = self.input.content();
                        if content.starts_with('/') {
                            self.claude.open_slash_picker(content[1..].to_string());
                        } else if content.starts_with('@') && content.len() > 1 {
                            if let Ok(cwd) = std::env::current_dir() {
                                self.claude.open_file_picker(content[1..].to_string(), &cwd);
                            }
                        } else if content.is_empty() || content == "!" {
                            self.claude.close_picker();
                            self.claude
                                .set_input_mode(crate::claude_ui::claude_input::InputMode::Chat);
                        }
                        self.pending_draw = true;
                    }
                    KeyCode::Delete => {
                        self.input.delete();
                        let content = self.input.content();
                        if content.starts_with('/') {
                            self.claude.open_slash_picker(content[1..].to_string());
                        } else if content.starts_with('@') && content.len() > 1 {
                            if let Ok(cwd) = std::env::current_dir() {
                                self.claude.open_file_picker(content[1..].to_string(), &cwd);
                            }
                        } else {
                            self.claude.close_picker();
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
                        self.claude.scroll_to_bottom();
                        self.input.move_end();
                        self.pending_draw = true;
                    }
                    KeyCode::PageUp => {
                        self.claude.scroll_up(5);
                        self.pending_draw = true;
                    }
                    KeyCode::PageDown => {
                        self.claude.scroll_down(5);
                        self.pending_draw = true;
                    }
                    KeyCode::Up => {
                        // Claude Code behavior: Up/Down scroll transcript, never history
                        if self.claude.is_picker_active() {
                            self.claude.picker_select_up();
                        } else if self.state.autocomplete.active {
                            self.state.autocomplete.select_up();
                        } else {
                            self.claude.scroll_up(3);
                        }
                        self.pending_draw = true;
                    }
                    KeyCode::Down => {
                        // Claude Code behavior: Up/Down scroll transcript, never history
                        if self.claude.is_picker_active() {
                            self.claude.picker_select_down();
                        } else if self.state.autocomplete.active {
                            self.state.autocomplete.select_down();
                        } else {
                            self.claude.scroll_down(3);
                        }
                        self.pending_draw = true;
                    }
                    KeyCode::Esc => {
                        if self.claude.is_picker_active() {
                            self.claude.close_picker();
                            self.pending_draw = true;
                        } else if self.state.autocomplete.active {
                            self.state.autocomplete.deactivate();
                            self.pending_draw = true;
                        } else {
                            let now = Instant::now();
                            if self.esc_armed_until.map(|t| now <= t).unwrap_or(false) {
                                self.input.clear();
                                self.notification_line = Some("Prompt cleared".to_string());
                                self.esc_armed_until = None;
                                self.pending_draw = true;
                            } else {
                                self.esc_armed_until = Some(now + Duration::from_millis(900));
                                self.notification_line =
                                    Some("Press Esc again to clear prompt".to_string());
                                self.pending_draw = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Poll input while the model/tool pipeline is busy.
    /// Returns a submitted prompt (Enter) that should be queued for later processing.
    pub(crate) fn poll_busy_submission(&mut self) -> io::Result<Option<String>> {
        if !self.raw_mode {
            return Ok(None);
        }

        // Process all pending events from the async channel.
        // The background input reader thread is the ONLY thing that calls
        // crossterm::event::poll / crossterm::event::read.
        if self.claude.is_picker_active() {
            while let Ok(ev) = self.event_rx.try_recv() {
                let Event::Key(KeyEvent {
                    code,
                    kind,
                    modifiers,
                    ..
                }) = ev
                else {
                    continue;
                };

                if kind != KeyEventKind::Press {
                    continue;
                }

                match code {
                    KeyCode::Esc => {
                        self.claude.close_picker();
                        self.pending_draw = true;
                    }
                    KeyCode::Enter => {
                        if let Some(cmd) = self.claude.selected_slash_command() {
                            self.input.set_content(cmd);
                            self.claude.close_picker();
                            self.input.push_to_history();
                            let input = self.input.content_trimmed();
                            self.input.clear();
                            self.pending_draw = true;
                            return Ok(Some(input));
                        } else if let Some(file) = self.claude.selected_file() {
                            let current = self.input.content();
                            if current.starts_with('@') {
                                self.input.set_content(&format!("@{}", file));
                            } else {
                                self.input.set_content(&file);
                            }
                            self.claude.close_picker();
                            self.pending_draw = true;
                        }
                    }
                    KeyCode::Up => {
                        self.claude.picker_select_up();
                        self.pending_draw = true;
                    }
                    KeyCode::Down => {
                        self.claude.picker_select_down();
                        self.pending_draw = true;
                    }
                    KeyCode::Char(c) => {
                        self.input.insert_char(c);
                        let content = self.input.content();
                        if content.starts_with('/') {
                            self.claude.open_slash_picker(content[1..].to_string());
                        } else if content.starts_with('@') && content.len() > 1 {
                            if let Ok(cwd) = std::env::current_dir() {
                                self.claude.open_file_picker(content[1..].to_string(), &cwd);
                            }
                        } else {
                            self.claude.close_picker();
                        }
                        self.pending_draw = true;
                    }
                    KeyCode::Backspace => {
                        self.input.backspace();
                        let content = self.input.content();
                        if content.starts_with('/') {
                            self.claude.open_slash_picker(content[1..].to_string());
                        } else if content.starts_with('@') && content.len() > 1 {
                            if let Ok(cwd) = std::env::current_dir() {
                                self.claude.open_file_picker(content[1..].to_string(), &cwd);
                            }
                        } else if content.is_empty() {
                            self.claude.close_picker();
                        }
                        self.pending_draw = true;
                    }
                    _ => {}
                }
            }
            return Ok(None);
        }

        while let Ok(ev) = self.event_rx.try_recv() {
            // Handle resize
            if let Event::Resize(_, _) = ev {
                self.previous_claude_screen = None;
                self.pending_draw = true;
                continue;
            }

            // Handle mouse events (trackpad scroll)
            if let Event::Mouse(mouse_event) = ev {
                match mouse_event.kind {
                    MouseEventKind::ScrollDown => {
                        self.claude.scroll_down(3);
                        self.pending_draw = true;
                    }
                    MouseEventKind::ScrollUp => {
                        self.claude.scroll_up(3);
                        self.pending_draw = true;
                    }
                    _ => {}
                }
                continue;
            }

            let Event::Key(KeyEvent {
                code,
                kind,
                modifiers,
                ..
            }) = ev
            else {
                continue;
            };

            if kind != KeyEventKind::Press {
                continue;
            }

            if self.state.modal.is_some() {
                match code {
                    KeyCode::Esc => {
                        self.state.clear_modal();
                        self.pending_draw = true;
                    }
                    _ => {}
                }
                continue;
            }

            // Handle permission request first
            if self.permission_tx.is_some() {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.resolve_permission(true);
                        self.pending_draw = true;
                        continue;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.resolve_permission(false);
                        self.pending_draw = true;
                        continue;
                    }
                    _ => {}
                }
            }

            match code {
                KeyCode::Enter if modifiers.contains(KeyModifiers::CONTROL) => {
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
                    self.pending_draw = true;
                    return Ok(Some(input));
                }
                KeyCode::Enter => {
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
                    self.pending_draw = true;
                    return Ok(Some(input));
                }
                KeyCode::Tab => {
                    if self.state.autocomplete.active && !self.state.autocomplete.matches.is_empty()
                    {
                        self.state.autocomplete.select_down();
                        self.pending_draw = true;
                    }
                }
                KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => {
                    // Shift+Enter: insert newline for multiline input
                    self.input.insert_newline();
                    self.pending_draw = true;
                }
                KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input.insert_newline();
                    self.pending_draw = true;
                }
                KeyCode::Char(c) if modifiers.contains(KeyModifiers::CONTROL) => match c {
                    'a' => {
                        self.input.move_home();
                        self.pending_draw = true;
                    }
                    'e' => {
                        self.input.move_end();
                        self.pending_draw = true;
                    }
                    'b' => {
                        self.input.move_left();
                        self.pending_draw = true;
                    }
                    'f' => {
                        self.input.move_right();
                        self.pending_draw = true;
                    }
                    'u' => {
                        self.input.delete_to_line_start();
                        self.pending_draw = true;
                    }
                    'w' => {
                        self.input.delete_word_before();
                        self.pending_draw = true;
                    }
                    'o' => {
                        // Toggle transcript mode (Normal <-> Transcript)
                        self.claude.transcript_mode = match self.claude.transcript_mode {
                            crate::claude_ui::claude_render::TranscriptMode::Normal => {
                                self.claude.set_transcript_expanded(true);
                                crate::claude_ui::claude_render::TranscriptMode::Transcript
                            }
                            _ => {
                                self.claude.set_transcript_expanded(false);
                                crate::claude_ui::claude_render::TranscriptMode::Normal
                            }
                        };
                        self.pending_draw = true;
                    }
                    'u' => {
                        // Half page up
                        self.claude.scroll_up(5);
                        self.pending_draw = true;
                    }
                    'd' => {
                        // Half page down
                        self.claude.scroll_down(5);
                        self.pending_draw = true;
                    }
                    't' => {
                        self.tasks.toggle();
                        self.pending_draw = true;
                    }
                    _ => {}
                },
                KeyCode::Char(c) => {
                    // Transcript mode shortcuts
                    match self.claude.transcript_mode {
                        crate::claude_ui::claude_render::TranscriptMode::Transcript => match c {
                            'q' => {
                                self.claude.transcript_mode =
                                    crate::claude_ui::claude_render::TranscriptMode::Normal;
                                self.claude.set_transcript_expanded(false);
                                self.pending_draw = true;
                                continue;
                            }
                            'g' => {
                                self.claude.scroll_to_bottom();
                                self.pending_draw = true;
                                continue;
                            }
                            'G' => {
                                self.claude.scroll_up(999999);
                                self.pending_draw = true;
                                continue;
                            }
                            'j' => {
                                self.claude.scroll_up(1);
                                self.pending_draw = true;
                                continue;
                            }
                            'k' => {
                                self.claude.scroll_down(1);
                                self.pending_draw = true;
                                continue;
                            }
                            'b' => {
                                self.claude.scroll_up(10);
                                self.pending_draw = true;
                                continue;
                            }
                            ' ' => {
                                self.claude.scroll_down(10);
                                self.pending_draw = true;
                                continue;
                            }
                            '/' => {
                                self.claude.transcript_mode =
                                    crate::claude_ui::claude_render::TranscriptMode::Search {
                                        query: String::new(),
                                        matches: Vec::new(),
                                        current: 0,
                                    };
                                self.pending_draw = true;
                                continue;
                            }
                            _ => {}
                        },
                        crate::claude_ui::claude_render::TranscriptMode::Search {
                            ref mut query,
                            ..
                        } => {
                            match c {
                                'n' => {
                                    // Next match
                                    self.pending_draw = true;
                                    continue;
                                }
                                'N' => {
                                    // Previous match
                                    self.pending_draw = true;
                                    continue;
                                }
                                _ => {
                                    query.push(c);
                                    self.pending_draw = true;
                                    continue;
                                }
                            }
                        }
                        _ => {}
                    }
                    self.input.insert_char(c);
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
                    self.claude.scroll_to_bottom();
                    self.input.move_end();
                    self.pending_draw = true;
                }
                KeyCode::PageUp => {
                    self.claude.scroll_up(5);
                    self.pending_draw = true;
                }
                KeyCode::PageDown => {
                    self.claude.scroll_down(5);
                    self.pending_draw = true;
                }
                KeyCode::Up => {
                    // Claude Code behavior: Up/Down scroll transcript, never history
                    if self.claude.is_picker_active() {
                        self.claude.picker_select_up();
                    } else if self.state.autocomplete.active {
                        self.state.autocomplete.select_up();
                    } else {
                        self.claude.scroll_up(3);
                    }
                    self.pending_draw = true;
                }
                KeyCode::Down => {
                    // Claude Code behavior: Up/Down scroll transcript, never history
                    if self.claude.is_picker_active() {
                        self.claude.picker_select_down();
                    } else if self.state.autocomplete.active {
                        self.state.autocomplete.select_down();
                    } else {
                        self.claude.scroll_down(3);
                    }
                    self.pending_draw = true;
                }
                KeyCode::PageUp => {
                    self.claude.scroll_up(5);
                    self.pending_draw = true;
                }
                KeyCode::PageDown => {
                    self.claude.scroll_down(5);
                    self.pending_draw = true;
                }
                KeyCode::Up => {
                    // Claude Code behavior: Up/Down scroll transcript, never history
                    if self.claude.is_picker_active() {
                        self.claude.picker_select_up();
                    } else if self.state.autocomplete.active {
                        self.state.autocomplete.select_up();
                    } else {
                        self.claude.scroll_up(3);
                    }
                    self.pending_draw = true;
                }
                KeyCode::Down => {
                    // Claude Code behavior: Up/Down scroll transcript, never history
                    if self.claude.is_picker_active() {
                        self.claude.picker_select_down();
                    } else if self.state.autocomplete.active {
                        self.state.autocomplete.select_down();
                    } else {
                        self.claude.scroll_down(3);
                    }
                    self.pending_draw = true;
                }
                KeyCode::Esc => {
                    if self.claude.is_picker_active() {
                        self.claude.close_picker();
                        self.pending_draw = true;
                    } else if self.state.autocomplete.active {
                        self.state.autocomplete.deactivate();
                        self.pending_draw = true;
                    } else {
                        let now = Instant::now();
                        if self.esc_armed_until.map(|t| now <= t).unwrap_or(false) {
                            self.input.clear();
                            self.notification_line = Some("Prompt cleared".to_string());
                            self.esc_armed_until = None;
                            self.pending_draw = true;
                        } else {
                            self.esc_armed_until = Some(now + Duration::from_millis(800));
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(None)
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
            execute!(
                io::stdout(),
                Show,
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal::disable_raw_mode()?;
            io::stdout().flush()?;
            self.raw_mode = false;
        }
        Ok(())
    }
}

fn read_line_non_interactive() -> io::Result<Option<String>> {
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(line.trim().to_string())),
        Err(e) => Err(e),
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
        let _ = MessageRole::Thinking;
        let _ = MessageRole::System;
    }
}
