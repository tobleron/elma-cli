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
use super::claude_state::{
    ClaudeMessage, ClaudeTranscript, NoticePersistence, UiNotice, FOOTER_HINTS,
};
use super::claude_stream::StreamingUI;
use crate::markdown_ansi::render_markdown_inline_to_ansi;
use crate::system_monitor;
use crate::ui_autocomplete;
use crate::ui_state::trace_log_state;
use crate::ui_theme::*;
use ratatui::prelude::*;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::*;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

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
    pub mode_label: Option<String>,
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
    pub(crate) status_thread: crate::ui_status_thread::StatusThread,
    // Animation frame counter for streaming indicators
    anim_frame: usize,
    // Hit-testing state for click-to-expand tool traces
    pub(crate) last_content_area: Option<ratatui::layout::Rect>,
    pub(crate) last_start_line: usize,
    pub(crate) last_line_mapping: Vec<usize>,
    // Hit-testing state for right panel thinking area (scroll + expand)
    pub(crate) last_thinking_area: Option<ratatui::layout::Rect>,
    // Hit-testing for scrollbar track in thinking area
    pub(crate) last_scrollbar_area: Option<ratatui::layout::Rect>,
    pub(crate) thinking_total_lines: usize,
    pub(crate) thinking_area_height: usize,
    // Thinking entries for right panel (collapsible, not expiring)
    thinking_entries: Vec<ThinkingEntry>,
    // Right panel thinking scroll
    pub(crate) thinking_scroll: usize,
    // Token count tracking for animated token counters
    input_token_count: usize,
    output_token_count: usize,
    // Latest budget/stop notice text for thinking panel footer
    last_notice_text: Option<String>,
}

/// A thinking/chain-of-thought entry for the right panel.
/// Collapses instead of disappearing after the delay.
#[derive(Clone, Debug)]
struct ThinkingEntry {
    content: String,
    word_count: usize,
    created_at: Instant,
    collapse_deadline: Instant,
    collapsed: bool,
    /// For summary entries: number of characters to reveal (streams in).
    reveal_chars: usize,
    /// True if this entry is a thought summary (not a raw thinking block).
    is_summary: bool,
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
            status_thread: crate::ui_status_thread::StatusThread::new(),
            anim_frame: 0,
            last_content_area: None,
            last_start_line: 0,
            last_line_mapping: Vec::new(),
            last_thinking_area: None,
            last_scrollbar_area: None,
            thinking_total_lines: 0,
            thinking_area_height: 0,
            thinking_entries: Vec::new(),
            thinking_scroll: 0,
            input_token_count: 0,
            output_token_count: 0,
            last_notice_text: None,
        }
    }

    pub(crate) fn push_message(&mut self, msg: ClaudeMessage) {
        // Capture last notice text and system messages for right panel thinking footer
        if let ClaudeMessage::Notice(ref notice) = msg {
            let kind_label = match notice.kind {
                crate::claude_ui::UiNoticeKind::Budget => "Budget",
                crate::claude_ui::UiNoticeKind::StopReason => "Stop",
                crate::claude_ui::UiNoticeKind::Compaction => "Compaction",
                crate::claude_ui::UiNoticeKind::Queue => "Queue",
                _ => "",
            };
            if !kind_label.is_empty() {
                self.last_notice_text = Some(format!("{}: {}", kind_label, notice.content));
            }
        }
        if let ClaudeMessage::System { ref content } = msg {
            self.last_notice_text = Some(content.clone());
        }
        let m = msg.clone();
        self.transcript.push(msg);

        // Append to session.md and terminal_transcript.txt if we can derive the session root
        if let Ok(guard) = trace_log_state().lock() {
            if let Some(ref trace_path) = *guard {
                if let Some(session_root) = trace_path.parent() {
                    // Generate transcript line before m is consumed by the match
                    let transcript_line = claude_message_to_transcript_line(&m);
                    use crate::session_write::{
                        append_session_markdown, append_terminal_transcript, MdEntry,
                    };
                    let entry = match &m {
                        ClaudeMessage::User { content } => MdEntry::User {
                            content: content.clone(),
                        },
                        ClaudeMessage::Assistant { content } => MdEntry::Assistant {
                            content: content.raw_markdown.clone(),
                        },
                        ClaudeMessage::Thinking { content, .. } => MdEntry::Thinking {
                            content: content.clone(),
                        },
                        ClaudeMessage::ToolStart { name, input } => MdEntry::ToolStart {
                            name: name.clone(),
                            input: format!("{:?}", input),
                        },
                        ClaudeMessage::ToolProgress { name, message } => MdEntry::ToolProgress {
                            name: name.clone(),
                            message: message.clone(),
                        },
                        ClaudeMessage::ToolResult {
                            name,
                            success,
                            output,
                            duration_ms,
                        } => MdEntry::ToolResult {
                            name: name.clone(),
                            success: *success,
                            output: output.clone(),
                            duration_ms: *duration_ms,
                        },
                        ClaudeMessage::ToolTrace {
                            name,
                            command,
                            status,
                            ..
                        } => match status {
                            crate::claude_ui::claude_state::ToolTraceStatus::Running => {
                                MdEntry::Meta {
                                    label: name.clone(),
                                    detail: format!("running: {}", command),
                                }
                            }
                            crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                                success,
                                output,
                                ..
                            } => MdEntry::ToolResult {
                                name: name.clone(),
                                success: *success,
                                output: output.clone(),
                                duration_ms: None,
                            },
                        },
                        ClaudeMessage::PermissionRequest { command, reason } => MdEntry::Meta {
                            label: "permission".into(),
                            detail: format!("{} reason={:?}", command, reason),
                        },
                        ClaudeMessage::CompactBoundary => MdEntry::Meta {
                            label: "compact".into(),
                            detail: String::new(),
                        },
                        ClaudeMessage::CompactSummary {
                            message_count,
                            context_preview,
                        } => MdEntry::Meta {
                            label: "compact".into(),
                            detail: format!(
                                "{} messages, preview={:?}",
                                message_count, context_preview
                            ),
                        },
                        ClaudeMessage::System { content } => MdEntry::Meta {
                            label: "system".into(),
                            detail: content.clone(),
                        },
                        ClaudeMessage::Notice(notice) => MdEntry::Meta {
                            label: "notice".into(),
                            detail: format!("{:?} {}", notice.kind, notice.content),
                        },
                    };
                    append_session_markdown(session_root, &entry);
                    append_terminal_transcript(session_root, &transcript_line);
                }
            }
        }
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
        if self.streaming.is_streaming_content {
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

    pub(crate) fn thinking_scroll_up(&mut self, lines: usize) {
        self.thinking_scroll = self.thinking_scroll.saturating_sub(lines);
    }

    pub(crate) fn thinking_scroll_down(&mut self, lines: usize) {
        self.thinking_scroll = self.thinking_scroll.saturating_add(lines);
    }

    pub(crate) fn toggle_thinking_entry(&mut self, mouse_y: u16, mouse_x: u16) {
        // First: check if click is on the scrollbar track
        if let Some(sb) = self.last_scrollbar_area {
            if mouse_y >= sb.y && mouse_y < sb.y + sb.height
                && mouse_x >= sb.x && mouse_x < sb.x + sb.width
            {
                // Scrollbar click: compute new scroll position from click Y
                if self.thinking_total_lines > self.thinking_area_height {
                    let max_scroll = self.thinking_total_lines - self.thinking_area_height;
                    let click_fraction = (mouse_y - sb.y) as f64 / sb.height as f64;
                    self.thinking_scroll = (click_fraction * max_scroll as f64).round() as usize;
                    if self.thinking_scroll > max_scroll {
                        self.thinking_scroll = max_scroll;
                    }
                }
                return;
            }
        }

        // Then: check if click is on a thinking entry (expand/collapse)
        let area = match self.last_thinking_area {
            Some(a) => a,
            None => return,
        };
        if mouse_y < area.y || mouse_y >= area.y + area.height {
            return;
        }
        if mouse_x < area.x || mouse_x >= area.x + area.width {
            return;
        }
        let relative_y = (mouse_y - area.y) as usize;
        let entry_idx = relative_y + self.thinking_scroll;
        if entry_idx < self.thinking_entries.len() {
            self.thinking_entries[entry_idx].collapsed =
                !self.thinking_entries[entry_idx].collapsed;
        }
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.transcript.scroll_to_bottom();
    }

    pub(crate) fn scroll_thinking_up(&mut self) {
        self.thinking_scroll = self.thinking_scroll.saturating_add(1);
    }

    pub(crate) fn scroll_thinking_down(&mut self) {
        self.thinking_scroll = self.thinking_scroll.saturating_sub(1);
    }

    pub(crate) fn set_thinking_scroll(&mut self, pos: usize) {
        self.thinking_scroll = pos;
    }

    pub(crate) fn set_token_counts(&mut self, input: usize, output: usize) {
        self.input_token_count = input;
        self.output_token_count = output;
    }

    pub(crate) fn update_input_tokens(&mut self, input: usize) {
        self.input_token_count = input;
    }

    pub(crate) fn add_output_tokens(&mut self, chars: usize) {
        self.output_token_count = self.output_token_count.saturating_add(chars / 4 + 1);
    }

    /// Increment output token count by delta chars (used during streaming)
    pub(crate) fn inc_output_tokens(&mut self, delta_chars: usize) {
        if delta_chars > 0 {
            self.output_token_count = self.output_token_count.saturating_add((delta_chars / 3).max(1));
        }
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
        let selected = match &self.picker_state {
            PickerState::Slash { selected, .. } => *selected,
            _ => 0,
        };
        self.picker_state = PickerState::Slash { query, selected };
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
                    // Exact matches first, then prefix matches on command name
                    // (strip leading / from name since query is content after /)
                    let mut exact: Vec<&super::claude_input::SlashCommand> = Vec::new();
                    let mut prefix: Vec<&super::claude_input::SlashCommand> = Vec::new();
                    for cmd in SLASH_COMMANDS.iter() {
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
                self.input_token_count = (content.len() / 2).max(1);
                self.output_token_count = 0;
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
                // Condense workspace_info for the in-memory trace display
                let trace_output = if name == "workspace_info" {
                    condense_workspace_info_for_transcript(&output)
                } else {
                    output.clone()
                };
                let truncated = terminal_tool_output_preview(&name, &trace_output);
                self.transcript.update_last_tool_trace(
                    &name,
                    crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                        success,
                        output: truncated,
                        duration_ms: None,
                    },
                );
                // Append completed trace to terminal transcript file
                // using transcript-safe truncation (not the raw output).
                if let Ok(guard) = trace_log_state().lock() {
                    if let Some(ref trace_path) = *guard {
                        if let Some(session_root) = trace_path.parent() {
                            let safe_output = transcript_safe_output(&name, &output);
                            let completed_msg = ClaudeMessage::ToolTrace {
                                name,
                                command: String::new(),
                                status:
                                    crate::claude_ui::claude_state::ToolTraceStatus::Completed {
                                        success,
                                        output: safe_output,
                                        duration_ms: None,
                                    },
                                collapsed: true,
                            };
                            crate::session_write::append_terminal_transcript(
                                session_root,
                                &claude_message_to_transcript_line(&completed_msg),
                            );
                        }
                    }
                }
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
        // Animate output token counter: roughly 1 token per 4 chars of thinking
        self.output_token_count = self.output_token_count.saturating_add(text.len() / 4 + 1);
    }

    pub(crate) fn finish_thinking(&mut self) {
        // Capture completed thinking for right panel (collapsible, never auto-removed)
        let thinking = self.streaming.thinking.clone();
        if !thinking.is_empty() {
            let word_count = thinking.split_whitespace().count();
            let delay_secs = 3.0;
            let now = Instant::now();
            self.thinking_entries.push(ThinkingEntry {
                content: thinking,
                word_count,
                created_at: now,
                collapse_deadline: now + std::time::Duration::from_secs_f64(delay_secs),
                collapsed: false,
                reveal_chars: 0,
                is_summary: false,
            });
            // Auto-collapse entries whose deadline has passed
            let now2 = Instant::now();
            for entry in &mut self.thinking_entries {
                if entry.collapse_deadline <= now2 {
                    entry.collapsed = true;
                }
            }
            // Trim old entries (keep last 50)
            if self.thinking_entries.len() > 50 {
                self.thinking_entries.remove(0);
            }
            // Auto-scroll to bottom so newest thought is visible
            self.thinking_scroll = usize::MAX;
        }
        self.streaming.finish_thinking();
        self.transcript.finish_live_thinking();
        self.streaming.thinking.clear();
    }

    pub(crate) fn start_content(&mut self) {
        self.streaming.start_content();
    }

    pub(crate) fn append_content(&mut self, text: &str) {
        self.streaming.append_content(text);
        // Animate output token counter: roughly 1 token per 4 chars of content
        self.output_token_count = self.output_token_count.saturating_add(text.len() / 4 + 1);
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

    /// Discard any accumulated streaming content without pushing it as a
    /// message. Used when a follow-up call (e.g., evidence finalizer) will
    /// produce the authoritative answer, making the model's raw streaming
    /// text redundant.
    pub(crate) fn discard_streaming_content(&mut self) {
        self.streaming.content.clear();
        self.streaming.is_streaming_content = false;
    }

    /// Replace the most recent thinking entry with a permanent summary that
    /// streams in gradually. The original thought disappears; the summary
    /// reveals character by character each frame.
    pub(crate) fn push_thought_summary(&mut self, summary: &str) {
        // Collapse all existing thoughts so only summary shows (thoughts expire at 3s)
        for entry in &mut self.thinking_entries {
            entry.collapsed = true;
        }
        let word_count = summary.split_whitespace().count();
        let now = Instant::now();
        let entry = ThinkingEntry {
            content: summary.to_string(),
            word_count,
            created_at: now,
            collapse_deadline: now + std::time::Duration::from_secs(18),
            collapsed: false,
            reveal_chars: 0,
            is_summary: true,
        };
        self.thinking_entries.push(entry);
        self.thinking_scroll = usize::MAX;
        // Auto-show reasoning panel so summary is visible
        if !crate::ui_state::is_reasoning_visible() {
            crate::ui_state::set_show_reasoning(true);
        }
    }

    pub(crate) fn is_streaming(&self) -> bool {
        self.streaming.is_streaming_thinking || self.streaming.is_streaming_content
    }

    pub(crate) fn next_redraw_deadline(&self) -> Option<Instant> {
        let transcript_deadline = self.transcript.thinking_redraw_deadline();

        // Also keep redrawing while any thinking entry is still revealing its text
        let any_revealing = self.thinking_entries.iter().any(|e| {
            e.reveal_chars < e.content.len()
        });

        if any_revealing {
            // Need to keep redrawing for smooth character reveal (~60fps)
            let reveal_deadline = Instant::now() + Duration::from_millis(16);
            match transcript_deadline {
                Some(t_deadline) => Some(t_deadline.min(reveal_deadline)),
                None => Some(reveal_deadline),
            }
        } else {
            transcript_deadline
        }
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

    pub(crate) fn replace_last_assistant_message(&mut self, content: AssistantContent) {
        self.transcript.replace_last_assistant_message(content);
    }

    pub(crate) fn remove_last_assistant_message(&mut self) {
        self.transcript.remove_last_assistant_message();
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
            crate::ui_state::ModalState::SessionPicker {
                entries,
                selected,
                filter,
                error,
            } => {
                let mut text = String::new();
                if let Some(err) = error {
                    text.push_str(&format!("⚠ {}\n\n", err));
                }
                if !filter.is_empty() {
                    text.push_str(&format!("filter: {}\n\n", filter));
                }
                if entries.is_empty() {
                    text.push_str("(no sessions)\n");
                    if !filter.is_empty() {
                        text.push_str("Clear filter with Backspace.\n");
                    }
                    text.push_str("N — New session  Esc — Back");
                } else {
                    let max_visible = (modal_height as usize).saturating_sub(4).min(entries.len());
                    let scroll_offset = if *selected >= max_visible {
                        selected.saturating_sub(max_visible.saturating_sub(1))
                    } else {
                        0
                    };
                    for i in scroll_offset..(scroll_offset + max_visible).min(entries.len()) {
                        let entry = &entries[i];
                        let marker = if i == *selected { "▸ " } else { "  " };
                        let curr = if entry.is_current { " ← current" } else { "" };
                        let warn = entry
                            .warning
                            .as_ref()
                            .map(|w| format!(" [{}]", w))
                            .unwrap_or_default();
                        let status_icon = match entry.status.as_str() {
                            "completed" => "✓",
                            "error" => "✗",
                            "interrupted" => "⊘",
                            _ => "●",
                        };
                        let age = format_relative_age(entry.last_modified_unix);
                        let model_suffix = entry
                            .model
                            .as_ref()
                            .map(|m| format!(" {}", m))
                            .unwrap_or_default();
                        let preview = if entry.preview.is_empty() {
                            String::new()
                        } else {
                            format!(" — {}", entry.preview)
                        };
                        text.push_str(&format!(
                            "{}{} {} {}{}{}{}{}\n",
                            marker,
                            status_icon,
                            &entry.id[..entry.id.len().min(20)],
                            age,
                            model_suffix,
                            curr,
                            warn,
                            preview
                        ));
                    }
                    if entries.len() > max_visible {
                        text.push_str(&format!(
                            "  … {} more (scroll with PgUp/PgDn)\n",
                            entries.len() - max_visible
                        ));
                    }
                    text.push_str("\nEnter resume  N new  R refresh  Esc");
                }
                ("Sessions".to_string(), text)
            }
        };

        let block = ratatui::widgets::Block::default()
            .title(title)
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(Style::default().fg(theme.fg_dim.to_ratatui_color()));

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
            Block::default().style(Style::default().bg(theme.bg.to_ratatui_color())),
            area,
        );

        // Horizontal gutter (2 columns each side) so content doesn't touch edges
        let gutter = 2u16;
        let content_area = ratatui::layout::Rect {
            x: area.x + gutter,
            y: area.y,
            width: area.width.saturating_sub(gutter * 2),
            height: area.height,
        };

        // Split horizontally: main content (left) + info panel (right)
        // Left = generous square: at least 60% of width, capped at terminal
        // height as the "square" side. Right = remainder, min 18 cols.
        let square_side = area.height;
        let min_main_width = content_area.width * 3 / 5;
        let max_main_width = content_area.width.saturating_sub(18);
        let main_width = if content_area.width >= 60 {
            square_side.min(max_main_width).max(min_main_width)
        } else {
            max_main_width
        };
        let panel_width = content_area.width.saturating_sub(main_width).max(18);
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(main_width), Constraint::Length(panel_width)])
            .split(content_area);
        let main_area = h_chunks[0];
        let panel_area = h_chunks[1];

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
        // 4. Input (fixed at bottom, dynamically sized based on wrapped lines)
        // 5. Footer (3 rows: margin top, content, margin bottom)
        let input_display_width = main_area.width.saturating_sub(6) as usize;
        let wrapped_input = wrap_input_lines(&self.input_lines, input_display_width);
        let input_height = wrapped_input.len().min(10) as u16;
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(task_height),
                Constraint::Length(picker_height),
                Constraint::Length(input_height),
                Constraint::Length(3),
            ])
            .split(main_area);

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
                    let byte_pos = transcript_area.width as usize - 5;
                    let end = if prompt.is_char_boundary(byte_pos) {
                        byte_pos
                    } else {
                        let mut pos = byte_pos;
                        while pos > 0 && !prompt.is_char_boundary(pos) {
                            pos -= 1;
                        }
                        pos
                    };
                    format!("{}…", &prompt[..end])
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

        // Reserve a dedicated scrollbar column + right margin so transcript text
        // never paints underneath the scrollbar or its edge.
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(2), Constraint::Length(1)])
            .split(content_area);
        let text_area = content_chunks[0];
        let _right_margin = content_chunks[1];
        let scrollbar_area = content_chunks[2];
        let height = text_area.height as usize;
        let content_width = content_area_width_guess(text_area.width as usize);

        let (transcript_lines, line_mapping) = self.transcript.render_ratatui(content_width);

        let mut all_lines = transcript_lines;
        let mut all_mapping = line_mapping.clone();
        if let Some(status_line) = self.status_thread.render() {
            let status_span = Line::from(vec![Span::styled(
                status_line,
                Style::default().fg(theme.fg_dim.to_ratatui_color()),
            )]);
            all_lines.push(status_span);
            all_mapping.push(all_mapping.last().copied().unwrap_or(0));
        }
        let (all_lines, all_mapping) =
            wrap_lines_with_mapping(all_lines, all_mapping, text_area.width as usize);
        let total_lines = all_lines.len();

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
        self.last_line_mapping = all_mapping.clone();

        // Slice visible lines from the transcript
        let visible_lines: Vec<Line<'static>> = all_lines
            .into_iter()
            .skip(start_line)
            .take(height)
            .collect();

        // Update scrollbar state
        self.scrollbar_state = ScrollbarState::new(total_lines)
            .position(start_line)
            .viewport_content_length(height);

        // Lines are pre-wrapped before viewport slicing. Keeping the Paragraph
        // unwrapped makes scroll-to-bottom operate on actual terminal rows.
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
            let total_width = pill_area.width as usize;
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

        // Input with wrapping
        let mut input_content = Vec::new();
        let prefix_width = 2;
        let text_width = input_area.width.saturating_sub(prefix_width as u16) as usize;
        let display_wrapped = wrap_input_lines(&self.input_lines, text_width.max(10));
        for line in &display_wrapped {
            input_content.push(Line::from(vec![Span::raw(line.clone())]));
        }

        // Ghost text: show autocomplete completion inline when the cursor
        // is at the end of input (the common prefix-typing case).
        if let Some(ref autocomplete) = self.autocomplete_state {
            if autocomplete.active && !autocomplete.matches.is_empty() {
                let sel = autocomplete.selected.min(autocomplete.matches.len() - 1);
                let label = &autocomplete.matches[sel].label;
                let prefix = &autocomplete.prefix;
                if label.len() > prefix.len()
                    && label.starts_with(prefix.as_str())
                    && self.input_cursor_row + 1 == self.input_lines.len()
                    && self.input_cursor_col == self.input_lines[self.input_cursor_row].len()
                {
                    let ghost = &label[prefix.len()..];
                    if let Some(last) = input_content.last_mut() {
                        let mut spans = last.spans.clone();
                        spans.push(Span::styled(
                            ghost.to_string(),
                            Style::default().fg(theme.fg_dim.to_ratatui_color()),
                        ));
                        *last = Line::from(spans);
                    }
                }
            }
        }
        f.render_widget(Paragraph::new(input_content), input_area);

        // Picker overlay (slash commands or file mentions)
        if picker_height > 0 {
            let mut picker_lines = match &self.picker_state {
                PickerState::Slash { query: _, selected } => {
                    let filtered = self.filtered_slash_commands();
                    let mut lines = Vec::new();
                    for (i, cmd) in filtered.iter().enumerate().take(8) {
                        let is_sel = i == *selected;
                        let arrow = if is_sel {
                            Span::styled(
                                "▸ ",
                                Style::default().fg(theme.fg_dim.to_ratatui_color()),
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
                                Style::default().fg(theme.fg_dim.to_ratatui_color()),
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
            // Add autocomplete suggestions if active (skip when slash picker is active
            // since the picker already provides slash command selection)
            if let Some(state) = &self.autocomplete_state {
                let is_slash_picker = matches!(self.picker_state, PickerState::Slash { .. });
                if state.active && !state.matches.is_empty() && !is_slash_picker {
                    let max_width = picker_area.width as usize;
                    let max_items = (picker_height - picker_lines.len() as u16).min(10) as usize;
                    let autocomplete_lines =
                        ui_autocomplete::render_autocomplete(state, max_width, max_items);
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

        // Footer is 3 rows: margin-top, content, margin-bottom
        let footer_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(footer_area);
        let footer_content = footer_chunks[1];

        if let Some(hint) = &self.prompt_hint {
            let line = Line::from(vec![
                Span::styled(
                    "◦ ",
                    Style::default()
                        .fg(theme.fg_dim.to_ratatui_color())
                        .bg(theme.bg_footer.to_ratatui_color()),
                ),
                Span::styled(
                    hint.content.clone(),
                    Style::default()
                        .fg(theme.fg_dim.to_ratatui_color())
                        .bg(theme.bg_footer.to_ratatui_color()),
                ),
            ]);
            f.render_widget(
                Paragraph::new(line).style(Style::default().bg(theme.bg_footer.to_ratatui_color())),
                footer_content,
            );
        } else if let Some(model) = &self.footer_model {
            let line = render_footer_line(model, self.footer_streaming_state(), footer_content.width);
            f.render_widget(Paragraph::new(line), footer_content);
        } else {
            let hints: Vec<Span> = FOOTER_HINTS
                .iter()
                .map(|s| {
                    Span::styled(
                        format!("{}  ", s),
                        Style::default()
                            .fg(theme.fg_dim.to_ratatui_color())
                            .bg(theme.bg_footer.to_ratatui_color()),
                    )
                })
                .collect();
            f.render_widget(
                Paragraph::new(Line::from(hints))
                    .style(Style::default().bg(theme.bg_footer.to_ratatui_color())),
                footer_content,
            );
        }

        // Render right-side info panel with thinking threads
        if panel_width > 0 {
            // Auto-collapse entries whose deadline has passed, and
            // increment reveal for streaming summary entries
            let now = Instant::now();
            for entry in &mut self.thinking_entries {
                if entry.collapse_deadline <= now {
                    entry.collapsed = true;
                }
                // Stream summary text in gradually (~8 chars per frame)
                if entry.reveal_chars < entry.content.len() {
                    entry.reveal_chars = (entry.reveal_chars + 8).min(entry.content.len());
                }
            }

            // Remove expired entries (deadline passed)
            let now = Instant::now();
            self.thinking_entries.retain(|e| now < e.collapse_deadline);

            // Collect visible thinking: oldest first (chronological)
            let all_thinking: Vec<&ThinkingEntry> = self
                .thinking_entries
                .iter()
                .collect();

            // Split panel into top info section + bottom thinking section
            let show_reasoning = crate::ui_state::is_reasoning_visible();
            let panel_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(if show_reasoning {
                    vec![
                        Constraint::Length(13),  // info area (compact logo + stats + model)
                        Constraint::Min(4),      // thinking area (fills remaining space)
                    ]
                } else {
                    vec![
                        Constraint::Min(0),    // info fills entire panel
                    ]
                })
                .split(panel_area);
            let info_area = panel_chunks[0];
            let thinking_area = if show_reasoning { panel_chunks[1] } else { Rect::default() };

            let is_processing = self.streaming.is_streaming_thinking || self.streaming.is_streaming_content;
            render_right_panel_info(
                info_area,
                f,
                &self.footer_model,
                self.anim_frame,
                self.input_token_count,
                self.output_token_count,
                is_processing,
            );

            // Render thinking section only if ctrl+t (reasoning) is active
            let show_reasoning = crate::ui_state::is_reasoning_visible();
            if show_reasoning {
                self.last_thinking_area = Some(thinking_area);
                render_right_panel_thinking(
                    thinking_area,
                    f,
                    &all_thinking,
                    self.anim_frame,
                    &mut self.thinking_scroll,
                    self.streaming.is_streaming_thinking,
                    &self.streaming.thinking,
                    self.last_notice_text.as_deref(),
                    &mut self.last_scrollbar_area,
                    &mut self.thinking_total_lines,
                    &mut self.thinking_area_height,
                );
            } else {
                self.last_thinking_area = None;
            }
        }

        // Render modal if active (Claude-style absolute overlay)
        if let Some(ref modal) = self.modal_state {
            self.render_modal_claude(f, modal, area);
        }

        // Render search modal if visible
        self.search_modal.render(area, f);

        // Render model picker if visible
        self.model_picker.render(area, f);

        // Compute cursor position in the wrapped display
        let (cursor_display_row, cursor_display_col) = cursor_in_wrapped(
            &self.input_lines,
            self.input_cursor_row,
            self.input_cursor_col,
            text_width.max(10),
        );
        // Set cursor
        f.set_cursor(
            input_area.x + cursor_display_col as u16,
            input_area.y + cursor_display_row as u16,
        );
    }

    /// Legacy render method (still returns ClaudeScreen for backward compatibility)
    pub(crate) fn render(&self) -> ClaudeScreen {
        // This is now a "best effort" ANSI-string renderer for non-ratatui paths
        // Implementation omitted for brevity, or kept as-is from previous turn
        let mut lines = self.transcript.render();

        if self.streaming.is_streaming_content && !self.streaming.content.is_empty() {
            let rendered = render_markdown_inline_to_ansi(&self.streaming.content);
            for line in rendered.lines() {
                lines.push(format!("  {}", line));
            }
        }

        let streaming_hint = if self.streaming.is_streaming_content {
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
        let display_col = if self.input_cursor_row < self.input_lines.len() {
            let line = &self.input_lines[self.input_cursor_row];
            let byte_pos = self.input_cursor_col.min(line.len());
            str_display_width(&line[..byte_pos])
        } else {
            self.input_cursor_col
        };
        let cursor_col = (2 + display_col) as u16;

        ClaudeScreen {
            lines,
            cursor_row,
            cursor_col,
        }
    }
}

fn content_area_width_guess(transcript_width: usize) -> usize {
    transcript_width.saturating_sub(8).max(12)
}

fn wrap_lines_with_mapping(
    lines: Vec<Line<'static>>,
    mapping: Vec<usize>,
    width: usize,
) -> (Vec<Line<'static>>, Vec<usize>) {
    let width = width.max(1);
    let mut wrapped_lines = Vec::new();
    let mut wrapped_mapping = Vec::new();

    for (index, line) in lines.into_iter().enumerate() {
        let mapped_index = mapping
            .get(index)
            .copied()
            .unwrap_or_else(|| mapping.last().copied().unwrap_or(0));

        // Batch consecutive characters with the same style together
        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut current_width = 0usize;
        let mut batch: Vec<(char, Style)> = Vec::new();

        for span in line.spans {
            let style = span.style;
            for ch in span.content.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if current_width > 0 && current_width + ch_width > width {
                    // Flush batch
                    if !batch.is_empty() {
                        let text: String = batch.iter().map(|(c, _)| c).collect();
                        let batch_style = batch[0].1;
                        current_spans.push(Span::styled(text, batch_style));
                        batch.clear();
                    }
                    wrapped_lines.push(Line::from(std::mem::take(&mut current_spans)));
                    wrapped_mapping.push(mapped_index);
                    current_width = 0;
                }
                // Batch same-style consecutive chars
                if batch.is_empty() || batch.last().map(|(_, s)| s == &style).unwrap_or(false) {
                    batch.push((ch, style));
                } else {
                    let text: String = batch.iter().map(|(c, _)| c).collect();
                    let batch_style = batch[0].1;
                    current_spans.push(Span::styled(text, batch_style));
                    batch.clear();
                    batch.push((ch, style));
                }
                current_width += ch_width;
            }
        }

        // Flush remaining batch
        if !batch.is_empty() {
            let text: String = batch.iter().map(|(c, _)| c).collect();
            let batch_style = batch[0].1;
            current_spans.push(Span::styled(text, batch_style));
            batch.clear();
        }

        if current_spans.is_empty() {
            wrapped_lines.push(Line::default());
        } else {
            wrapped_lines.push(Line::from(current_spans));
        }
        wrapped_mapping.push(mapped_index);
    }

    (wrapped_lines, wrapped_mapping)
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

/// Wrap input lines to fit within `display_width` columns, breaking at word
/// boundaries. Returns lines with "> " or "  " prefix already included.
fn wrap_input_lines(input_lines: &[String], display_width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let prefix_width = 2; // "> " or "  "
    let text_width = display_width.saturating_sub(prefix_width);

    for (i, line) in input_lines.iter().enumerate() {
        let prefix = if i == 0 { "> " } else { "  " };
        if line.is_empty() {
            result.push(prefix.to_string());
            continue;
        }

        let mut remaining = line.as_str();
        let mut first = true;
        loop {
            let current_prefix = if first { prefix } else { "  " };
            let remaining_width = str_display_width(remaining);
            if remaining_width <= text_width {
                result.push(format!("{}{}", current_prefix, remaining));
                break;
            }

            // Find a good break point (word boundary)
            let mut split_at = text_width;
            let mut char_count = 0;
            let mut last_ws = None;
            for (ci, c) in remaining.char_indices() {
                let char_width = char_display_width(c);
                if char_count + char_width > text_width {
                    break;
                }
                char_count += char_width;
                split_at = ci + c.len_utf8();
                if c == ' ' || c == '\t' {
                    last_ws = Some(ci);
                }
            }

            // Prefer word boundary if we found one past the midpoint
            if let Some(ws) = last_ws {
                if ws > text_width / 2 {
                    split_at = ws;
                }
            }

            let chunk = &remaining[..split_at];
            let rest = remaining[split_at..].trim_start();
            result.push(format!("{}{}", current_prefix, chunk));
            remaining = rest;
            first = false;
        }
    }
    result
}

/// Map raw (cursor_row, cursor_col) in the input lines to (display_row, display_col)
/// in the wrapped output, accounting for "> " / "  " prefixes.
fn cursor_in_wrapped(
    input_lines: &[String],
    cursor_row: usize,
    cursor_col: usize,
    text_width: usize,
) -> (usize, usize) {
    let prefix_width = 2;
    let mut display_row = 0usize;

    for (li, line) in input_lines.iter().enumerate() {
        if li == cursor_row {
            let prefix = if li == 0 { "> " } else { "  " };
            let prefix_chars: usize = prefix.chars().map(|c| char_display_width(c)).sum();

            let before_cursor = if cursor_col == 0 {
                ""
            } else if cursor_col < line.len() {
                &line[..cursor_col]
            } else {
                line
            };
            let before_width = str_display_width(before_cursor);

            let wrapped_before = before_width / text_width;
            display_row += wrapped_before;
            let col_in_row = before_width % text_width;

            return (display_row, prefix_chars + col_in_row);
        }

        if line.is_empty() {
            display_row += 1;
        } else {
            let line_width = str_display_width(line);
            display_row += 1.max((line_width + text_width - 1) / text_width.max(1));
        }
    }

    (display_row, prefix_width + cursor_col)
}

/// Display width of a single character in terminal columns.
fn char_display_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Display width of a string in terminal columns.
fn str_display_width(s: &str) -> usize {
    s.chars().map(char_display_width).sum()
}

/// Simple word-wrap: split `text` into lines no wider than `width` chars,
/// breaking at word boundaries when possible.
fn wrap_text_at_width(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut remaining = para;
        while remaining.chars().count() > width {
            let mut split_at = width;
            if let Some(pos) = remaining[..width].rfind(' ') {
                split_at = pos + 1;
            }
            if split_at == 0 {
                split_at = width;
            }
            let left = remaining[..split_at].trim_end().to_string();
            if !left.is_empty() {
                lines.push(left);
            }
            remaining = remaining[split_at..].trim_start();
        }
        if !remaining.is_empty() {
            lines.push(remaining.to_string());
        }
    }
    lines
}

fn render_right_panel_info(
    area: Rect,
    f: &mut Frame,
    footer_model: &Option<FooterModel>,
    anim_frame: usize,
    input_tokens: usize,
    output_tokens: usize,
    is_processing: bool,
) {
    let theme = current_theme();
    let dim = Style::default().fg(theme.fg_dim.to_ratatui_color());
    let accent = Style::default().fg(theme.accent_primary.to_ratatui_color());
    let secondary = Style::default().fg(theme.accent_secondary.to_ratatui_color());
    let pad = "  ";
    let text_width = area.width.saturating_sub(3) as usize;

    let mut all_lines: Vec<Line<'static>> = Vec::new();

    // ── Top: ELMA logo with alternating letter animation ──
    // Split logo into letter groups: E(0-2) L(3-5) M(6-8) A(9-11)
    let logo = r#"┏━╸╻  ┏┳┓┏━┓
┣╸ ┃  ┃┃┃┣━┫
┗━╸┗━╸╹ ╹╹ ╹
"#;
    let logo_lines: Vec<&str> = logo.lines().collect();
    let logo_width = logo_lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let logo_pad = if logo_width < text_width {
        (text_width - logo_width) / 2
    } else {
        0
    };
    let logo_pad_str: String = std::iter::repeat(' ').take(logo_pad).collect();

    // Animation: sequential one-at-a-time highlighting
    // Processing: ELMA cycles fast with 3s gap between cycles (90 frames)
    // Startup: quick burst
    let elma_highlight = if is_processing {
        // Fast cycle (5 frames per letter = 20 per cycle), wait 90 frames between cycles
        let phase = anim_frame % 110; // 20 frames cycle + 90 frames gap
        if phase < 20 { (phase / 5) % 4 } else { 5 }
    } else if anim_frame >= 2 && anim_frame < 10 {
        (anim_frame - 2) / 2
    } else {
        5
    };
    let letter_styles = [
        if elma_highlight == 0 { accent } else { dim },
        if elma_highlight == 1 { accent } else { dim },
        if elma_highlight == 2 { accent } else { dim },
        if elma_highlight == 3 { accent } else { dim },
    ];

    for row in &logo_lines {
        let mut spans = vec![Span::raw(logo_pad_str.clone())];
        let chars: Vec<char> = row.chars().collect();
        let groups = [
            chars.get(0..3).map(|c| c.iter().collect::<String>()).unwrap_or_default(),
            chars.get(3..6).map(|c| c.iter().collect::<String>()).unwrap_or_default(),
            chars.get(6..9).map(|c| c.iter().collect::<String>()).unwrap_or_default(),
            chars.get(9..12).map(|c| c.iter().collect::<String>()).unwrap_or_default(),
        ];
        for (gi, g) in groups.iter().enumerate() {
            if !g.is_empty() {
                spans.push(Span::styled(g.clone(), letter_styles[gi]));
            }
        }
        all_lines.push(Line::from(spans));
    }

    // Tagline (centered) in secondary color, cycles by character
    all_lines.push(Line::from(""));
    let tagline = "Local first terminal agent v0.1.0";
    let tagline_chars: Vec<String> = tagline.chars().map(|c| c.to_string()).collect();
    let tagline_pad = if tagline.chars().count() < text_width {
        (text_width - tagline.chars().count()) / 2
    } else {
        0
    };
    let tag_pad_str: String = std::iter::repeat(' ').take(tagline_pad).collect();

    // Tagline cycles through characters continuously, 3 frames per char
    let active_char = Some((anim_frame / 3) % tagline_chars.len());

    let mut tagline_spans = vec![Span::raw(tag_pad_str)];
    for (ci, ch) in tagline_chars.iter().enumerate() {
        let style = if active_char == Some(ci) { secondary } else { dim };
        tagline_spans.push(Span::styled(ch.clone(), style));
    }
    all_lines.push(Line::from(tagline_spans));

    // ── System info with thin progress bars ──
    all_lines.push(Line::from(""));
    if let Some(snap) = system_monitor::get_snapshot() {
        let bar_width = (text_width.saturating_sub(12)).max(8).min(30);
        all_lines.push(render_progress_bar_line(
            "CPU", snap.cpu_pct / 100.0, bar_width, anim_frame, theme, pad,
        ));
        all_lines.push(render_progress_bar_line(
            "MEM", snap.mem_pct / 100.0, bar_width, anim_frame.wrapping_add(3), theme, pad,
        ));
        all_lines.push(Line::from(vec![
            Span::styled(format!("{}COR {} cores", pad, snap.num_cpus), dim),
        ]));
    }

    // ── Token counters (animated, in complementary color) ──
    fn fmt_tokens(n: usize) -> String {
        if n >= 1000 { format!("{:.1}k", n as f64 / 1000.0) } else { n.to_string() }
    }
    all_lines.push(Line::from(""));
    all_lines.push(Line::from(vec![
        Span::raw(format!("{pad}")),
        Span::styled("↓ ", secondary),
        Span::styled(format!("in {}", fmt_tokens(input_tokens)), secondary),
        Span::raw(format!("  ")),
        Span::styled("↑ ", secondary),
        Span::styled(format!("out {}", fmt_tokens(output_tokens)), secondary),
    ]));

    // Model name (under token counter, just above thinking)
    if let Some(ref fm) = footer_model {
        if let Some(ref model) = fm.model_label {
            all_lines.push(Line::from(""));
            all_lines.push(Line::from(vec![Span::styled(
                truncate_to_width(&format!("{}{}", pad, model), text_width), dim,
            )]));
        }
    }

    let panel = Paragraph::new(all_lines)
        .style(Style::default().bg(theme.bg.to_ratatui_color()));

    f.render_widget(panel, area);
}

fn render_right_panel_thinking(
    area: Rect,
    f: &mut Frame,
    entries: &[&ThinkingEntry],
    anim_frame: usize,
    scroll: &mut usize,
    is_streaming: bool,
    live_text: &str,
    notice_text: Option<&str>,
    last_scrollbar_area: &mut Option<Rect>,
    thinking_total_lines: &mut usize,
    thinking_area_height: &mut usize,
) {
    let theme = current_theme();
    let dim = Style::default().fg(theme.fg_dim.to_ratatui_color());
    let accent = Style::default().fg(theme.accent_primary.to_ratatui_color());

    let mut all_lines: Vec<Line<'static>> = Vec::new();

    // Blank line for visual separation from model name
    all_lines.push(Line::from(""));

    // Separator header — shown only once at top
    all_lines.push(Line::from(vec![Span::styled(
        "── Thinking ──",
        Style::default().fg(theme.fg_dim.to_ratatui_color()),
    )]));

    // Completed thinking entries: oldest first, newest last
    for entry in entries {
        // Summary entries always show full text, never collapse
        if entry.is_summary {
            let max_w = (area.width.saturating_sub(4) as usize).max(10);
            // Only show characters up to reveal_chars (streaming effect)
            let visible: String = entry.content.chars().take(entry.reveal_chars).collect();
            let wrapped = wrap_text_at_width(&visible, max_w);
            for (li, wline) in wrapped.iter().enumerate() {
                let bullet = if li == 0 { "* " } else { "  " };
                all_lines.push(Line::from(vec![
                    Span::styled(bullet, accent),
                    Span::styled(wline.clone(), accent),
                ]));
            }
            continue;
        }

        if entry.collapsed {
            // Collapsed thought: grey (dim) until summary arrives
            let max_w = (area.width.saturating_sub(4) as usize).max(10);
            let first_line = entry.content.lines().next().unwrap_or(&entry.content);
            let wrapped = wrap_text_at_width(first_line, max_w);
            if let Some(wline) = wrapped.first() {
                all_lines.push(Line::from(vec![
                    Span::styled("* ", dim),
                    Span::styled(wline.clone(), dim),
                ]));
            }
        } else {
            // Active thought: grey (dim), show full content
            let max_w = (area.width.saturating_sub(4) as usize).max(10);
            let wrapped = wrap_text_at_width(&entry.content, max_w);
            for (li, wline) in wrapped.iter().enumerate() {
                let bullet = if li == 0 { "⌄ " } else { "  " };
                all_lines.push(Line::from(vec![
                    Span::styled(bullet, dim),
                    Span::styled(wline.clone(), dim),
                ]));
            }
        }
    }

    // Live streaming thinking — grey (dim) at bottom
    if is_streaming && !live_text.is_empty() {
        all_lines.push(Line::from(""));
        let max_w = (area.width.saturating_sub(4) as usize).max(10);
        let wrapped = wrap_text_at_width(live_text, max_w);
        for (li, wline) in wrapped.iter().enumerate() {
            let bullet = if li == 0 { "* " } else { "  " };
            all_lines.push(Line::from(vec![
                Span::styled(bullet, dim),
                Span::styled(wline.clone(), dim),
            ]));
        }
    }

    // System info at bottom of thinking section
    if let Some(notice) = notice_text {
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(vec![Span::styled(
            truncate_to_width(notice, area.width.saturating_sub(4) as usize),
            Style::default().fg(theme.fg_dim.to_ratatui_color()),
        )]));
    }

    let total_lines = all_lines.len();
    let area_height = area.height.saturating_sub(0) as usize;

    if total_lines > area_height {
        let max_scroll = total_lines.saturating_sub(area_height);
        // Auto-scroll to bottom when streaming
        if is_streaming {
            *scroll = max_scroll;
        } else if *scroll > max_scroll {
            *scroll = max_scroll;
        }
        let visible: Vec<Line<'static>> = all_lines
            .iter()
            .skip(*scroll)
            .take(area_height)
            .cloned()
            .collect();

        let thinking_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(2), Constraint::Length(2)])
            .split(area);
        let text_area = thinking_chunks[0];
        let scrollbar_track = thinking_chunks[1];
        let scrollbar_thumb = thinking_chunks[2];

        // Store for hit-testing: the full scrollbar track area (thumb + margin)
        *last_scrollbar_area = Some(Rect {
            x: scrollbar_track.x,
            y: scrollbar_track.y,
            width: scrollbar_thumb.x + scrollbar_thumb.width - scrollbar_track.x,
            height: scrollbar_track.height,
        });
        *thinking_total_lines = total_lines;
        *thinking_area_height = area_height;

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(*scroll)
            .viewport_content_length(area_height);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(theme.fg_dim.to_ratatui_color()))
            .track_style(Style::default().fg(theme.border.to_ratatui_color()));
        f.render_stateful_widget(scrollbar, scrollbar_thumb, &mut scrollbar_state);

        let panel = Paragraph::new(visible)
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(theme.border.to_ratatui_color()))
                    .padding(ratatui::widgets::Padding::new(0, 1, 0, 0)),
            )
            .style(Style::default().bg(theme.bg.to_ratatui_color()));
        f.render_widget(panel, text_area);
    } else {
        let panel = Paragraph::new(all_lines)
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(theme.border.to_ratatui_color()))
                    .padding(ratatui::widgets::Padding::new(0, 1, 0, 0)),
            )
            .style(Style::default().bg(theme.bg.to_ratatui_color()));
        f.render_widget(panel, area);
    }
}

/// Render a single thin animated progress bar line
fn render_progress_bar_line(
    label: &str,
    fraction: f64,
    bar_width: usize,
    anim_frame: usize,
    theme: &Theme,
    pad: &str,
) -> Line<'static> {
    let pct = (fraction.clamp(0.0, 1.0) * 100.0) as u8;
    let filled = (fraction.clamp(0.0, 1.0) * bar_width as f64).round() as usize;
    let dim = Style::default().fg(theme.fg_dim.to_ratatui_color());
    let accent = Style::default().fg(theme.accent_primary.to_ratatui_color());
    let secondary = Style::default().fg(theme.accent_secondary.to_ratatui_color());

    let bar_color = match label {
        "CPU" => secondary,
        "MEM" => secondary,
        _ => accent,
    };

    // Thin horizontal line: filled portion in color, empty portion dim
    let fill: String = "─".repeat(filled);
    let empty: String = "─".repeat(bar_width.saturating_sub(filled));

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(format!("{}{} ", pad, label), dim));
    spans.push(Span::styled(format!("{}% ", pct), accent));
    spans.push(Span::styled(fill, bar_color));
    spans.push(Span::styled(empty, Style::default().fg(theme.border.to_ratatui_color())));

    Line::from(spans)
}

/// Truncate a string to fit within a width, handling multi-byte characters correctly.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else {
        s.chars().take(max_width).collect()
    }
}

fn render_footer_plain(model: &FooterModel, streaming_state: Option<String>) -> String {
    let mut parts = Vec::new();
    if let Some(ctx) = model.context_pct {
        let pct = ctx.min(100) as f64 / 100.0;
        let bar_steps = 6usize;
        let total_sub_steps = bar_steps * 4;
        let filled_sub = (pct * total_sub_steps as f64).round() as usize;
        let full_blocks = filled_sub / 4;
        let remainder = filled_sub % 4;
        let mut bar = String::new();
        for i in 0..bar_steps {
            if i < full_blocks {
                bar.push('█');
            } else if i == full_blocks {
                bar.push(match remainder {
                    1 => '░',
                    2 => '▒',
                    3 => '▓',
                    _ => ' ',
                });
            } else {
                bar.push(' ');
            }
        }
        parts.push(format!("ctx {}% [{}]", ctx.min(100), bar));
    }
    if let Some(tx) = &model.transcript_metric {
        parts.push(tx.clone());
    }
    if let Some(state) = streaming_state {
        parts.push(state);
    }
    let mode = crate::ui_state::current_response_mode();
    let mode_str = match mode {
        crate::ui_state::ResponseMode::Concise => "Concise",
        crate::ui_state::ResponseMode::Long => "Long",
    };
    parts.push(mode_str.to_string());
    parts.join("  ")
}

fn render_footer_line(
    model: &FooterModel,
    streaming_state: Option<String>,
    width: u16,
) -> Line<'static> {
    let theme = current_theme();
    let footer_bg = Style::default().bg(theme.bg_footer.to_ratatui_color());
    let dim_style = Style::default()
        .fg(theme.fg_dim.to_ratatui_color())
        .bg(theme.bg_footer.to_ratatui_color());
    let accent_style = Style::default()
        .fg(theme.accent_primary.to_ratatui_color())
        .add_modifier(Modifier::BOLD)
        .bg(theme.bg_footer.to_ratatui_color());

    let width = width as usize;

    // Left section: mode label + workspace path
    let left: String = model.mode_label.as_deref().unwrap_or("").to_string();
    let left_width = left.chars().count();
    let left_pad = if left.is_empty() { 0 } else { left_width + 2 };

    let mut spans: Vec<Span> = Vec::new();

    // Left: mode label
    if !left.is_empty() {
        spans.push(Span::styled(left, accent_style));
        spans.push(Span::styled("  ", dim_style));
    }

    // Left: red dot then workspace path (compact)
    if let Ok(cwd) = std::env::current_dir() {
        let ws = cwd.to_string_lossy();
        let ws_short: String = if ws.chars().count() > 28 {
            ws.chars().take(26).collect::<String>() + "…"
        } else {
            ws.to_string()
        };
        let red_dot = Style::default()
            .fg(theme.accent_primary.to_ratatui_color())
            .bg(theme.bg_footer.to_ratatui_color());
        spans.push(Span::styled("● ", red_dot));
        spans.push(Span::styled(format!("{}  ", ws_short), dim_style));
    }

    // ----  Build left-center segments (context bar + tx metric)  ----
    let mut left_segments: Vec<(String, Style)> = Vec::new();
    let mut tx_metric_idx: Option<usize> = None;

    if let Some(ctx) = model.context_pct {
        let pct = ctx.min(100) as f64 / 100.0;
        let bar_width = 10usize;
        let filled = (pct * bar_width as f64).round() as usize;
        // At any non-zero percentage, show at least 1 char of fill
        let filled = if ctx > 0 && filled == 0 { 1 } else { filled };

        let bar_fill: String = "─".repeat(filled);
        let bar_empty: String = "─".repeat(bar_width.saturating_sub(filled));

        let bar_color = Style::default()
            .fg(theme.accent_primary.to_ratatui_color())
            .bg(theme.bg_footer.to_ratatui_color());
        let empty_color = Style::default()
            .fg(theme.border.to_ratatui_color())
            .bg(theme.bg_footer.to_ratatui_color());

        left_segments.push((format!("ctx {}% ", ctx.min(100)), dim_style));
        left_segments.push((bar_fill, bar_color));
        left_segments.push((bar_empty, empty_color));

        if let Some(tx) = &model.transcript_metric {
            tx_metric_idx = Some(left_segments.len());
            left_segments.push((format!("  {}", tx), dim_style));
        }
    }

    // ----  Build right segments (streaming state + mode)  ----
    let mut right_segments: Vec<(String, Style)> = Vec::new();
    if let Some(state) = streaming_state.as_ref() {
        right_segments.push((state.clone(), dim_style));
    }
    let response_mode = crate::ui_state::current_response_mode();
    let response_str = match response_mode {
        crate::ui_state::ResponseMode::Concise => "Concise",
        crate::ui_state::ResponseMode::Long => "Long",
    };
    let access_mode = crate::ui_state::current_access_mode();
    let access_str = match access_mode {
        crate::ui_state::AccessMode::Review => "Review",
        crate::ui_state::AccessMode::Full => "Full",
    };
    if !right_segments.is_empty() {
        right_segments.push(("  ".to_string(), dim_style));
    }
    right_segments.push((response_str.to_string(), accent_style));
    right_segments.push((" | ".to_string(), dim_style));
    right_segments.push((access_str.to_string(), accent_style));

    // Width-aware rendering
    let available = width.saturating_sub(left_pad);

    let left_total: usize = left_segments.iter().map(|(s, _)| s.chars().count()).sum();
    let right_total: usize = right_segments.iter().map(|(s, _)| s.chars().count()).sum();
    if left_total + right_total > available {
        if let Some(idx) = tx_metric_idx {
            left_segments.remove(idx);
        }
    }
    let left_total: usize = left_segments.iter().map(|(s, _)| s.chars().count()).sum();
    let right_total: usize = right_segments.iter().map(|(s, _)| s.chars().count()).sum();

    if left_total + right_total <= available {
        for (text, style) in &left_segments {
            spans.push(Span::styled(text.clone(), *style));
        }
        let pad = available.saturating_sub(left_total + right_total);
        if pad > 0 {
            spans.push(Span::styled(" ".repeat(pad), dim_style));
        }
        for (text, style) in &right_segments {
            spans.push(Span::styled(text.clone(), *style));
        }
    } else {
        let mut remaining = available;
        for (text, style) in &left_segments {
            let w = text.chars().count();
            if w <= remaining {
                spans.push(Span::styled(text.clone(), *style));
                remaining -= w;
            } else {
                break;
            }
        }
    }

    Line::from(spans).style(footer_bg)
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
        assert!(lines[0].starts_with("|_ "));
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
        // Expanded thinking no longer shows inline collapse hint;
        // collapse is managed via click or ctrl+o (event-driven, not text-driven).
        let any_collapse_hint = lines
            .iter()
            .any(|l| l.contains("(ctrl+o to collapse)"));
        assert!(!any_collapse_hint);
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

        // Thinking is removed from left panel — hidden in transcript render
        let live_lines = renderer.transcript.render();
        assert!(
            live_lines.iter().all(|line| !line.contains("∴")),
            "thinking header should NOT appear in left panel transcript"
        );
        assert!(
            live_lines.iter().all(|line| !line.contains("live reasoning text")),
            "thinking content should NOT appear in left panel"
        );

        renderer.finish_thinking();
        let held_lines = renderer.transcript.render();
        assert!(
            held_lines.iter().all(|line| !line.contains("live reasoning text")),
            "finished thinking should NOT appear in left panel"
        );

        renderer.transcript.thinking_collapse_deadline =
            Some((0, Instant::now() - Duration::from_secs(1)));
        let collapsed_lines = renderer.transcript.render();
        assert!(
            collapsed_lines.iter().all(|line| !line.contains("Thinking")),
            "thinking should NOT render as collapsed row in left panel"
        );
    }

    #[test]
    fn test_live_thinking_renders_in_expanded_layout_while_streaming() {
        let mut renderer = ClaudeRenderer::new(80, 24);
        renderer.start_thinking();
        renderer.append_thinking("first second third fourth fifth");

        let (lines, _) = renderer.transcript.render_ratatui(80);
        // Thinking is removed from left panel — lives only in right panel.
        // The transcript should have NO thinking content shown.
        assert!(
            lines.is_empty(),
            "thinking should not render in left panel transcript"
        );
        assert!(
            !lines.iter().any(|l| fragments_contain(l, "first second third fourth fifth")),
            "thinking content should not appear in left panel"
        );
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

        // Thinking is removed from left panel — not rendered in transcript.
        let (lines, mapping) = renderer.transcript.render_ratatui(80);
        assert_eq!(lines.len(), 0, "thinking should not render in left panel");

        // Clicking thinking row in transcript has no visible effect
        if let Some(&msg_idx) = mapping.get(0) {
            renderer.transcript.toggle_trace_collapse(msg_idx);
        }
        let (lines2, _) = renderer.transcript.render_ratatui(80);
        assert_eq!(lines2.len(), 0, "after click, thinking still not in left panel");
    }

    #[test]
    fn test_footer_drops_transcript_metric_on_narrow_width() {
        let model = FooterModel {
            context_pct: Some(84),
            model_label: Some(
                "granite-4.0-h-micro-UD-Q8_K_XL.gguf/very/long/model/name".to_string(),
            ),
            transcript_metric: Some("tx 123456".to_string()),
            mode_label: None,
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
    fn test_footer_model_label_removed_from_bottom_bar() {
        let model = FooterModel {
            context_pct: Some(40),
            model_label: Some("granite-4.0-h-micro-UD-Q8_K_XL.gguf".to_string()),
            transcript_metric: Some("tx 1024".to_string()),
            mode_label: None,
        };
        let line = render_footer_line(&model, None, 120);
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(!text.contains("granite"));
        assert!(text.contains("ctx 40%"));
    }

    #[test]
    fn test_transcript_lines_wrap_before_viewport_math() {
        let line = Line::from(vec![Span::raw("abcdefghi")]);
        let (wrapped, mapping) = wrap_lines_with_mapping(vec![line], vec![7], 4);

        let rendered: Vec<String> = wrapped
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();

        assert_eq!(rendered, vec!["abcd", "efgh", "i"]);
        assert_eq!(mapping, vec![7, 7, 7]);
    }

    #[test]
    fn test_bottom_view_uses_wrapped_physical_rows() {
        let long = Line::from(vec![Span::raw("abcdefghij")]);
        let end = Line::from(vec![Span::raw("END")]);
        let (wrapped, _) = wrap_lines_with_mapping(vec![long, end], vec![0, 1], 4);

        let height = 2usize;
        let total_lines = wrapped.len();
        let start_line = total_lines.saturating_sub(height);
        let visible: Vec<String> = wrapped
            .into_iter()
            .skip(start_line)
            .take(height)
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect();

        assert_eq!(visible, vec!["ij", "END"]);
    }

    #[test]
    fn test_footer_context_bar_block_colors() {
        let theme = current_theme();
        let model = FooterModel {
            // 38% → filled = round(0.38 * 10) = 4 → 4 ─ filled + 6 ─ empty
            context_pct: Some(38),
            model_label: None,
            transcript_metric: None,
            mode_label: None,
        };
        let line = render_footer_line(&model, None, 80);
        let mut seen_fill_accent = false;
        let mut seen_empty_border = false;
        for span in &line.spans {
            let fg = span.style.fg;
            if span.content.contains('─') {
                if fg == Some(theme.accent_primary.to_ratatui_color()) {
                    seen_fill_accent = true;
                }
                if fg == Some(theme.border.to_ratatui_color()) {
                    seen_empty_border = true;
                }
            }
        }
        assert!(seen_fill_accent, "filled bar chars should use accent_primary");
        assert!(seen_empty_border, "empty bar chars should use border color");
    }

    #[test]
    fn test_footer_context_label_dim() {
        let theme = current_theme();
        let model = FooterModel {
            context_pct: Some(42),
            model_label: None,
            transcript_metric: None,
            mode_label: None,
        };
        let line = render_footer_line(&model, None, 80);
        let label = line.spans.iter().find(|s| s.content.starts_with("ctx"));
        assert!(
            label.is_some_and(|s| s.style.fg == Some(theme.fg_dim.to_ratatui_color())),
            "ctx label should use dim style"
        );
    }

    fn fragments_contain(line: &ratatui::text::Line, needle: &str) -> bool {
        line.spans.iter().any(|s| s.content.contains(needle))
    }
}

/// Format a unix timestamp as a short relative age string.
fn format_relative_age(unix_s: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let diff = now.saturating_sub(unix_s);
    if diff < 60 {
        " just now".to_string()
    } else if diff < 3600 {
        format!(" {}m", diff / 60)
    } else if diff < 86400 {
        format!(" {}h", diff / 3600)
    } else if diff < 604800 {
        format!(" {}d", diff / 86400)
    } else if diff < 2592000 {
        format!(" {}w", diff / 604800)
    } else {
        format!(" {}mo", diff / 2592000)
    }
}

/// Format a ClaudeMessage into a terminal transcript line (same format as cleanup()).
pub(crate) fn claude_message_to_transcript_line(msg: &ClaudeMessage) -> String {
    match msg {
        ClaudeMessage::User { content } => {
            format!("> {}\n\n", content)
        }
        ClaudeMessage::Assistant { content } => {
            format!("● {}\n\n", content.raw_markdown)
        }
        ClaudeMessage::Thinking { content, .. } => {
            format!("∴ Thinking: {}\n\n", content)
        }
        ClaudeMessage::ToolStart { name, input } => {
            let mut out = format!("▸ Tool start: {}\n", name);
            if let Some(i) = input {
                out.push_str(&format!("input: {}\n", i));
            }
            out.push('\n');
            out
        }
        ClaudeMessage::ToolProgress { name, message } => {
            format!("▸ Tool progress ({}): {}\n\n", name, message)
        }
        ClaudeMessage::ToolResult {
            name,
            success,
            output,
            duration_ms,
        } => {
            // Truncated output — full tool results live in artifacts/
            let truncated = terminal_tool_output_preview(&name, output);
            format!(
                "✓ Tool result ({}): success={} duration_ms={:?}\n{}\n\n",
                name, success, duration_ms, truncated
            )
        }
        ClaudeMessage::ToolTrace {
            name,
            command,
            status,
            ..
        } => {
            let mut out = format!("▸ Tool trace ({}): {}\n", name, command);
            match status {
                super::claude_state::ToolTraceStatus::Running => {
                    out.push_str("status: running\n\n");
                }
                super::claude_state::ToolTraceStatus::Completed {
                    success,
                    output,
                    duration_ms,
                } => {
                    let truncated = terminal_tool_output_preview(&name, output);
                    out.push_str(&format!(
                        "status: completed success={} duration_ms={:?}\n{}\n\n",
                        success, duration_ms, truncated
                    ));
                }
            }
            out
        }
        ClaudeMessage::PermissionRequest { command, reason } => {
            format!(
                "? Permission requested: {} reason={:?}\n\n",
                command, reason
            )
        }
        ClaudeMessage::CompactBoundary => "✻ Conversation compacted\n\n".to_string(),
        ClaudeMessage::CompactSummary {
            message_count,
            context_preview,
        } => {
            format!(
                "✻ Compact summary: {} messages\n{}\n\n",
                message_count,
                context_preview.as_deref().unwrap_or("")
            )
        }
        ClaudeMessage::System { content } => {
            format!("system: {}\n\n", content)
        }
        ClaudeMessage::Notice(notice) => {
            format!("◦ NOTICE ({:?}): {}\n\n", notice.kind, notice.content)
        }
    }
}

/// Character limit for transcript tool output preview.
const TRANSCRIPT_OUTPUT_LIMIT: usize = 1024;
const SHELL_OUTPUT_LINES: usize = 6; // Visual truncation for shell commands like `cat`

/// Truncate tool output for terminal transcript to prevent memory spikes.
/// For shell tool, show only first N lines visually (context still gets full output).
fn terminal_tool_output_preview(name: &str, output: &str) -> String {
    // Visual truncation for shell tool: show only first 6 lines
    if name == "shell" {
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() > SHELL_OUTPUT_LINES {
            let preview_lines = lines[..SHELL_OUTPUT_LINES].join("\n");
            return format!(
                "{}\n… [+{} lines truncated for display]",
                preview_lines,
                lines.len() - SHELL_OUTPUT_LINES
            );
        }
    }
    // Default: truncate by character count
    let preview: String = output.chars().take(TRANSCRIPT_OUTPUT_LIMIT).collect();
    if output.len() > TRANSCRIPT_OUTPUT_LIMIT {
        format!(
            "{preview}… [+{} characters truncated]",
            output.len().saturating_sub(TRANSCRIPT_OUTPUT_LIMIT)
        )
    } else {
        preview
    }
}

/// Condense workspace_info output for transcript display (Task 592).
/// Strips the directory tree listing (200+ lines) and replaces it with
/// a summary line. Keeps: root path, project type, git status, guidance.
fn condense_workspace_info_for_transcript(output: &str) -> String {
    let mut result = String::new();
    let mut in_dir_tree = false;
    let mut dir_tree_lines = 0usize;
    let mut lines_after_dir = Vec::new();
    let mut passed_dir_tree = false;

    for line in output.lines() {
        if line.starts_with("## Directory Structure") {
            in_dir_tree = true;
            dir_tree_lines = 1;
            continue;
        }
        if in_dir_tree {
            if line.starts_with("## ") {
                in_dir_tree = false;
                passed_dir_tree = true;
            } else {
                dir_tree_lines += 1;
                continue;
            }
        }
        if passed_dir_tree {
            lines_after_dir.push(line);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result.push_str(&format!(
        "Directory tree available in evidence; summary: ~{} entries\n",
        dir_tree_lines
    ));
    for line in &lines_after_dir {
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Truncate large tool output for the terminal transcript file (Task 593).
/// Returns output safe for the transcript file (not the model context).
fn transcript_safe_output(name: &str, output: &str) -> String {
    if name == "workspace_info" {
        let condensed = condense_workspace_info_for_transcript(output);
        terminal_tool_output_preview(name, &condensed)
    } else if name == "shell" {
        let limit = 2000usize;
        let preview: String = output.chars().take(limit).collect();
        if output.len() > limit {
            format!(
                "{preview}… [+{} characters truncated — full content available to model and in session evidence]",
                output.len().saturating_sub(limit)
            )
        } else {
            preview
        }
    } else {
        terminal_tool_output_preview(name, output)
    }
}
