//! @efficiency-role: infra-config
//!
//! UI - State Management

use crate::ui_autocomplete::AutocompleteState;
use crate::*;
use std::collections::HashMap;

static TRACE_LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static REASONING_DISPLAY: OnceLock<Mutex<(bool, bool)>> = OnceLock::new();
static JSON_OUTPUTTER_PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static FINAL_ANSWER_EXTRACTOR_PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static MODEL_BEHAVIOR_PROFILE: OnceLock<Mutex<Option<ModelBehaviorProfile>>> = OnceLock::new();
/// Tracks intel unit failures: (unit_name -> [(error_message, count)])
static INTEL_FAILURE_COUNTS: OnceLock<Mutex<HashMap<String, usize>>> = OnceLock::new();
/// Whether the TUI is currently active (to suppress stderr status messages)
static TUI_ACTIVE: OnceLock<Mutex<bool>> = OnceLock::new();

pub(crate) fn trace_log_state() -> &'static Mutex<Option<PathBuf>> {
    TRACE_LOG_PATH.get_or_init(|| Mutex::new(None))
}

pub(crate) fn reasoning_display_state() -> &'static Mutex<(bool, bool)> {
    REASONING_DISPLAY.get_or_init(|| Mutex::new((false, false)))
}

pub(crate) fn json_outputter_state() -> &'static Mutex<Option<Profile>> {
    JSON_OUTPUTTER_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn final_answer_extractor_state() -> &'static Mutex<Option<Profile>> {
    FINAL_ANSWER_EXTRACTOR_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn model_behavior_state() -> &'static Mutex<Option<ModelBehaviorProfile>> {
    MODEL_BEHAVIOR_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_trace_log_path(path: Option<PathBuf>) {
    if let Ok(mut slot) = trace_log_state().lock() {
        *slot = path;
    }
}

pub(crate) fn set_reasoning_display(show_terminal: bool, no_color: bool) {
    if let Ok(mut slot) = reasoning_display_state().lock() {
        *slot = (show_terminal, no_color);
    }
}

pub(crate) fn set_json_outputter_profile(profile: Option<Profile>) {
    if let Ok(mut slot) = json_outputter_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn set_final_answer_extractor_profile(profile: Option<Profile>) {
    if let Ok(mut slot) = final_answer_extractor_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn set_model_behavior_profile(profile: Option<ModelBehaviorProfile>) {
    if let Ok(mut slot) = model_behavior_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn current_model_behavior_profile() -> Option<ModelBehaviorProfile> {
    model_behavior_state().lock().ok()?.clone()
}

pub(crate) fn json_outputter_profile() -> Option<Profile> {
    json_outputter_state().lock().ok()?.clone()
}

pub(crate) fn final_answer_extractor_profile() -> Option<Profile> {
    final_answer_extractor_state().lock().ok()?.clone()
}

/// Increment the failure counter for an intel unit.
pub(crate) fn increment_intel_failure_count(unit_name: &str, _error: &str) {
    if let Ok(mut counts) = INTEL_FAILURE_COUNTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        *counts.entry(unit_name.to_string()).or_insert(0) += 1;
    }
}

/// Get the total failure count across all intel units.
pub(crate) fn get_total_intel_failures() -> usize {
    INTEL_FAILURE_COUNTS
        .get()
        .and_then(|m| m.lock().ok())
        .map(|m| m.values().sum())
        .unwrap_or(0)
}

/// Get per-unit failure counts.
pub(crate) fn get_intel_failure_counts() -> HashMap<String, usize> {
    INTEL_FAILURE_COUNTS
        .get()
        .and_then(|m| m.lock().ok())
        .map(|m| m.clone())
        .unwrap_or_default()
}

/// Reset the failure counters (called at session start).
pub(crate) fn reset_intel_failure_counts() {
    if let Ok(mut counts) = INTEL_FAILURE_COUNTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        counts.clear();
    }
}

/// Mark the TUI as active (called when entering TUI mode).
pub(crate) fn set_tui_active(active: bool) {
    if let Ok(mut slot) = TUI_ACTIVE.get_or_init(|| Mutex::new(false)).lock() {
        *slot = active;
    }
}

/// Check if the TUI is currently active.
pub(crate) fn is_tui_active() -> bool {
    TUI_ACTIVE
        .get()
        .and_then(|m| m.lock().ok())
        .map(|v| *v)
        .unwrap_or(false)
}

// ============================================================================
// Reasoning Visibility Toggle (Task U008)
// ============================================================================

static SHOW_REASONING: OnceLock<Mutex<bool>> = OnceLock::new();

/// Get the reasoning visibility state.
pub(crate) fn show_reasoning_state() -> &'static Mutex<bool> {
    SHOW_REASONING.get_or_init(|| Mutex::new(false))
}

/// Check if reasoning blocks should be shown.
pub(crate) fn is_reasoning_visible() -> bool {
    match show_reasoning_state().lock() {
        Ok(guard) => *guard,
        Err(_) => true,
    }
}

/// Set reasoning visibility.
pub(crate) fn set_show_reasoning(show: bool) {
    if let Ok(mut slot) = show_reasoning_state().lock() {
        *slot = show;
    }
}

/// Toggle reasoning visibility and return the new state.
pub(crate) fn toggle_show_reasoning() -> bool {
    let mut slot = show_reasoning_state().lock().unwrap();
    *slot = !*slot;
    *slot
}

// ============================================================================
// UI State Model (Premium Terminal UI — Task 142)
//
// Per-session display state. Separate from the global OnceLock state above.
// ============================================================================

/// A single durable item in the conversation transcript.
#[derive(Clone, Debug)]
pub(crate) enum TranscriptItem {
    /// User message. Prefix: "> "
    User { content: String },
    /// Assistant markdown response. Prefix: "● "
    Assistant { content: String },
    /// Tool execution started.
    ToolStart { name: String, command: String },
    /// Tool execution completed.
    ToolResult {
        name: String,
        success: bool,
        output: String,
        duration_ms: Option<u64>,
    },
    /// Process / system meta event: PLAN, CLASSIFY, REFLECT, etc.
    MetaEvent { category: String, message: String },
    /// Warning / blocked action alert.
    Warning { message: String },
    /// Hidden reasoning / thinking trace (shown only when flagged).
    Thinking { content: String },
    /// Generic system message.
    System { content: String },
}

/// Footer metrics display state.
#[derive(Clone, Debug, Default)]
pub(crate) struct FooterMetrics {
    pub model: String,
    pub context_current: u64,
    pub context_max: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub effort: String,
    pub route: String,
    pub approval_policy: String, // "yolo", "auto", "approve"
}

/// Activity rail state — ephemeral live status above the composer.
#[derive(Clone, Debug, Default)]
pub(crate) enum ActivityState {
    #[default]
    Idle,
    Active {
        label: String,
        message: String,
    },
}

/// Modal overlay state.
#[derive(Clone, Debug)]
pub(crate) enum ModalState {
    /// Confirmation prompt for destructive commands.
    Confirm { title: String, message: String },
    /// Help / slash-command reference (two-column keyboard shortcut reference).
    Help { content: String },
    /// Selection list (delegates to inquire for actual prompting).
    Select { title: String, options: Vec<String> },
    /// Settings display: provider, model, approval policy, paths.
    Settings { content: String },
    /// Usage/stats dialog: token count + cost.
    Usage { content: String },
    /// Tool approval dialog with Yes/Always/No options.
    ToolApproval {
        tool_name: String,
        description: String,
        selected: usize, // 0=Yes, 1=Always, 2=No
    },
    /// General permission gate for destructive commands.
    PermissionGate {
        command: String,
        risk_level: String,
        selected: usize, // 0=Yes, 1=Always, 2=No
    },
    /// Plan progress widget.
    PlanProgress {
        title: String,
        current: usize,
        total: usize,
        steps: Vec<String>, // step descriptions with status
    },
    /// Notification — auto-dismiss message.
    Notification {
        message: String,
        level: String, // "info", "warning", "error"
    },
    /// Splash screen on startup.
    Splash { content: String },
}

/// Viewport / scroll state.
#[derive(Clone, Debug, Default)]
pub(crate) struct ViewportState {
    pub scroll_offset: usize,
    pub total_rendered_lines: usize,
    pub visible_lines: usize,
    /// Whether the user has manually scrolled up (disables auto-scroll until they return to bottom)
    pub user_scrolled_up: bool,
}

/// Header metadata — shown in the top strip.
#[derive(Clone, Debug, Default)]
pub(crate) struct HeaderInfo {
    pub model: String,
    pub endpoint: String,
    pub route: String,
    pub workspace: String,
    pub session: String,
    pub workflow: String,
    pub verbose: bool,
}

/// Streaming/rendering state for live response display.
#[derive(Clone, Debug, Default)]
pub(crate) struct StreamingState {
    /// What kind of streaming is happening.
    pub kind: StreamingKind,
    /// Current animation frame index (cycles 0-9).
    pub animation_frame: usize,
    /// Elapsed seconds.
    pub elapsed_s: u64,
    /// Token count.
    pub tokens: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum StreamingKind {
    /// Tool execution / waiting — no text streaming yet.
    Processing,
    /// Text response streaming — tokens arriving.
    Responding,
    #[default]
    Idle,
}

/// Complete UI state for rendering the full screen.
#[derive(Clone, Debug, Default)]
pub(crate) struct UIState {
    pub header: HeaderInfo,
    pub transcript: Vec<TranscriptItem>,
    pub activity: ActivityState,
    pub footer: FooterMetrics,
    pub modal: Option<ModalState>,
    pub viewport: ViewportState,
    pub show_thinking: bool,
    pub autocomplete: AutocompleteState,
    pub streaming: StreamingState,
}

impl UIState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // --- Transcript push methods ---

    pub(crate) fn push_user_message(&mut self, content: &str) {
        self.transcript.push(TranscriptItem::User {
            content: content.to_string(),
        });
        // Only auto-scroll if user hasn't scrolled up
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    pub(crate) fn push_assistant_markdown(&mut self, content: &str) {
        self.transcript.push(TranscriptItem::Assistant {
            content: content.to_string(),
        });
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    pub(crate) fn push_meta_event(&mut self, category: &str, message: &str) {
        self.transcript.push(TranscriptItem::MetaEvent {
            category: category.to_string(),
            message: message.to_string(),
        });
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    pub(crate) fn push_tool_start(&mut self, name: &str, command: &str) {
        self.transcript.push(TranscriptItem::ToolStart {
            name: name.to_string(),
            command: command.to_string(),
        });
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    pub(crate) fn push_tool_finish(
        &mut self,
        name: &str,
        success: bool,
        output: &str,
        duration_ms: Option<u64>,
    ) {
        self.transcript.push(TranscriptItem::ToolResult {
            name: name.to_string(),
            success,
            output: output.to_string(),
            duration_ms,
        });
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    pub(crate) fn push_warning(&mut self, message: &str) {
        self.transcript.push(TranscriptItem::Warning {
            message: message.to_string(),
        });
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    pub(crate) fn push_thinking(&mut self, content: &str) {
        if self.show_thinking {
            self.transcript.push(TranscriptItem::Thinking {
                content: content.to_string(),
            });
            if !self.viewport.user_scrolled_up {
                self.viewport.scroll_offset = 0;
            }
        }
    }

    pub(crate) fn push_system(&mut self, content: &str) {
        self.transcript.push(TranscriptItem::System {
            content: content.to_string(),
        });
        if !self.viewport.user_scrolled_up {
            self.viewport.scroll_offset = 0;
        }
    }

    // --- Activity rail ---

    pub(crate) fn set_activity(&mut self, label: &str, message: &str) {
        self.activity = ActivityState::Active {
            label: label.to_string(),
            message: message.to_string(),
        };
    }

    pub(crate) fn clear_activity(&mut self) {
        self.activity = ActivityState::Idle;
    }

    // --- Footer ---

    pub(crate) fn set_footer_metrics(&mut self, metrics: FooterMetrics) {
        self.footer = metrics;
    }

    // --- Modal ---

    pub(crate) fn set_modal(&mut self, modal: ModalState) {
        self.modal = Some(modal);
    }

    pub(crate) fn clear_modal(&mut self) {
        self.modal = None;
    }

    // --- Scroll ---

    pub(crate) fn scroll_up(&mut self, amount: usize) {
        self.viewport.scroll_offset = self.viewport.scroll_offset.saturating_add(amount);
        self.viewport.user_scrolled_up = true;
    }

    pub(crate) fn scroll_down(&mut self, amount: usize) {
        self.viewport.scroll_offset = self.viewport.scroll_offset.saturating_sub(amount);
        // Check if we're back at bottom
        if self.viewport.scroll_offset == 0 {
            self.viewport.user_scrolled_up = false;
        }
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.viewport.scroll_offset = 0;
        self.viewport.user_scrolled_up = false;
    }

    // --- Reset ---

    pub(crate) fn reset(&mut self) {
        self.transcript.clear();
        self.activity = ActivityState::Idle;
        self.modal = None;
        self.viewport = ViewportState::default();
    }
}

#[cfg(test)]
mod ui_state_tests {
    use super::*;

    #[test]
    fn test_ui_state_new() {
        let state = UIState::new();
        assert!(state.transcript.is_empty());
        assert!(matches!(state.activity, ActivityState::Idle));
        assert!(state.modal.is_none());
    }

    #[test]
    fn test_push_user_message() {
        let mut state = UIState::new();
        state.push_user_message("hello");
        assert_eq!(state.transcript.len(), 1);
        if let TranscriptItem::User { content } = &state.transcript[0] {
            assert_eq!(content, "hello");
        } else {
            panic!("expected User");
        }
    }

    #[test]
    fn test_push_assistant_markdown() {
        let mut state = UIState::new();
        state.push_assistant_markdown("**bold** text");
        assert_eq!(state.transcript.len(), 1);
        if let TranscriptItem::Assistant { content } = &state.transcript[0] {
            assert_eq!(content, "**bold** text");
        } else {
            panic!("expected Assistant");
        }
    }

    #[test]
    fn test_push_meta_event() {
        let mut state = UIState::new();
        state.push_meta_event("PLAN", "3 steps");
        assert_eq!(state.transcript.len(), 1);
        if let TranscriptItem::MetaEvent { category, message } = &state.transcript[0] {
            assert_eq!(category, "PLAN");
            assert_eq!(message, "3 steps");
        } else {
            panic!("expected MetaEvent");
        }
    }

    #[test]
    fn test_push_tool_start_and_finish() {
        let mut state = UIState::new();
        state.push_tool_start("SHELL", "cargo test");
        state.push_tool_finish("SHELL", true, "ok", Some(300));
        assert_eq!(state.transcript.len(), 2);
        if let TranscriptItem::ToolResult {
            name,
            success,
            duration_ms,
            ..
        } = &state.transcript[1]
        {
            assert_eq!(name, "SHELL");
            assert!(*success);
            assert_eq!(*duration_ms, Some(300));
        } else {
            panic!("expected ToolResult");
        }
    }

    #[test]
    fn test_push_warning() {
        let mut state = UIState::new();
        state.push_warning("destructive command blocked");
        assert_eq!(state.transcript.len(), 1);
        if let TranscriptItem::Warning { message } = &state.transcript[0] {
            assert_eq!(message, "destructive command blocked");
        } else {
            panic!("expected Warning");
        }
    }

    #[test]
    fn test_activity_toggle() {
        let mut state = UIState::new();
        assert!(matches!(state.activity, ActivityState::Idle));
        state.set_activity("shell", "cargo test");
        if let ActivityState::Active { label, message } = &state.activity {
            assert_eq!(label, "shell");
            assert_eq!(message, "cargo test");
        } else {
            panic!("expected Active");
        }
        state.clear_activity();
        assert!(matches!(state.activity, ActivityState::Idle));
    }

    #[test]
    fn test_scroll() {
        let mut state = UIState::new();
        state.scroll_up(5);
        assert_eq!(state.viewport.scroll_offset, 5);
        state.scroll_down(3);
        assert_eq!(state.viewport.scroll_offset, 2);
        state.scroll_to_bottom();
        assert_eq!(state.viewport.scroll_offset, 0);
    }

    #[test]
    fn test_reset() {
        let mut state = UIState::new();
        state.push_user_message("hello");
        state.set_activity("x", "y");
        state.scroll_up(10);
        state.reset();
        assert!(state.transcript.is_empty());
        assert!(matches!(state.activity, ActivityState::Idle));
        assert_eq!(state.viewport.scroll_offset, 0);
    }

    #[test]
    fn test_footer_metrics() {
        let mut state = UIState::new();
        let metrics = FooterMetrics {
            model: "qwen3:4b".to_string(),
            context_current: 4096,
            context_max: 8192,
            tokens_in: 500,
            tokens_out: 200,
            effort: "⏱ 2.3s".to_string(),
            route: "WORKFLOW".to_string(),
            approval_policy: "auto".to_string(),
        };
        state.set_footer_metrics(metrics);
        assert_eq!(state.footer.model, "qwen3:4b");
        assert_eq!(state.footer.context_current, 4096);
        assert_eq!(state.footer.approval_policy, "auto");
    }

    #[test]
    fn test_user_scrolled_up_flag() {
        let mut state = UIState::new();
        assert!(!state.viewport.user_scrolled_up);
        state.scroll_up(5);
        assert!(state.viewport.user_scrolled_up);
        state.scroll_to_bottom();
        assert!(!state.viewport.user_scrolled_up);
    }

    #[test]
    fn test_scroll_down_resets_user_scrolled_up() {
        let mut state = UIState::new();
        state.scroll_up(10);
        assert!(state.viewport.user_scrolled_up);
        state.scroll_down(10);
        assert_eq!(state.viewport.scroll_offset, 0);
        assert!(!state.viewport.user_scrolled_up);
    }

    #[test]
    fn test_auto_scroll_respects_user_scrolled_up() {
        let mut state = UIState::new();
        // User scrolls up
        state.scroll_up(5);
        assert!(state.viewport.user_scrolled_up);
        assert_eq!(state.viewport.scroll_offset, 5);

        // New message arrives
        state.push_user_message("new message");
        // scroll_offset should NOT reset because user scrolled up
        assert_eq!(state.viewport.scroll_offset, 5);
        assert!(state.viewport.user_scrolled_up);

        // User scrolls to bottom
        state.scroll_to_bottom();
        assert!(!state.viewport.user_scrolled_up);

        // New message now auto-scrolls
        state.push_user_message("another message");
        assert_eq!(state.viewport.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_offset_not_changed_when_user_scrolled_up() {
        let mut state = UIState::new();
        state.push_user_message("msg1");
        state.push_user_message("msg2");
        state.scroll_up(3);
        assert!(state.viewport.user_scrolled_up);

        state.push_tool_start("SHELL", "ls");
        assert_eq!(state.viewport.scroll_offset, 3);

        state.push_tool_finish("SHELL", true, "ok", Some(100));
        assert_eq!(state.viewport.scroll_offset, 3);

        state.push_meta_event("PLAN", "planning");
        assert_eq!(state.viewport.scroll_offset, 3);
    }
}
