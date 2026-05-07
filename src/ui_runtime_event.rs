//! @efficiency-role: data-model
//!
//! Canonical UI Runtime Event — single typed event enum for all UI state changes.
//!
//! Every UI-affecting action in the system reduces to one of these events.
//! Events flow: producer → reducer → UiViewState → renderers.
//! Terminal I/O (crossterm) is a producer; it never mutates view state directly.
//! Session persistence is a consumer; it subscribes to events and writes artifacts.
//!
//! Task 635: This is the canonical event type. All previous ad-hoc event types
//! (UiEvent in claude_ui/mod.rs, raw crossterm events in ui_terminal.rs) should
//! route through this type.

use serde::{Deserialize, Serialize};

/// Canonical UI runtime event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum UiRuntimeEvent {
    // ── User input ──────────────────────────────────────────────────────────
    UserSubmitted(String),
    InputChanged(String),
    InputModeChanged(String),

    // ── Model / Assistant lifecycle ─────────────────────────────────────────
    TurnStarted,
    ThinkingStarted,
    ThinkingDelta(String),
    ThinkingFinished,
    AssistantContentStarted,
    AssistantContentDelta(String),
    AssistantContentFinished,
    /// Fired when the final processed answer is ready (after evidence finalizer).
    AssistantFinalAnswer {
        raw: String,
        display: String,
    },

    // ── Tool lifecycle ──────────────────────────────────────────────────────
    ToolProposed {
        name: String,
        input: String,
    },
    ToolStarted {
        name: String,
        command: String,
    },
    ToolProgress {
        name: String,
        message: String,
    },
    ToolSucceeded {
        name: String,
        output: String,
        duration_ms: Option<u64>,
    },
    ToolFailed {
        name: String,
        output: String,
        duration_ms: Option<u64>,
    },
    ToolDenied {
        name: String,
        reason: Option<String>,
    },
    ToolCancelled {
        name: String,
    },
    ToolTimedOut {
        name: String,
    },

    // ── Permission ──────────────────────────────────────────────────────────
    PermissionRequested {
        command: String,
        reason: Option<String>,
    },
    PermissionResolved {
        command: String,
        approved: bool,
    },

    // ── Session lifecycle ───────────────────────────────────────────────────
    SessionStarted {
        id: String,
    },
    SessionCleared,
    SessionResumed {
        id: String,
    },
    CompactBoundary,
    CompactSummary {
        message_count: usize,
        context_preview: Option<String>,
    },
    ExitRequested,
    ExitConfirmed,

    // ── Notices / operational visibility ────────────────────────────────────
    RouteNotice {
        complexity: String,
        route: String,
        formula: String,
    },
    BudgetNotice {
        total: u64,
        used: u64,
        action: String,
    },
    CompactionNotice {
        reason: String,
    },
    StopReasonNotice {
        reason: String,
    },
    RetryNotice {
        attempt: usize,
        cause: String,
    },
    ToolDiscoveryNotice {
        tool_name: String,
        match_type: String,
    },
    SystemNotice {
        message: String,
        level: String,
    },

    // ── Layout ──────────────────────────────────────────────────────────────
    Resize {
        cols: usize,
        rows: usize,
    },

    // ── Footer ──────────────────────────────────────────────────────────────
    FooterModelUpdated {
        model: String,
    },
    FooterTokenCounts {
        input_tokens: u64,
        output_tokens: u64,
        context_current: u64,
        context_max: u64,
    },
    FooterElapsed(u64),

    // ── Background tasks ────────────────────────────────────────────────────
    BackgroundTaskAdded {
        id: String,
        description: String,
    },
    BackgroundTaskUpdated {
        id: String,
        status: String,
    },
    BackgroundTaskRemoved {
        id: String,
    },
    CoverageProgress {
        completed: usize,
        total: usize,
    },
    /// Fired when scope discovery upgrades the initial shape-based assessment (Task 760).
    ComplexityReassessed {
        original: String,
        reassessed: String,
        reason: String,
    },
}

impl UiRuntimeEvent {
    /// Short human-readable label for transcript rows.
    pub(crate) fn event_label(&self) -> &'static str {
        match self {
            UiRuntimeEvent::UserSubmitted(_) => "user_submit",
            UiRuntimeEvent::InputChanged(_) => "input_change",
            UiRuntimeEvent::InputModeChanged(_) => "input_mode",
            UiRuntimeEvent::TurnStarted => "turn_start",
            UiRuntimeEvent::ThinkingStarted => "think_start",
            UiRuntimeEvent::ThinkingDelta(_) => "think_delta",
            UiRuntimeEvent::ThinkingFinished => "think_end",
            UiRuntimeEvent::AssistantContentStarted => "assistant_start",
            UiRuntimeEvent::AssistantContentDelta(_) => "assistant_delta",
            UiRuntimeEvent::AssistantContentFinished => "assistant_end",
            UiRuntimeEvent::AssistantFinalAnswer { .. } => "assistant_final",
            UiRuntimeEvent::ToolProposed { .. } => "tool_proposed",
            UiRuntimeEvent::ToolStarted { .. } => "tool_start",
            UiRuntimeEvent::ToolProgress { .. } => "tool_progress",
            UiRuntimeEvent::ToolSucceeded { .. } => "tool_ok",
            UiRuntimeEvent::ToolFailed { .. } => "tool_fail",
            UiRuntimeEvent::ToolDenied { .. } => "tool_denied",
            UiRuntimeEvent::ToolCancelled { .. } => "tool_cancel",
            UiRuntimeEvent::ToolTimedOut { .. } => "tool_timeout",
            UiRuntimeEvent::PermissionRequested { .. } => "perm_request",
            UiRuntimeEvent::PermissionResolved { .. } => "perm_resolve",
            UiRuntimeEvent::SessionStarted { .. } => "session_start",
            UiRuntimeEvent::SessionCleared => "session_clear",
            UiRuntimeEvent::SessionResumed { .. } => "session_resume",
            UiRuntimeEvent::CompactBoundary => "compact",
            UiRuntimeEvent::CompactSummary { .. } => "compact_summary",
            UiRuntimeEvent::ExitRequested => "exit_request",
            UiRuntimeEvent::ExitConfirmed => "exit_confirm",
            UiRuntimeEvent::RouteNotice { .. } => "route",
            UiRuntimeEvent::BudgetNotice { .. } => "budget",
            UiRuntimeEvent::CompactionNotice { .. } => "compaction",
            UiRuntimeEvent::StopReasonNotice { .. } => "stop_reason",
            UiRuntimeEvent::RetryNotice { .. } => "retry",
            UiRuntimeEvent::ToolDiscoveryNotice { .. } => "tool_discovery",
            UiRuntimeEvent::SystemNotice { .. } => "system",
            UiRuntimeEvent::Resize { .. } => "resize",
            UiRuntimeEvent::FooterModelUpdated { .. } => "footer_model",
            UiRuntimeEvent::FooterTokenCounts { .. } => "footer_tokens",
            UiRuntimeEvent::FooterElapsed(_) => "footer_elapsed",
            UiRuntimeEvent::BackgroundTaskAdded { .. } => "bg_task_add",
            UiRuntimeEvent::BackgroundTaskUpdated { .. } => "bg_task_update",
            UiRuntimeEvent::BackgroundTaskRemoved { .. } => "bg_task_remove",
            UiRuntimeEvent::CoverageProgress { .. } => "coverage",
            UiRuntimeEvent::ComplexityReassessed { .. } => "complexity_reassess",
        }
    }
}

// ── Conversion from older UiEvent ───────────────────────────────────────────

impl From<crate::claude_ui::UiEvent> for UiRuntimeEvent {
    fn from(old: crate::claude_ui::UiEvent) -> Self {
        match old {
            crate::claude_ui::UiEvent::TurnStarted => UiRuntimeEvent::TurnStarted,
            crate::claude_ui::UiEvent::UserSubmitted(s) => UiRuntimeEvent::UserSubmitted(s),
            crate::claude_ui::UiEvent::ThinkingStarted => UiRuntimeEvent::ThinkingStarted,
            crate::claude_ui::UiEvent::ThinkingDelta(s) => UiRuntimeEvent::ThinkingDelta(s),
            crate::claude_ui::UiEvent::ThinkingFinished => UiRuntimeEvent::ThinkingFinished,
            crate::claude_ui::UiEvent::AssistantContentDelta(s) => {
                UiRuntimeEvent::AssistantContentDelta(s)
            }
            crate::claude_ui::UiEvent::AssistantFinished => {
                UiRuntimeEvent::AssistantContentFinished
            }
            crate::claude_ui::UiEvent::ToolStarted { name, command } => {
                UiRuntimeEvent::ToolStarted { name, command }
            }
            crate::claude_ui::UiEvent::ToolProgress { name, message } => {
                UiRuntimeEvent::ToolProgress { name, message }
            }
            crate::claude_ui::UiEvent::ToolFinished {
                name,
                success,
                output,
            } => {
                if success {
                    UiRuntimeEvent::ToolSucceeded {
                        name,
                        output,
                        duration_ms: None,
                    }
                } else {
                    UiRuntimeEvent::ToolFailed {
                        name,
                        output,
                        duration_ms: None,
                    }
                }
            }
            crate::claude_ui::UiEvent::PermissionRequested { command } => {
                UiRuntimeEvent::PermissionRequested {
                    command,
                    reason: None,
                }
            }
            crate::claude_ui::UiEvent::PermissionResolved { command, approved } => {
                UiRuntimeEvent::PermissionResolved { command, approved }
            }
            crate::claude_ui::UiEvent::TasksUpdated => {
                // TasksUpdated doesn't map directly; emit a notice
                UiRuntimeEvent::SystemNotice {
                    message: "tasks updated".into(),
                    level: "info".into(),
                }
            }
            crate::claude_ui::UiEvent::CompactBoundary => UiRuntimeEvent::CompactBoundary,
            crate::claude_ui::UiEvent::StatusUpdated { model, ctx_tokens } => {
                UiRuntimeEvent::FooterModelUpdated { model }
            }
            crate::claude_ui::UiEvent::Notification { message, level } => {
                UiRuntimeEvent::SystemNotice { message, level }
            }
            crate::claude_ui::UiEvent::InputChanged(s) => UiRuntimeEvent::InputChanged(s),
            crate::claude_ui::UiEvent::ModeChanged(s) => UiRuntimeEvent::InputModeChanged(s),
            crate::claude_ui::UiEvent::Resize { cols, rows } => {
                UiRuntimeEvent::Resize { cols, rows }
            }
            crate::claude_ui::UiEvent::ExitRequested => UiRuntimeEvent::ExitRequested,
            crate::claude_ui::UiEvent::RecipeLoaded { .. } => UiRuntimeEvent::SystemNotice {
                message: "recipe loaded".into(),
                level: "info".into(),
            },
            crate::claude_ui::UiEvent::RecipeStageStarted { .. } => UiRuntimeEvent::SystemNotice {
                message: "recipe stage started".into(),
                level: "info".into(),
            },
            crate::claude_ui::UiEvent::RecipeStageComplete { .. } => UiRuntimeEvent::SystemNotice {
                message: "recipe stage complete".into(),
                level: "info".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_label_is_compact() {
        let event = UiRuntimeEvent::TurnStarted;
        let label = event.event_label();
        assert!(label.len() < 24, "label should be compact: {label}");
    }

    #[test]
    fn test_convert_from_old_ui_event() {
        let old = crate::claude_ui::UiEvent::TurnStarted;
        let canonical: UiRuntimeEvent = old.into();
        assert!(matches!(canonical, UiRuntimeEvent::TurnStarted));

        let old = crate::claude_ui::UiEvent::ToolStarted {
            name: "bash".into(),
            command: "ls".into(),
        };
        let canonical: UiRuntimeEvent = old.into();
        assert!(matches!(
            canonical,
            UiRuntimeEvent::ToolStarted { name, .. } if name == "bash"
        ));
    }

    #[test]
    fn test_tool_finished_maps_to_success_or_failure() {
        let old = crate::claude_ui::UiEvent::ToolFinished {
            name: "read".into(),
            success: true,
            output: "content".into(),
        };
        let canonical: UiRuntimeEvent = old.into();
        assert!(
            matches!(&canonical, UiRuntimeEvent::ToolSucceeded { name, .. } if name == "read"),
            "successful tool should map to ToolSucceeded"
        );

        let old = crate::claude_ui::UiEvent::ToolFinished {
            name: "bash".into(),
            success: false,
            output: "error".into(),
        };
        let canonical: UiRuntimeEvent = old.into();
        assert!(
            matches!(&canonical, UiRuntimeEvent::ToolFailed { name, .. } if name == "bash"),
            "failed tool should map to ToolFailed"
        );
    }
}
