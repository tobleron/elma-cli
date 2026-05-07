//! @efficiency-role: data-model
//!
//! UiViewState — pure view state derived from UiRuntimeEvents via a reducer.
//!
//! This is a "model" in the Model-View-Update sense.  It holds everything
//! the renderer needs and nothing else.  No terminal I/O, no session I/O,
//! no domain logic — just a typed snapshot of what to draw.
//!
//! Task 635: New pure view state extracted from ClaudeRenderer's ad-hoc fields.
//! Task 636: Wrapped-line caches, token counters, and disk I/O will live in
//! separate services keyed by (message_id, width).

use crate::claude_ui::{
    AssistantContent, ClaudeMessage, ClaudeTranscript, ExitState, InputMode, PickerState,
    SessionCommand, TaskList,
};
use crate::ui::ui_autocomplete::AutocompleteState;
use crate::ui::ui_modal_search::SearchModal;
use crate::ui::ui_model_picker::ModelPicker;
use crate::ui_state::ModalState;
use crate::ui_status_thread::StatusThread;
use ratatui::widgets::ScrollbarState;
use std::time::Instant;

/// Footer data — the only persistent bar in the UI.
#[derive(Clone, Debug)]
pub(crate) struct FooterState {
    pub model_label: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_current: u64,
    pub context_max: u64,
    pub elapsed_secs: u64,
}

impl Default for FooterState {
    fn default() -> Self {
        Self {
            model_label: None,
            input_tokens: 0,
            output_tokens: 0,
            context_current: 0,
            context_max: 0,
            elapsed_secs: 0,
        }
    }
}

/// Thinking entry for the right panel.
#[derive(Clone, Debug)]
pub(crate) struct ThinkingEntry {
    pub content: String,
    pub word_count: usize,
    pub created_at: Instant,
    pub collapse_deadline: Instant,
    pub collapsed: bool,
    pub reveal_chars: usize,
    pub is_summary: bool,
}

/// Full view state — all data the renderers need to draw one frame.
#[derive(Clone, Debug)]
pub(crate) struct UiViewState {
    // ── Transcript ──────────────────────────────────────────────────────
    pub transcript: ClaudeTranscript,
    pub transcript_mode: TranscriptViewMode,

    // ── Streaming ───────────────────────────────────────────────────────
    pub is_streaming_thinking: bool,
    pub is_streaming_content: bool,
    pub streaming_thought: String,
    pub streaming_text: String,

    // ── Tool traces ─────────────────────────────────────────────────────
    pub active_tool_traces: Vec<ActiveToolTrace>,

    // ── Thinking panel ──────────────────────────────────────────────────
    pub thinking_entries: Vec<ThinkingEntry>,
    pub thinking_scroll: usize,
    pub last_notice_text: Option<String>,

    // ── Input ───────────────────────────────────────────────────────────
    pub input_lines: Vec<String>,
    pub input_cursor_row: usize,
    pub input_cursor_col: usize,
    pub input_mode: InputMode,
    pub picker_state: PickerState,
    pub file_matches: Vec<String>,
    pub autocomplete_state: Option<AutocompleteState>,

    // ── Footer ──────────────────────────────────────────────────────────
    pub footer: FooterState,

    // ── Task list (left sidebar) ────────────────────────────────────────
    pub task_list: Option<TaskList>,

    // ── Modals ──────────────────────────────────────────────────────────
    pub modal: Option<ModalState>,
    pub search_modal: SearchModal,
    pub model_picker: ModelPicker,
    pub picker: PickerState,

    // ── Background tasks ────────────────────────────────────────────────
    pub background_tasks_visible: bool,
    pub selected_background_task: Option<String>,

    // ── Layout ──────────────────────────────────────────────────────────
    pub terminal_width: usize,
    pub terminal_height: usize,

    // ── Permissions ─────────────────────────────────────────────────────
    pub active_permission_request: Option<PermissionRequestView>,

    // ── Session ─────────────────────────────────────────────────────────
    pub session_id: Option<String>,
    pub exit_state: ExitState,
    pub session_command: SessionCommand,
}

#[derive(Clone, Debug, Default)]
pub(crate) enum TranscriptViewMode {
    #[default]
    Normal,
    Transcript,
    Search {
        query: String,
        matches: Vec<usize>,
        current: usize,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveToolTrace {
    pub name: String,
    pub command: String,
    pub status: ToolTraceViewStatus,
    pub collapsed: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum ToolTraceViewStatus {
    Pending,
    Running,
    Succeeded {
        output: String,
        duration_ms: Option<u64>,
    },
    Failed {
        output: String,
        duration_ms: Option<u64>,
    },
    Denied {
        reason: Option<String>,
    },
    Cancelled,
    TimedOut,
}

#[derive(Clone, Debug)]
pub(crate) struct PermissionRequestView {
    pub command: String,
    pub reason: Option<String>,
}

impl Default for UiViewState {
    fn default() -> Self {
        Self {
            transcript: ClaudeTranscript::new(),
            transcript_mode: TranscriptViewMode::Normal,
            is_streaming_thinking: false,
            is_streaming_content: false,
            streaming_thought: String::new(),
            streaming_text: String::new(),
            active_tool_traces: Vec::new(),
            thinking_entries: Vec::new(),
            thinking_scroll: 0,
            last_notice_text: None,
            input_lines: vec![String::new()],
            input_cursor_row: 0,
            input_cursor_col: 0,
            input_mode: InputMode::Chat,
            picker_state: PickerState::None,
            file_matches: Vec::new(),
            autocomplete_state: None,
            footer: FooterState::default(),
            task_list: None,
            modal: None,
            search_modal: SearchModal::new(),
            model_picker: ModelPicker::new(),
            picker: PickerState::None,
            background_tasks_visible: false,
            selected_background_task: None,
            terminal_width: 80,
            terminal_height: 24,
            active_permission_request: None,
            session_id: None,
            exit_state: ExitState::new(),
            session_command: SessionCommand::None,
        }
    }
}

impl UiViewState {
    /// Context percentage for the footer bar.
    pub(crate) fn context_pct(&self) -> usize {
        if self.footer.context_max > 0 {
            ((self.footer.context_current * 100) / self.footer.context_max) as usize
        } else {
            0
        }
        .min(100)
    }
}
