//! @efficiency-role: ui-component
//!
//! Claude Code-style Terminal Renderer
//!
//! Design (from Claude Code study):
//! - Sparse message rows, no persistent header/activity/context chrome
//! - Prompt at bottom, transient picker modals only
//! - Uses theme tokens from ui_theme.rs

use super::claude_input::{InputMode, PickerState, SLASH_COMMANDS};
use super::claude_markdown::AssistantContent;
use crate::ui_autocomplete;
use super::claude_state::{
    ClaudeMessage, ClaudeTranscript, NoticePersistence, UiNotice, FOOTER_HINTS,
};
use super::claude_stream::StreamingUI;
use crate::ui_theme::*;
use ratatui::prelude::*;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::*;
use std::path::PathBuf;
use std::time::Instant;

// ============================================================================
// Screen Buffer (Legacy/Compatibility)
// ============================================================================

pub(crate) struct ClaudeScreen {
    pub lines: Vec<String>,
    pub cursor_row: u16,
    pub cursor_col: u16,
}

// ============================================================================
// Renderer
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) enum TranscriptMode {
    #[default]
    Normal,
    Transcript,
    Search {
        query: String,
        matches: Vec<usize>,
        current: usize,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FooterModel {
    pub context_pct: Option<usize>,
    pub model_label: Option<String>,
    pub transcript_metric: Option<String>,
}

pub(crate) struct ClaudeRenderer {
    pub(crate) transcript: ClaudeTranscript,
    input_lines: Vec<String>,
    input_cursor_row: usize,
    input_cursor_col: usize,
    pub terminal_width: usize,
    pub terminal_height: usize,
    pub(crate) streaming: StreamingUI,
    task_list: Option<crate::claude_ui::claude_tasks::TaskList>,
    footer_model: Option<FooterModel>,
    prompt_hint: Option<UiNotice>,
    pub search_modal: crate::ui::ui_modal_search::SearchModal,
    pub model_picker: crate::ui::ui_model_picker::ModelPicker,
    pub autocomplete_state: Option<crate::ui::ui_autocomplete::AutocompleteState>,
    picker_state: PickerState,
    input_mode: InputMode,
    file_matches: Vec<String>,
    pub transcript_mode: TranscriptMode,
    modal_state: Option<crate::ui_state::ModalState>,
    scrollbar_state: ScrollbarState,
    // Animation frame counter for streaming indicators
    anim_frame: usize,
    // Hit-testing state for click-to-expand tool traces
    pub(crate) last_content_area: Option<ratatui::layout::Rect>,
    pub(crate) last_start_line: usize,
    pub(crate) last_line_mapping: Vec<usize>,
}

impl ClaudeRenderer {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            transcript: ClaudeTranscript::new(),
            input_lines: vec![String::new()],
            input_cursor_row: 0,
            input_cursor_col: 0,
            terminal_width: width,
            terminal_height: height,
            streaming: StreamingUI::new(),
            task_list: None,
            footer_model: None,
            prompt_hint: None,
            search_modal: crate::ui::ui_modal_search::SearchModal::new(),
            model_picker: crate::ui::ui_model_picker::ModelPicker::new(),
            autocomplete_state: None,
            picker_state: PickerState::None,
            input_mode: InputMode::Chat,
            file_matches: Vec::new(),
            transcript_mode: TranscriptMode::Normal,
            modal_state: None,
            scrollbar_state: ScrollbarState::default(),
            anim_frame: 0,
            last_content_area: None,
            last_start_line: 0,
            last_line_mapping: Vec::new(),
        }
    }

    pub(crate) fn push_message(&mut self, msg: ClaudeMessage) {
        self.transcript.push(msg);
    }

    pub(crate) fn clear_transcript(&mut self) {
        self.transcript.messages.clear();
    }

    pub(crate) fn set_input(&mut self, lines: Vec<String>) {
        self.input_lines = lines;
    }

    pub(crate) fn set_input_cursor(&mut self, row: usize, col: usize) {
        self.input_cursor_row = row;
        self.input_cursor_col = col;
    }

    pub(crate) fn toggle_transcript(&mut self) {
        self.transcript.expanded = !self.transcript.expanded;
    }

    pub(crate) fn set_task_list(&mut self, tasks: crate::claude_ui::claude_tasks::TaskList) {
        self.task_list = Some(tasks);
    }

    pub(crate) fn set_task_lines(&mut self, _lines: Vec<String>) {
        // Kept for backward compatibility; use set_task_list instead.
    }

    pub(crate) fn set_footer_model(&mut self, model: Option<FooterModel>) {
        self.footer_model = model;
    }

    pub(crate) fn set_prompt_hint(&mut self, hint: Option<UiNotice>) {
        self.prompt_hint = hint;
    }

    fn clear_expired_prompt_hint(&mut self) {
        let expired = self.prompt_hint.as_ref().is_some_and(|hint| {
            hint.persistence == NoticePersistence::EphemeralPromptHint
                && Instant::now().duration_since(hint.created_at)
                    > std::time::Duration::from_millis(1200)
        });
        if expired {
            self.prompt_hint = None;
        }
    }

    fn prompt_hint_text(&self) -> Option<String> {
        self.prompt_hint
            .as_ref()
            .map(|hint| format!("◦ {}", hint.content))
    }

    fn footer_streaming_state(&self) -> Option<String> {
        if self.streaming.is_streaming_thinking {
            Some("∴ Thinking...".to_string())
        } else if self.streaming.is_streaming_content {
            Some("…".to_string())
        } else {
            None
        }
    }

    pub(crate) fn scroll_up(&mut self, lines: usize) {
        self.transcript.scroll_up(lines);
    }

    pub(crate) fn scroll_down(&mut self, lines: usize) {
        self.transcript.scroll_down(lines);
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.transcript.scroll_to_bottom();
    }

    pub(crate) fn set_transcript_expanded(&mut self, expanded: bool) {
        self.transcript.expanded = expanded;
    }

    pub(crate) fn transcript_message_count(&self) -> usize {
        self.transcript.messages.len()
    }

    pub(crate) fn set_modal(&mut self, modal: Option<crate::ui_state::ModalState>) {
        self.modal_state = modal;
    }

    pub(crate) fn clear_modal(&mut self) {
        self.modal_state = None;
    }

    pub(crate) fn show_search(&mut self) {
        self.search_modal.show();
    }

    pub(crate) fn hide_search(&mut self) {
        self.search_modal.hide();
    }

    pub(crate) fn update_search_query(&mut self, query: String) {
        self.search_modal.update_query(query);
    }

    pub(crate) fn search_select_next(&mut self) {
        self.search_modal.select_next();
    }

    pub(crate) fn search_select_prev(&mut self) {
        self.search_modal.select_prev();
    }

    pub(crate) fn set_autocomplete_state(
        &mut self,
        state: Option<&crate::ui::ui_autocomplete::AutocompleteState>,
    ) {
        self.autocomplete_state = state.cloned();
    }

    pub(crate) fn show_model_picker(&mut self) {
        self.model_picker.show();
    }

    pub(crate) fn hide_model_picker(&mut self) {
        self.model_picker.hide();
    }

    pub(crate) fn model_picker_select_next(&mut self) {
        self.model_picker.select_next();
    }

    pub(crate) fn model_picker_select_prev(&mut self) {
        self.model_picker.select_prev();
    }

    // --- Slash Picker / File Picker / Input Mode (Task 173) ---

    pub(crate) fn open_slash_picker(&mut self, query: String) {
        self.picker_state = PickerState::Slash { query, selected: 0 };
    }

    pub(crate) fn open_file_picker(&mut self, query: String, workdir: &PathBuf) {
        self.file_matches = discover_workspace_files(workdir, &query);
        self.picker_state = PickerState::File { query, selected: 0 };
    }

    pub(crate) fn close_picker(&mut self) {
        self.picker_state = PickerState::None;
    }

    pub(crate) fn picker_select_down(&mut self) {
        let max = match &self.picker_state {
            PickerState::Slash { .. } => {
                let filtered: Vec<_> = self.filtered_slash_commands();
                filtered.len()
            }
            PickerState::File { .. } => self.file_matches.len(),
            PickerState::None => 0,
        };
        self.picker_state.select_next(max);
    }

    pub(crate) fn picker_select_up(&mut self) {
        let max = match &self.picker_state {
            PickerState::Slash { .. } => {
                let filtered: Vec<_> = self.filtered_slash_commands();
                filtered.len()
            }
            PickerState::File { .. } => self.file_matches.len(),
            PickerState::None => 0,
        };
        self.picker_state.select_prev(max);
    }

    pub(crate) fn is_picker_active(&self) -> bool {
        self.picker_state.is_active()
    }

    pub(crate) fn selected_slash_command(&self) -> Option<&'static str> {
        if let PickerState::Slash { selected, .. } = self.picker_state {
            let filtered = self.filtered_slash_commands();
            filtered.get(selected).map(|c| c.name)
        } else {
            None
        }
    }

    pub(crate) fn selected_file(&self) -> Option<String> {
        if let PickerState::File { selected, .. } = self.picker_state {
            self.file_matches.get(selected).cloned()
        } else {
            None
        }
    }

    pub(crate) fn set_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
    }

    pub(crate) fn input_mode(&self) -> &InputMode {
        &self.input_mode
    }

    fn filtered_slash_commands(&self) -> Vec<&super::claude_input::SlashCommand> {
        match &self.picker_state {
            PickerState::Slash { query, .. } => {
                if query.is_empty() {
                    SLASH_COMMANDS.iter().collect()
                } else {
                    let q = query.to_lowercase();
                    SLASH_COMMANDS
                        .iter()
                        .filter(|c| c.name.contains(&q) || c.description.contains(&q))
                        .collect()
                }
            }
            _ => vec![],
        }
    }

    /// Primary event handler for Claude-style UI (Task 169)
    pub(crate) fn handle_event(&mut self, event: crate::claude_ui::UiEvent) {
        use crate::claude_ui::{ClaudeMessage, UiEvent};
        let show_reasoning = crate::ui_state::is_reasoning_visible();

        match event {
            UiEvent::TurnStarted => {
                // The transcript thinking row starts when real reasoning begins.
            }
            UiEvent::UserSubmitted(content) => {
                self.push_message(ClaudeMessage::User { content });
            }
            UiEvent::ThinkingStarted => {
                if show_reasoning {
                    self.start_thinking();
                }
            }
            UiEvent::ThinkingDelta(delta) => {
                if show_reasoning {
                    self.append_thinking(&delta);
                }
            }
            UiEvent::ThinkingFinished => {
                if show_reasoning {
                    self.finish_thinking();
                }
            }
            UiEvent::AssistantContentDelta(delta) => {
                self.start_content(); // Ensure content mode is active
                self.append_content(&delta);
            }
            UiEvent::AssistantFinished => {
                self.finish_content();
            }
            UiEvent::ToolStarted { name, command } => {
                self.push_message(ClaudeMessage::ToolTrace {
                    name,
                    command,
                    status: crate::claude_ui::claude_state::ToolTraceStatus::Running,
                    collapsed: false,
                });
            }
            UiEvent::ToolProgress {
                name: _,
                message: _,
            } => {
                // Tool progress is now implicit — the trace shows "Running" state.
                // If we wanted granular progress, we could store it in the trace.
            }
            UiEvent::ToolFinished {
                name,
                success,
                output,
            } => {
                self.transcript.update_last_tool_trace(
                    &name,
                    crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                        success,
                        output,
                        duration_ms: None,
                    },
                );
            }
            UiEvent::PermissionRequested { command } => {
                self.push_message(ClaudeMessage::PermissionRequest {
                    command,
                    reason: None,
                });
            }
            UiEvent::CompactBoundary => {
                self.push_message(ClaudeMessage::CompactBoundary);
            }
            UiEvent::Resize { cols, rows } => {
                self.terminal_width = cols;
                self.terminal_height = rows;
            }
            UiEvent::StatusUpdated {
                model: _,
                ctx_tokens: _,
            } => {
                // Update status in renderer state if needed
            }
            _ => {
                // Other events handled by higher layers or ignored for now
            }
        }
    }

    // --- Streaming API ---
    pub(crate) fn start_thinking(&mut self) {
        self.streaming.start_thinking();
        self.transcript.start_live_thinking();
    }

    pub(crate) fn append_thinking(&mut self, text: &str) {
        self.streaming.append_thinking(text);
        self.transcript.append_live_thinking(text);
    }

    pub(crate) fn finish_thinking(&mut self) {
        self.streaming.finish_thinking();
        self.transcript.finish_live_thinking();
        self.streaming.thinking.clear();
    }

    pub(crate) fn start_content(&mut self) {
        self.streaming.start_content();
    }

    pub(crate) fn append_content(&mut self, text: &str) {
        self.streaming.append_content(text);
    }

    pub(crate) fn finish_content(&mut self) {
        self.streaming.finish_content();

        if !self.streaming.content.is_empty() {
            self.transcript.push(ClaudeMessage::Assistant {
                content: AssistantContent::from_markdown(&self.streaming.content),
            });
        }

        // Clear streaming state
        self.streaming.content.clear();
        self.streaming.is_streaming_thinking = false;
        self.streaming.is_streaming_content = false;
    }

    pub(crate) fn is_streaming(&self) -> bool {
        self.streaming.is_streaming_thinking || self.streaming.is_streaming_content
    }

    pub(crate) fn next_redraw_deadline(&self) -> Option<Instant> {
        self.transcript.thinking_redraw_deadline()
    }

    pub(crate) fn last_assistant_message(&self) -> Option<&String> {
        self.transcript.messages.iter().rev().find_map(|m| {
            if let ClaudeMessage::Assistant { content } = m {
                Some(&content.raw_markdown)
            } else {
                None
            }
        })
    }

    /// Render modal overlay in Claude style
    fn render_modal_claude(&self, f: &mut Frame, modal: &crate::ui_state::ModalState, area: Rect) {
        let theme = current_theme();
        let modal_width = (area.width * 3 / 4).min(60);
        let modal_height = (area.height * 2 / 3).min(20);
        let modal_x = (area.width - modal_width) / 2;
        let modal_y = (area.height - modal_height) / 2;
        let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

        f.render_widget(ratatui::widgets::Clear, modal_area);

        let (title, content) = match modal {
            crate::ui_state::ModalState::Confirm { title, message } => {
                (title.clone(), message.clone())
            }
            crate::ui_state::ModalState::Help { content } => ("Help".to_string(), content.clone()),
            crate::ui_state::ModalState::Select { title, options } => {
                (title.clone(), options.join("\n"))
            }
            crate::ui_state::ModalState::Settings { content } => {
                ("Settings".to_string(), content.clone())
            }
            crate::ui_state::ModalState::Usage { content } => {
                ("Usage".to_string(), content.clone())
            }
            crate::ui_state::ModalState::ToolApproval {
                tool_name,
                description,
                selected,
            } => {
                let options = vec!["Yes", "Always", "No"];
                let mut text = format!("Tool: {}\n{}", tool_name, description);
                for (i, opt) in options.iter().enumerate() {
                    let marker = if i == *selected { "▸ " } else { "  " };
                    text.push_str(&format!("\n{}{}", marker, opt));
                }
                ("Tool Approval".to_string(), text)
            }
            crate::ui_state::ModalState::PermissionGate {
                command,
                risk_level,
                selected,
            } => {
                let options = vec!["Yes", "Always", "No"];
                let mut text = format!("Command: {}\nRisk: {}", command, risk_level);
                for (i, opt) in options.iter().enumerate() {
                    let marker = if i == *selected { "▸ " } else { "  " };
                    text.push_str(&format!("\n{}{}", marker, opt));
                }
                ("Permission Required".to_string(), text)
            }
            crate::ui_state::ModalState::PlanProgress {
                title,
                current,
                total,
                steps,
            } => {
                let mut text = format!("Step {}/{}", current, total);
                for step in steps {
                    text.push_str(&format!("\n  {}", step));
                }
                (title.clone(), text)
            }
            crate::ui_state::ModalState::Notification { message, level } => {
                (format!("Notification ({})", level), message.clone())
            }
            crate::ui_state::ModalState::Splash { content } => {
                ("Elma".to_string(), content.clone())
            }
        };

        let block = ratatui::widgets::Block::default()
            .title(title)
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(Style::default().fg(theme.accent_primary.to_ratatui_color()));

        let text = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false });

        f.render_widget(text, modal_area);
    }

    /// Render using Ratatui
    pub(crate) fn render_ratatui(&mut self, f: &mut Frame) {
        let theme = current_theme();
        self.anim_frame = self.anim_frame.wrapping_add(1);
        let area = f.size();

        // Always repaint the full frame background so stale rows from prior
        // frames or prior alternate-screen sessions cannot survive in
        // transcript/input/footer regions when the layout changes.
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            area,
        );

        let picker_height = match &self.picker_state {
            PickerState::Slash { .. } => {
                let filtered = self.filtered_slash_commands();
                filtered.len().min(8) as u16 + 2
            }
            PickerState::File { .. } => self.file_matches.len().min(8) as u16 + 2,
            PickerState::None => 0,
        };

        let task_height = match &self.task_list {
            Some(tl) if tl.visible && !tl.tasks.is_empty() => {
                let (_, hidden) = tl.visible_tasks_with_hidden();
                (tl.tasks.len().min(tl.max_visible) + hidden + 1) as u16
            }
            _ => 0,
        };

        // Claude Code-style compact layout:
        // 1. Transcript (scrollable, takes all available space)
        // 2. Task list (if visible)
        // 3. Picker (if active)
        // 4. Input (fixed at bottom)
        // 5. Footer (1 row)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(task_height),
                Constraint::Length(picker_height),
                Constraint::Length(self.input_lines.len() as u16),
                Constraint::Length(1),
            ])
            .split(area);

        let transcript_area = main_chunks[0];
        let task_area = main_chunks[1];
        let picker_area = main_chunks[2];
        let input_area = main_chunks[3];
        let footer_area = main_chunks[4];

        // Check if we need sticky header (scrolled up and have user messages)
        let show_sticky =
            self.transcript.scroll_offset > 0 && self.transcript.last_user_message().is_some();
        let sticky_height = if show_sticky { 1 } else { 0 };

        // Split transcript area into sticky header + scrollable content
        let transcript_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(sticky_height), Constraint::Min(0)])
            .split(transcript_area);

        // Render sticky header if visible
        if show_sticky {
            if let Some(prompt) = self.transcript.last_user_message() {
                let truncated = if prompt.len() > transcript_area.width as usize - 4 {
                    format!("{}…", &prompt[..transcript_area.width as usize - 5])
                } else {
                    prompt
                };
                let sticky_line = Line::from(vec![
                    Span::styled(
                        "❯ ",
                        Style::default()
                            .fg(theme.accent_primary.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        truncated,
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    ),
                ]);
                f.render_widget(
                    Paragraph::new(sticky_line)
                        .style(Style::default().bg(theme.border.to_ratatui_color())),
                    transcript_chunks[0],
                );
            }
        }

        let scrollable_area = transcript_chunks[1];

        // New messages pill (if scrolled up and new messages arrived)
        let unseen_count = self.transcript.count_unseen_assistant_turns();
        let show_pill = self.transcript.scroll_offset > 0 && unseen_count > 0;
        let pill_height = if show_pill { 1 } else { 0 };

        // Split scrollable area into content + pill
        let scrollable_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(pill_height)])
            .split(scrollable_area);

        let content_area = scrollable_chunks[0];
        let pill_area = scrollable_chunks[1];

        // Reserve a dedicated scrollbar column so transcript text never paints
        // underneath it. This avoids edge-glyph artifacts on long tool rows.
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(content_area);
        let text_area = content_chunks[0];
        let scrollbar_area = content_chunks[1];
        let height = text_area.height as usize;
        let content_width = content_area_width_guess(text_area.width as usize);

        let (transcript_lines, line_mapping) = self.transcript.render_ratatui(content_width);
        let total_lines = transcript_lines.len();

        // Manual line slicing: compute visible window based on scroll_offset
        // scroll_offset=0 means at bottom (latest), scroll_offset increases = scrolled up
        let max_offset = total_lines.saturating_sub(height);
        let start_line = if total_lines <= height {
            0
        } else if self.transcript.scroll_offset == 0 {
            max_offset
        } else {
            max_offset
                .saturating_sub(self.transcript.scroll_offset)
                .min(max_offset)
        };

        // Store hit-testing state for click-to-expand tool traces
        self.last_content_area = Some(content_area);
        self.last_start_line = start_line;
        self.last_line_mapping = line_mapping;

        // Slice visible lines from the transcript
        let visible_lines: Vec<Line<'static>> = transcript_lines
            .into_iter()
            .skip(start_line)
            .take(height)
            .collect();

        // Update scrollbar state
        self.scrollbar_state = ScrollbarState::new(total_lines)
            .position(start_line)
            .viewport_content_length(height);

        // Render transcript without ratatui re-wrapping. We already control the
        // row model; letting Paragraph wrap again can produce stray edge glyphs
        // and broken line mapping for long tool rows.
        let paragraph = Paragraph::new(visible_lines);

        f.render_widget(paragraph, text_area);

        // Render scrollbar on the right edge
        if total_lines > height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.fg_dim.to_ratatui_color()))
                .track_style(Style::default().fg(theme.border.to_ratatui_color()));
            f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scrollbar_state);
        }

        // Render new messages pill
        if show_pill {
            let pill_text = format!(" {} new messages ▼ ", unseen_count);
            let total_width = content_area.width as usize;
            let side = (total_width.saturating_sub(pill_text.len())) / 2;
            let pill_line_text = format!(
                "{}{}{}",
                "─".repeat(side),
                pill_text,
                "─".repeat(total_width.saturating_sub(side + pill_text.len()))
            );
            let pill_line = Line::from(Span::styled(
                pill_line_text,
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            ));
            f.render_widget(Paragraph::new(pill_line), pill_area);
        }

        // Task list (if visible)
        if let Some(tl) = &self.task_list {
            let task_lines = tl.render_ratatui();
            if !task_lines.is_empty() {
                f.render_widget(Paragraph::new(task_lines), task_area);
            }
        }

        // Input
        let mut input_content = Vec::new();
        for (i, line) in self.input_lines.iter().enumerate() {
            let prefix = if i == 0 {
                match self.input_mode {
                    InputMode::Bash => Span::styled(
                        "! ",
                        Style::default()
                            .fg(theme.accent_secondary.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    InputMode::Background => Span::styled(
                        "& ",
                        Style::default()
                            .fg(theme.warning.to_ratatui_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    _ => Span::styled(
                        "> ",
                        Style::default().fg(theme.accent_primary.to_ratatui_color()),
                    ),
                }
            } else {
                Span::raw("  ")
            };
            input_content.push(Line::from(vec![prefix, Span::raw(line.clone())]));
        }
        f.render_widget(Paragraph::new(input_content), input_area);

// Picker overlay (slash commands or file mentions)
         if picker_height > 0 {
             let mut picker_lines = match &self.picker_state {
                 PickerState::Slash { query, selected } => {
                     let filtered = self.filtered_slash_commands();
                     let mut lines = vec![Line::from(vec![
                         Span::styled(
                             " Commands ",
                             Style::default()
                                 .fg(theme.accent_primary.to_ratatui_color())
                                 .add_modifier(Modifier::BOLD),
                         ),
                         Span::raw(query),
                     ])];
                     lines.push(Line::from(""));
                     for (i, cmd) in filtered.iter().enumerate().take(8) {
                         let is_sel = i == *selected;
                         let arrow = if is_sel {
                             Span::styled(
                                 "▸ ",
                                 Style::default().fg(theme.accent_primary.to_ratatui_color()),
                             )
                         } else {
                             Span::raw("  ")
                         };
                         let name_style = if is_sel {
                             Style::default()
                                 .fg(theme.accent_primary.to_ratatui_color())
                                 .add_modifier(Modifier::BOLD)
                         } else {
                             Style::default().fg(theme.fg_dim.to_ratatui_color())
                         };
                         lines.push(Line::from(vec![
                             arrow,
                             Span::styled(cmd.name, name_style),
                             Span::raw("  "),
                             Span::styled(
                                 cmd.description,
                                 Style::default().fg(theme.fg_dim.to_ratatui_color()),
                             ),
                         ]));
                     }
                     lines
                 }
                 PickerState::File { query, selected } => {
                     let mut lines = vec![Line::from(vec![
                         Span::styled(
                             " Files ",
                             Style::default()
                                 .fg(theme.accent_secondary.to_ratatui_color())
                                 .add_modifier(Modifier::BOLD),
                         ),
                         Span::raw(query),
                     ])];
                     lines.push(Line::from(""));
                     for (i, path) in self.file_matches.iter().enumerate().take(8) {
                         let is_sel = i == *selected;
                         let arrow = if is_sel {
                             Span::styled(
                                 "▸ ",
                                 Style::default().fg(theme.accent_primary.to_ratatui_color()),
                             )
                         } else {
                             Span::raw("  ")
                         };
                         let path_style = if is_sel {
                             Style::default()
                                 .fg(theme.accent_primary.to_ratatui_color())
                                 .add_modifier(Modifier::BOLD)
                         } else {
                             Style::default().fg(theme.fg_dim.to_ratatui_color())
                         };
                         lines.push(Line::from(vec![arrow, Span::styled(path, path_style)]));
                     }
                     lines
                 }
                 PickerState::None => vec![],
             };
// Add autocomplete suggestions if active
             if let Some(state) = &self.autocomplete_state {
                 if state.active && !state.matches.is_empty() {
                     let max_width = picker_area.width as usize;
                     let max_items = (picker_height - picker_lines.len() as u16).min(10) as usize;
                     let autocomplete_lines = ui_autocomplete::render_autocomplete(
                         state,
                         max_width,
                         max_items,
                     );
                     picker_lines.extend(autocomplete_lines.into_iter().map(Line::from));
                 }
             }
             if !picker_lines.is_empty() {
                 f.render_widget(Clear, picker_area);
                 let picker_block = Paragraph::new(picker_lines).block(
                     ratatui::widgets::Block::default()
                         .borders(ratatui::widgets::Borders::ALL)
                         .border_style(Style::default().fg(theme.border.to_ratatui_color())),
                 );
                 f.render_widget(picker_block, picker_area);
             }
         }

        self.clear_expired_prompt_hint();

        if let Some(hint) = &self.prompt_hint {
            let line = Line::from(vec![
                Span::styled("◦ ", Style::default().fg(theme.fg_dim.to_ratatui_color())),
                Span::styled(
                    hint.content.clone(),
                    Style::default().fg(theme.fg_dim.to_ratatui_color()),
                ),
            ]);
            f.render_widget(Paragraph::new(line), footer_area);
        } else if let Some(model) = &self.footer_model {
            let line = render_footer_line(model, self.footer_streaming_state(), footer_area.width);
            f.render_widget(Paragraph::new(line), footer_area);
        } else {
            let hints: Vec<Span> = FOOTER_HINTS
                .iter()
                .map(|s| {
                    Span::styled(
                        format!("{}  ", s),
                        Style::default().fg(theme.fg_dim.to_ratatui_color()),
                    )
                })
                .collect();
            f.render_widget(Paragraph::new(Line::from(hints)), footer_area);
        }

        // Render modal if active (Claude-style absolute overlay)
        if let Some(ref modal) = self.modal_state {
            self.render_modal_claude(f, modal, area);
        }

        // Render search modal if visible
        self.search_modal.render(area, f);

        // Render model picker if visible
        self.model_picker.render(area, f);

        // Set cursor
        f.set_cursor(
            input_area.x + 2 + self.input_cursor_col as u16,
            input_area.y + self.input_cursor_row as u16,
        );
    }

    /// Legacy render method (still returns ClaudeScreen for backward compatibility)
    pub(crate) fn render(&self) -> ClaudeScreen {
        // This is now a "best effort" ANSI-string renderer for non-ratatui paths
        // Implementation omitted for brevity, or kept as-is from previous turn
        let mut lines = self.transcript.render();

        if self.streaming.is_streaming_content && !self.streaming.content.is_empty() {
            lines.push(format!("… {}", &self.streaming.content));
        }

        let streaming_hint = if self.streaming.is_streaming_thinking {
            Some(crate::ui_theme::dim("∴ Thinking..."))
        } else if self.streaming.is_streaming_content {
            Some(crate::ui_theme::dim("…"))
        } else {
            None
        };

        let input_height = self.input_lines.len();
        let footer_height = 1;
        let transcript_height = self
            .terminal_height
            .saturating_sub(input_height + footer_height);

        while lines.len() > transcript_height {
            lines.remove(0);
        }
        while lines.len() < transcript_height {
            lines.push(String::new());
        }

        for (i, line) in self.input_lines.iter().enumerate() {
            let prefix = if i == 0 {
                elma_accent("> ")
            } else {
                "  ".to_string()
            };
            lines.push(format!("{}{}", prefix, line));
        }

        if let Some(hint) = self.prompt_hint_text() {
            lines.push(hint);
        } else if let Some(model) = &self.footer_model {
            lines.push(render_footer_plain(model, self.footer_streaming_state()));
        } else if let Some(hint) = streaming_hint {
            lines.push(hint);
        } else {
            let hints_line: String = FOOTER_HINTS
                .iter()
                .map(|s| crate::ui_theme::meta_comment(s))
                .collect::<Vec<_>>()
                .join("  ");
            lines.push(hints_line);
        }

        let cursor_row = (transcript_height + self.input_cursor_row) as u16;
        let cursor_col = (2 + self.input_cursor_col) as u16;

        ClaudeScreen {
            lines,
            cursor_row,
            cursor_col,
        }
    }
}

fn content_area_width_guess(transcript_width: usize) -> usize {
    transcript_width.saturating_sub(1).max(12)
}

fn shorten_model_label(label: &str, max_chars: usize) -> String {
    let count = label.chars().count();
    if count <= max_chars {
        label.to_string()
    } else {
        let tail: String = label
            .chars()
            .rev()
            .take(max_chars.saturating_sub(1))
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("…{}", tail)
    }
}

fn render_footer_plain(model: &FooterModel, streaming_state: Option<String>) -> String {
    let mut parts = Vec::new();
    if let Some(ctx) = model.context_pct {
        let bar_width = 6usize;
        let filled = (ctx.min(100) * bar_width) / 100;
        parts.push(format!(
            "ctx {}% {}{}",
            ctx.min(100),
            "█".repeat(filled),
            "░".repeat(bar_width.saturating_sub(filled))
        ));
    }
    if let Some(label) = &model.model_label {
        parts.push(label.clone());
    }
    if let Some(tx) = &model.transcript_metric {
        parts.push(tx.clone());
    }
    if let Some(state) = streaming_state {
        parts.push(state);
    }
    parts.join("  ")
}

fn render_footer_line(
    model: &FooterModel,
    streaming_state: Option<String>,
    width: u16,
) -> Line<'static> {
    let theme = current_theme();
    let mut segments: Vec<(String, Style)> = Vec::new();

    if let Some(ctx) = model.context_pct {
        let bar_width = 6usize;
        let filled = (ctx.min(100) * bar_width) / 100;
        segments.push((
            format!(
                "ctx {}% {}{}",
                ctx.min(100),
                "█".repeat(filled),
                "░".repeat(bar_width.saturating_sub(filled))
            ),
            Style::default().fg(theme.fg_dim.to_ratatui_color()),
        ));
    }
    if let Some(label) = &model.model_label {
        segments.push((
            label.clone(),
            Style::default().fg(theme.fg_dim.to_ratatui_color()),
        ));
    }
    if let Some(tx) = &model.transcript_metric {
        segments.push((
            tx.clone(),
            Style::default().fg(theme.fg_dim.to_ratatui_color()),
        ));
    }
    if let Some(state) = streaming_state {
        segments.push((state, Style::default().fg(theme.fg_dim.to_ratatui_color())));
    }

    let mut text_len = footer_len(&segments);
    if text_len > width as usize && segments.len() >= 3 {
        segments.remove(2);
        text_len = footer_len(&segments);
    }
    if text_len > width as usize && segments.len() >= 3 {
        segments.pop();
        text_len = footer_len(&segments);
    }
    if text_len > width as usize && segments.len() >= 2 {
        let reserved_without_model = footer_len(&[
            segments[0].clone(),
            segments
                .last()
                .cloned()
                .unwrap_or_else(|| segments[1].clone()),
        ]);
        let available = width as usize;
        let budget = available.saturating_sub(reserved_without_model + 2).max(12);
        if let Some((model_text, _)) = segments.get_mut(1) {
            *model_text = shorten_model_label(model_text, budget);
        }
    }

    let mut spans = Vec::new();
    for (index, (text, style)) in segments.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(text, style));
    }
    Line::from(spans)
}

fn footer_len(segments: &[(String, Style)]) -> usize {
    segments
        .iter()
        .map(|(text, _)| text.chars().count())
        .sum::<usize>()
        + segments.len().saturating_sub(1) * 2
}

// ============================================================================
// Workspace File Discovery (Task 173 @ picker, Task 188 recursive discovery)
// ============================================================================

fn discover_workspace_files(workdir: &PathBuf, query: &str) -> Vec<String> {
    use ignore::WalkBuilder;

    let mut results = Vec::new();
    let canonical_workdir = match workdir.canonicalize() {
        Ok(p) => p,
        Err(_) => return results,
    };

    let mut walk_builder = WalkBuilder::new(&canonical_workdir);
    walk_builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .require_git(false)
        .max_depth(Some(10))
        .skip_stdout(true);

    for entry in walk_builder.build().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Ok(rel) = path.strip_prefix(&canonical_workdir) {
            let rel_str = rel.to_string_lossy().to_string();

            if rel_str.contains(query) {
                results.push(rel_str);
            }

            if results.len() >= 10000 {
                break;
            }
        }
    }

    results.sort();
    results.truncate(30);
    results
}

impl Default for ClaudeRenderer {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message_prefix() {
        let msg = ClaudeMessage::User {
            content: "hello world".to_string(),
        };
        let lines = msg.to_lines(false);
        assert!(lines[0].starts_with("> "));
    }

    #[test]
    fn test_assistant_message_prefix() {
        let msg = ClaudeMessage::Assistant {
            content: AssistantContent::from_markdown("Hello!"),
        };
        let lines = msg.to_lines(false);
        assert!(lines[0].starts_with("● "));
    }

    #[test]
    fn test_thinking_collapsed() {
        let msg = ClaudeMessage::Thinking {
            content: "Let me think...".to_string(),
            is_streaming: false,
            word_count: 3,
        };
        let lines = msg.to_lines(false);
        assert!(lines[0].contains("Thinking"));
    }

    #[test]
    fn test_thinking_expanded() {
        let msg = ClaudeMessage::Thinking {
            content: "Let me think...".to_string(),
            is_streaming: false,
            word_count: 3,
        };
        let lines = msg.to_lines(true);
        assert!(lines[0].contains("Thinking"));
        assert!(lines
            .last()
            .map(|l| l.contains("(ctrl+o to collapse)"))
            .unwrap_or(false));
    }

    #[test]
    fn test_tool_trace_success() {
        let msg = ClaudeMessage::ToolTrace {
            name: "shell".to_string(),
            command: "cat file.txt".to_string(),
            status: crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                success: true,
                output: "file content".to_string(),
                duration_ms: Some(1500),
            },
            collapsed: false,
        };
        let lines = msg.to_lines(false);
        assert!(lines[0].contains("✓"));
        assert!(lines[0].contains("shell"));
        assert!(lines[0].contains("cat file.txt"));
    }

    #[test]
    fn test_tool_trace_failure() {
        let msg = ClaudeMessage::ToolTrace {
            name: "shell".to_string(),
            command: "cat missing.txt".to_string(),
            status: crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                success: false,
                output: "error".to_string(),
                duration_ms: Some(100),
            },
            collapsed: false,
        };
        let lines = msg.to_lines(false);
        assert!(lines[0].contains("✗"));
        assert!(lines[0].contains("shell"));
        assert!(lines[0].contains("cat missing.txt"));
    }

    #[test]
    fn test_compact_boundary() {
        let msg = ClaudeMessage::CompactBoundary;
        // When not expanded, compact boundary is hidden
        let lines_hidden = msg.to_lines(false);
        assert!(
            lines_hidden.is_empty(),
            "CompactBoundary should be hidden when not expanded"
        );
        // When expanded, compact boundary is visible
        let lines_visible = msg.to_lines(true);
        assert!(
            !lines_visible.is_empty(),
            "CompactBoundary should be visible when expanded"
        );
        assert!(lines_visible[0].contains("compacted"));
    }

    #[test]
    fn test_finish_thinking_persists_and_clears_live_buffer() {
        let mut renderer = ClaudeRenderer::new(80, 24);
        renderer.start_thinking();
        renderer.append_thinking("step-by-step");
        renderer.finish_thinking();

        assert!(renderer.streaming.thinking.is_empty());
        assert!(matches!(
            renderer.transcript.messages.last(),
            Some(ClaudeMessage::Thinking { content, .. }) if content == "step-by-step"
        ));
    }

    #[test]
    fn test_thinking_streams_in_transcript_then_collapses() {
        use std::time::{Duration, Instant};

        let mut renderer = ClaudeRenderer::new(80, 24);
        renderer.start_thinking();
        renderer.append_thinking("live reasoning text");

        let live_lines = renderer.transcript.render();
        // During streaming, show the active thinking row plus the live text.
        assert!(
            live_lines.iter().any(|line| line.contains("∴"))
                && live_lines.iter().any(|line| line.contains("Thinking...")),
            "active thinking should show a visible thinking header"
        );
        assert!(
            live_lines
                .iter()
                .any(|line| line.contains("live reasoning text")),
            "active thinking should stream visible content"
        );

        renderer.finish_thinking();
        let held_lines = renderer.transcript.render();
        assert!(
            held_lines
                .iter()
                .any(|line| line.contains("live reasoning text")),
            "finished thinking should remain expanded briefly before collapsing"
        );

        renderer.transcript.thinking_collapse_deadline =
            Some((0, Instant::now() - Duration::from_secs(1)));
        let collapsed_lines = renderer.transcript.render();
        assert!(
            collapsed_lines.iter().any(|line| line.contains("Thinking")),
            "finished thinking should remain as a collapsed transcript row"
        );
        assert!(
            collapsed_lines
                .iter()
                .all(|line| !line.contains("live reasoning text")),
            "finished thinking should collapse by default"
        );
        // Collapsed state should show ">" prefix and time label
        assert!(
            collapsed_lines.iter().any(|line| line.contains(">")),
            "collapsed thinking should have > prefix"
        );
    }

    #[test]
    fn test_live_thinking_renders_in_expanded_layout_while_streaming() {
        let mut renderer = ClaudeRenderer::new(80, 24);
        renderer.start_thinking();
        renderer.append_thinking("first second third fourth fifth");

        let (lines, _) = renderer.transcript.render_ratatui(80);
        assert!(
            lines.len() >= 2,
            "live thinking should render expanded content"
        );
        assert!(fragments_contain(&lines[0], "Thinking..."));
        assert!(lines
            .iter()
            .any(|line| fragments_contain(line, "first second third fourth fifth")));
    }

    #[test]
    fn test_empty_thinking_step_is_not_kept_in_transcript() {
        let mut renderer = ClaudeRenderer::new(80, 24);
        renderer.start_thinking();
        renderer.finish_thinking();

        assert!(renderer.transcript.messages.is_empty());
    }

    #[test]
    fn test_renderer_basic() {
        let mut r = ClaudeRenderer::new(80, 24);
        r.push_message(ClaudeMessage::User {
            content: "test".to_string(),
        });
        let screen = r.render();
        assert!(!screen.lines.is_empty());
    }

    #[test]
    fn test_click_to_expand_thinking() {
        use std::time::Duration;
        let mut renderer = ClaudeRenderer::new(80, 24);

        // Add a thinking message
        renderer.start_thinking();
        renderer.append_thinking("step-by-step reasoning");
        renderer.finish_thinking();

        // Simulate deadline passed (collapsed state)
        renderer.transcript.thinking_collapse_deadline =
            Some((0, std::time::Instant::now() - Duration::from_secs(1)));

        // Verify it's collapsed (1 line, no content)
        let (lines, mapping) = renderer.transcript.render_ratatui(80);
        assert_eq!(lines.len(), 1, "collapsed thinking should be 1 line");
        assert!(
            lines.iter().any(|l| fragments_contain(l, "Thinking..")),
            "collapsed should have Thinking.."
        );
        assert!(
            !lines.iter().any(|l| fragments_contain(l, "step-by-step")),
            "collapsed should not have content"
        );

        // Click on the thinking row to expand it
        if let Some(&msg_idx) = mapping.get(0) {
            renderer.transcript.toggle_trace_collapse(msg_idx);
        }

        // Verify it's now expanded (multiple lines + ctrl+o hint)
        let (lines2, _) = renderer.transcript.render_ratatui(80);
        assert!(
            lines2.len() > 1,
            "expanded thinking should have multiple lines (got {})",
            lines2.len()
        );
        assert!(
            lines2.iter().any(|l| fragments_contain(l, "step-by-step")),
            "expanded thinking should show content"
        );
        assert!(
            lines2
                .iter()
                .any(|l| fragments_contain(l, "(ctrl+o to collapse)")),
            "expanded thinking should show collapse hint"
        );

        // Click again to collapse
        if let Some(&msg_idx) = mapping.get(0) {
            renderer.transcript.toggle_trace_collapse(msg_idx);
        }
        let (lines3, _) = renderer.transcript.render_ratatui(80);
        assert_eq!(
            lines3.len(),
            1,
            "clicking again should collapse (got {} lines)",
            lines3.len()
        );
    }

    #[test]
    fn test_footer_drops_transcript_metric_on_narrow_width() {
        let model = FooterModel {
            context_pct: Some(84),
            model_label: Some(
                "granite-4.0-h-micro-UD-Q8_K_XL.gguf/very/long/model/name".to_string(),
            ),
            transcript_metric: Some("tx 123456".to_string()),
        };
        let line = render_footer_line(&model, Some("∴ Thinking...".to_string()), 36);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.contains("ctx 84%"));
        assert!(!text.contains("tx 123456"));
    }

    #[test]
    fn test_footer_shortens_model_label() {
        let shortened = shorten_model_label("a/b/c/really-long-model-name.gguf", 12);
        assert!(shortened.starts_with('…'));
        assert!(shortened.len() <= 14);
    }

    #[test]
    fn test_footer_keeps_full_model_label_when_width_allows() {
        let model = FooterModel {
            context_pct: Some(40),
            model_label: Some("granite-4.0-h-micro-UD-Q8_K_XL.gguf".to_string()),
            transcript_metric: Some("tx 1024".to_string()),
        };
        let line = render_footer_line(&model, None, 120);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.contains("granite-4.0-h-micro-UD-Q8_K_XL.gguf"));
    }

    fn fragments_contain(line: &ratatui::text::Line, needle: &str) -> bool {
        line.spans.iter().any(|s| s.content.contains(needle))
    }
}
