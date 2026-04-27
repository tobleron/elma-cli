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
    // Task 268: Background task panel
    background_tasks_visible: bool,
    selected_background_task: Option<String>,
    // Double-key chord state
    esc_armed_until: Option<Instant>,
    ctrl_c_armed_until: Option<Instant>,
    ctrl_d_armed_until: Option<Instant>,
    queued_submissions: VecDeque<String>,
    // Async permission request channel
    permission_tx: Option<tokio::sync::oneshot::Sender<bool>>,
    // Async event channel: background thread reads crossterm events,
    // all TerminalUI methods drain from here. This is the ONLY event source.
    event_rx: tokio::sync::mpsc::UnboundedReceiver<Event>,
    // Incremental token counter for transcript
    transcript_token_estimate: u64,
}

#[cfg(unix)]
fn is_stdin_tty() -> bool {
    std::io::stdin().is_terminal()
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
            let mut terminal = ratatui::Terminal::new(backend)?;
            terminal.clear()?;
            Some(terminal)
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
            queued_submissions: VecDeque::new(),
            permission_tx: None,
            event_rx,
            transcript_token_estimate: 0,
            background_tasks_visible: false,
            selected_background_task: None,
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

        self.transcript_token_estimate += content.len() as u64 / 4;

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
                        content: crate::claude_ui::AssistantContent::from_markdown(&content),
                    });
                }
            }
            MessageRole::Thinking => {
                self.claude.push_message(ClaudeMessage::Thinking {
                    content: content.clone(),
                    is_streaming: false,
                    word_count: content.split_whitespace().count(),
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

    pub(crate) fn update_context_tokens(&mut self, tokens: u64) {
        self.state.footer.context_current = tokens;
        self.pending_draw = true;
    }

    pub(crate) fn get_context_max(&self) -> u64 {
        self.state.footer.context_max
    }

    pub(crate) fn get_context_current(&self) -> u64 {
        self.state.footer.context_current
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

    fn push_notice(
        &mut self,
        kind: crate::claude_ui::UiNoticeKind,
        persistence: crate::claude_ui::NoticePersistence,
        content: &str,
    ) {
        let notice = crate::claude_ui::UiNotice {
            kind,
            content: content.to_string(),
            created_at: Instant::now(),
            persistence: persistence.clone(),
            collapsed: false,
        };
        if persistence == crate::claude_ui::NoticePersistence::EphemeralPromptHint {
            self.claude.set_prompt_hint(Some(notice));
        } else {
            self.claude
                .push_message(crate::claude_ui::ClaudeMessage::Notice(notice));
        }
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

    // --- Status thread ---

    pub(crate) fn start_status(&mut self, description: &str) {
        self.state.start_status(description);
        self.pending_draw = true;
    }

    pub(crate) fn complete_status(&mut self, description: &str) {
        self.state.complete_status(description);
        self.pending_draw = true;
    }

    pub(crate) fn clear_status(&mut self) {
        self.state.clear_status();
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
        self.transcript_token_estimate = 0;
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

    // === Task 268: Background Task Panel ===

    pub(crate) fn toggle_background_tasks(&mut self) {
        self.background_tasks_visible = !self.background_tasks_visible;
    }

    pub(crate) fn is_background_tasks_visible(&self) -> bool {
        self.background_tasks_visible
    }

    pub(crate) fn select_next_background_task(&mut self, task_ids: &[String]) {
        if task_ids.is_empty() {
            self.selected_background_task = None;
            return;
        }
        let current = self.selected_background_task.as_ref();
        let idx = task_ids
            .iter()
            .position(|id| current.map(|c| c == id).unwrap_or(false));
        match idx {
            Some(i) => {
                let next = (i + 1) % task_ids.len();
                self.selected_background_task = Some(task_ids[next].clone());
            }
            None => self.selected_background_task = Some(task_ids[0].clone()),
        }
    }

    pub(crate) fn select_previous_background_task(&mut self, task_ids: &[String]) {
        if task_ids.is_empty() {
            self.selected_background_task = None;
            return;
        }
        let current = self.selected_background_task.as_ref();
        let idx = task_ids
            .iter()
            .position(|id| current.map(|c| c == id).unwrap_or(false));
        match idx {
            Some(i) => {
                let prev = if i == 0 { task_ids.len() - 1 } else { i - 1 };
                self.selected_background_task = Some(task_ids[prev].clone());
            }
            None => self.selected_background_task = Some(task_ids.last().unwrap().clone()),
        }
    }

    pub(crate) fn get_selected_background_task(&self) -> Option<&str> {
        self.selected_background_task.as_deref()
    }

    pub(crate) fn render_background_tasks_panel(
        &self,
        tasks: &[crate::background_task::BackgroundTask],
        width: usize,
    ) -> Vec<ratatui::prelude::Line<'static>> {
        use ratatui::prelude::*;
        use ratatui::widgets::*;

        let mut lines = Vec::new();

        if tasks.is_empty() {
            lines.push(Line::from(" No background tasks ".fg(Color::Gray)));
            return lines;
        }

        let header = Line::from(vec![
            " Background Tasks ".bold(),
            format!(" ({}) ", tasks.len()).fg(Color::Gray),
        ]);
        lines.push(header);

        for task in tasks {
            let is_selected = self
                .selected_background_task
                .as_ref()
                .map(|s| s == &task.id)
                .unwrap_or(false);

            let prefix = if is_selected { "▸ " } else { "  " };

            let status_color = match task.status {
                crate::background_task::BackgroundTaskStatus::Pending => Color::Yellow,
                crate::background_task::BackgroundTaskStatus::Running => Color::Cyan,
                crate::background_task::BackgroundTaskStatus::Completed => Color::Green,
                crate::background_task::BackgroundTaskStatus::Failed => Color::Red,
                crate::background_task::BackgroundTaskStatus::Cancelled => Color::DarkGray,
                crate::background_task::BackgroundTaskStatus::OOMKilled => Color::Magenta,
            };

            let runtime = task
                .runtime_seconds()
                .map(|s| format!("{}s", s))
                .unwrap_or_else(|| "-".to_string());

            let row = format!(
                "{}{} [{}] Mem:{}MB Time:{}",
                prefix,
                task.name,
                task.status.to_string().fg(status_color),
                task.memory_usage_mb,
                runtime
            );

            let line = if is_selected {
                Line::from(row.fg(Color::White).bg(Color::DarkGray))
            } else {
                Line::from(row.fg(Color::Gray))
            };
            lines.push(line);
        }

        lines
    }

    pub(crate) fn notify(&mut self, message: &str) {
        self.push_notice(
            crate::claude_ui::UiNoticeKind::Session,
            crate::claude_ui::NoticePersistence::EphemeralPromptHint,
            message,
        );
    }

    pub(crate) fn push_budget_notice(&mut self, message: &str) {
        self.push_notice(
            crate::claude_ui::UiNoticeKind::Budget,
            crate::claude_ui::NoticePersistence::TranscriptCollapsible,
            message,
        );
    }

    pub(crate) fn push_compaction_notice(&mut self, message: &str) {
        self.push_notice(
            crate::claude_ui::UiNoticeKind::Compaction,
            crate::claude_ui::NoticePersistence::TranscriptCollapsible,
            message,
        );
    }

    pub(crate) fn push_stop_notice(&mut self, message: &str) {
        self.push_notice(
            crate::claude_ui::UiNoticeKind::StopReason,
            crate::claude_ui::NoticePersistence::TranscriptPersistent,
            message,
        );
    }

    pub(crate) fn enqueue_submission(&mut self, input: String) {
        self.queued_submissions.push_back(input);
        self.push_notice(
            crate::claude_ui::UiNoticeKind::Queue,
            crate::claude_ui::NoticePersistence::TranscriptCollapsible,
            "1 message queued (will run after current response)",
        );
    }

    pub(crate) fn take_queued_submissions(&mut self) -> Vec<String> {
        self.queued_submissions.drain(..).collect()
    }

    /// Estimate tokens from text (rough: ~4 chars per token).
    #[allow(dead_code)]
    pub(crate) fn estimate_tokens(text: &str) -> u64 {
        text.len() as u64 / 4
    }

    /// Handle a mouse click in the transcript area to toggle tool trace collapse.
    fn handle_transcript_click(&mut self, mouse_event: &crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;
        if !matches!(mouse_event.kind, MouseEventKind::Down(_)) {
            return;
        }
        if let Some(area) = self.claude.last_content_area {
            let row = mouse_event.row;
            let col = mouse_event.column;
            // Check if click is within the content area
            if row >= area.y
                && row < area.y + area.height
                && col >= area.x
                && col < area.x + area.width
            {
                let relative_line = (row - area.y) as usize;
                let absolute_line = self.claude.last_start_line + relative_line;
                if let Some(&msg_idx) = self.claude.last_line_mapping.get(absolute_line) {
                    self.claude.transcript.toggle_trace_collapse(msg_idx);
                    self.pending_draw = true;
                }
            }
        }
    }

    #[cfg(test)]
    fn estimate_transcript_tokens(&self) -> u64 {
        use crate::claude_ui::ClaudeMessage;
        let mut total = 0u64;
        for msg in &self.claude.transcript.messages {
            let text = match msg {
                ClaudeMessage::User { content } => content.as_str(),
                ClaudeMessage::Assistant { content } => content.raw_markdown.as_str(),
                ClaudeMessage::Thinking { content, .. } => content.as_str(),
                ClaudeMessage::ToolTrace {
                    command, status, ..
                } => {
                    total += command.len() as u64 / 4;
                    if let crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                        output,
                        ..
                    } = status
                    {
                        total += output.len() as u64 / 4;
                    }
                    continue;
                }
                ClaudeMessage::ToolResult { output, .. } => {
                    total += output.len() as u64 / 4;
                    continue;
                }
                ClaudeMessage::System { content } => content.as_str(),
                ClaudeMessage::Notice(notice) => notice.content.as_str(),
                _ => continue,
            };
            total += text.len() as u64 / 4;
        }
        total += self.claude.streaming.thinking.len() as u64 / 4;
        total += self.claude.streaming.content.len() as u64 / 4;
        total
    }

    // --- Drawing ---

    /// Non-blocking UI pump — call between async steps to keep UI alive.
    /// Forces a redraw if pending, and briefly polls for resize events.
    pub(crate) fn pump_ui(&mut self) -> io::Result<()> {
        // Keep repainting while the status thread is active (spinner animation)
        if self.state.status_thread.is_working() {
            self.pending_draw = true;
        }
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

        // Estimate the current model budget (base from last update + streaming)
        let streaming_tokens =
            (self.claude.streaming.thinking.len() + self.claude.streaming.content.len()) as u64 / 4;
        let model_context_tokens_estimate = self.state.footer.context_current + streaming_tokens;
        let transcript_tokens_estimate = self.transcript_token_estimate;

        // Compact status: always show context bar, using model budget
        let ctx_pct = if self.state.footer.context_max > 0 {
            ((model_context_tokens_estimate * 100) / self.state.footer.context_max) as usize
        } else {
            0
        }
        .min(100);

        let bar_width = 6;
        let filled = (ctx_pct * bar_width / 100) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));

        let transcript_metric = if self.state.header.verbose
            && transcript_tokens_estimate > model_context_tokens_estimate + 500
        {
            Some(format!("tx {}", transcript_tokens_estimate))
        } else {
            None
        };
        let _ = bar;
        self.claude
            .set_footer_model(Some(crate::claude_ui::claude_render::FooterModel {
                context_pct: Some(ctx_pct),
                model_label: Some(self.state.footer.model.clone()),
                transcript_metric,
            }));

        // Sync status thread state (UIState is authoritative, ClaudeRenderer renders it)
        self.claude.status_thread = self.state.status_thread.clone();

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
                let redraw_deadline = self.claude.next_redraw_deadline();
                if let Some(deadline) = redraw_deadline {
                    let now = Instant::now();
                    if now < deadline {
                        match tokio::time::timeout(deadline - now, self.event_rx.recv()).await {
                            Ok(Some(ev)) => ev,
                            Ok(None) => return Ok(None),
                            Err(_) => {
                                self.pending_draw = true;
                                continue;
                            }
                        }
                    } else {
                        self.pending_draw = true;
                        continue;
                    }
                } else {
                    match self.event_rx.recv().await {
                        Some(ev) => ev,
                        None => return Ok(None),
                    }
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
                    _ => {
                        self.handle_transcript_click(&mouse_event);
                    }
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
                                    self.push_notice(
                                        crate::claude_ui::UiNoticeKind::InputHint,
                                        crate::claude_ui::NoticePersistence::EphemeralPromptHint,
                                        "Prompt cleared (Ctrl+C again to exit)",
                                    );
                                    self.pending_draw = true;
                                } else {
                                    let now = Instant::now();
                                    if self.ctrl_c_armed_until.map(|t| now <= t).unwrap_or(false) {
                                        return Ok(None);
                                    }
                                    self.ctrl_c_armed_until =
                                        Some(now + Duration::from_millis(1200));
                                    self.push_notice(
                                        crate::claude_ui::UiNoticeKind::InputHint,
                                        crate::claude_ui::NoticePersistence::EphemeralPromptHint,
                                        "Press Ctrl+C again to exit",
                                    );
                                    self.pending_draw = true;
                                }
                            }
                            'd' => {
                                let now = Instant::now();
                                if self.ctrl_d_armed_until.map(|t| now <= t).unwrap_or(false) {
                                    return Ok(None);
                                }
                                self.ctrl_d_armed_until = Some(now + Duration::from_millis(1200));
                                self.push_notice(
                                    crate::claude_ui::UiNoticeKind::InputHint,
                                    crate::claude_ui::NoticePersistence::EphemeralPromptHint,
                                    "Press Ctrl+D again to exit",
                                );
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
                                self.push_notice(
                                    crate::claude_ui::UiNoticeKind::InputHint,
                                    crate::claude_ui::NoticePersistence::EphemeralPromptHint,
                                    "Prompt cleared",
                                );
                                self.esc_armed_until = None;
                                self.pending_draw = true;
                            } else {
                                self.esc_armed_until = Some(now + Duration::from_millis(900));
                                self.push_notice(
                                    crate::claude_ui::UiNoticeKind::InputHint,
                                    crate::claude_ui::NoticePersistence::EphemeralPromptHint,
                                    "Press Esc again to clear prompt",
                                );
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

            // Handle mouse events (trackpad scroll + click to expand traces)
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
                    _ => {
                        self.handle_transcript_click(&mouse_event);
                    }
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
                            self.push_notice(
                                crate::claude_ui::UiNoticeKind::InputHint,
                                crate::claude_ui::NoticePersistence::EphemeralPromptHint,
                                "Prompt cleared",
                            );
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
            // Persist visible transcript (plain-text) so terminal output is saved
            // under sessions/ for debugging and replay.
            let transcript_path = std::env::current_dir()
                .ok()
                .map(|d| d.join("sessions").join("terminal_transcript.txt"));
            if let Some(ref tpath) = transcript_path {
                if let Some(parent) = tpath.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut out = String::new();
                out.push_str(&format!(
                    "=== Terminal Transcript ({}) ===\n\n",
                    chrono::Local::now().to_rfc3339()
                ));
                for msg in &self.claude.transcript.messages {
                    use crate::claude_ui::claude_state::ClaudeMessage;
                    match msg {
                        ClaudeMessage::User { content } => {
                            out.push_str(&format!("> {}\n\n", content));
                        }
                        ClaudeMessage::Assistant { content } => {
                            out.push_str(&format!("● {}\n\n", content.raw_markdown));
                        }
                        ClaudeMessage::Thinking { content, .. } => {
                            out.push_str(&format!("∴ Thinking: {}\n\n", content));
                        }
                        ClaudeMessage::ToolStart { name, input } => {
                            out.push_str(&format!("▸ Tool start: {}\n", name));
                            if let Some(i) = input {
                                out.push_str(&format!("input: {}\n", i));
                            }
                            out.push_str("\n");
                        }
                        ClaudeMessage::ToolProgress { name, message } => {
                            out.push_str(&format!("▸ Tool progress ({}): {}\n\n", name, message));
                        }
                        ClaudeMessage::ToolResult {
                            name,
                            success,
                            output,
                            duration_ms,
                        } => {
                            out.push_str(&format!(
                                "✓ Tool result ({}): success={} duration_ms={:?}\n{}\n\n",
                                name, success, duration_ms, output
                            ));
                        }
                        ClaudeMessage::ToolTrace {
                            name,
                            command,
                            status,
                            ..
                        } => {
                            out.push_str(&format!("▸ Tool trace ({}): {}\n", name, command));
                            match status {
                                crate::claude_ui::claude_state::ToolTraceStatus::Running => {
                                    out.push_str("status: running\n\n");
                                }
                                crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                                    success,
                                    output,
                                    duration_ms,
                                } => {
                                    out.push_str(&format!(
                                        "status: completed success={} duration_ms={:?}\n{}\n\n",
                                        success, duration_ms, output
                                    ));
                                }
                            }
                        }
                        ClaudeMessage::PermissionRequest { command, reason } => {
                            out.push_str(&format!(
                                "? Permission requested: {} reason={:?}\n\n",
                                command, reason
                            ));
                        }
                        ClaudeMessage::CompactBoundary => {
                            out.push_str("✻ Conversation compacted\n\n");
                        }
                        ClaudeMessage::CompactSummary {
                            message_count,
                            context_preview,
                        } => {
                            out.push_str(&format!(
                                "✻ Compact summary: {} messages\n{}\n\n",
                                message_count,
                                context_preview.as_deref().unwrap_or("")
                            ));
                        }
                        ClaudeMessage::System { content } => {
                            out.push_str(&format!("system: {}\n\n", content));
                        }
                        ClaudeMessage::Notice(notice) => {
                            out.push_str(&format!(
                                "◦ NOTICE ({:?}): {}\n\n",
                                notice.kind, notice.content
                            ));
                        }
                    }
                }
                let _ = std::fs::write(tpath, out);
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

impl Drop for TerminalUI {
    fn drop(&mut self) {
        // Best-effort cleanup — errors are not propagable from Drop.
        // The explicit cleanup() call in the chat loop handles the clean-exit case;
        // this Drop handles panics and propagated errors.
        let _ = self.cleanup();
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

    #[test]
    fn test_transcript_budget_divergence() {
        let mut tui = TerminalUI::new().unwrap();
        // Model context budget is small (e.g. 100 tokens from prompt)
        tui.update_context_tokens(100);

        // A massive tool trace in the transcript (e.g. 40,000 bytes = 10,000 tokens)
        let big_output = "x".repeat(40000);
        tui.push_tool_start("shell", "find / -type f");
        tui.push_tool_finish("shell", true, &big_output, Some(100));

        let transcript_tokens = tui.estimate_transcript_tokens();
        assert!(
            transcript_tokens >= 10000,
            "Transcript tokens should include the massive tool output"
        );

        let streaming_tokens =
            (tui.claude.streaming.thinking.len() + tui.claude.streaming.content.len()) as u64 / 4;
        let model_budget = tui.state.footer.context_current + streaming_tokens;
        assert_eq!(
            model_budget, 100,
            "Model budget should not be polluted by the transcript tool traces"
        );
        assert!(transcript_tokens > model_budget);
    }
}
