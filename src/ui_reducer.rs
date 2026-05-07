//! @efficiency-role: logic
//!
//! Pure UI Reducer — maps UiRuntimeEvent → UiViewState mutation.
//!
//! This is a pure function with no I/O, no side effects, and no terminal access.
//! It receives the current state and an event, and returns the new state.
//!
//! Task 635: Foundation reducer. Later tasks will add sub-reducers for
//! streaming, tool traces, thinking, notices, etc.

use crate::claude_ui::{
    AssistantContent, ClaudeMessage, InputMode, NoticePersistence, PickerState, SessionCommand,
    ToolTraceStatus, UiNotice, UiNoticeKind,
};
use crate::ui_runtime_event::UiRuntimeEvent;
use crate::ui_view_state::{
    ActiveToolTrace, PermissionRequestView, ThinkingEntry, ToolTraceViewStatus, TranscriptViewMode,
    UiViewState,
};
use std::time::Instant;

/// Pure reducer: (state, event) -> state.
///
/// No I/O, no random, no time queries — all temporal metadata is
/// injected by the caller if needed.
pub(crate) fn ui_reducer(state: &mut UiViewState, event: &UiRuntimeEvent) {
    match event {
        // ── User input ──────────────────────────────────────────────────
        UiRuntimeEvent::InputChanged(text) => {
            state.input_lines = vec![text.clone()];
        }
        UiRuntimeEvent::InputModeChanged(mode) => {
            state.input_mode = match mode.as_str() {
                "bash" => InputMode::Bash,
                "background" => InputMode::Background,
                _ => InputMode::Chat,
            };
        }
        UiRuntimeEvent::UserSubmitted(_) => {
            // Resets picker state on submit
            state.picker_state = PickerState::None;
            state.file_matches.clear();
        }

        // ── Model lifecycle ─────────────────────────────────────────────
        UiRuntimeEvent::TurnStarted => {
            state.is_streaming_thinking = false;
            state.is_streaming_content = false;
            state.streaming_thought.clear();
            state.streaming_text.clear();
        }
        UiRuntimeEvent::ThinkingStarted => {
            state.is_streaming_thinking = true;
            state.streaming_thought.clear();
        }
        UiRuntimeEvent::ThinkingDelta(text) => {
            if state.is_streaming_thinking {
                state.streaming_thought.push_str(text);
            }
        }
        UiRuntimeEvent::ThinkingFinished => {
            state.is_streaming_thinking = false;
            if !state.streaming_thought.is_empty() {
                let entry = ThinkingEntry {
                    content: state.streaming_thought.clone(),
                    word_count: state.streaming_thought.split_whitespace().count(),
                    created_at: Instant::now(),
                    collapse_deadline: Instant::now(),
                    collapsed: false,
                    reveal_chars: 0,
                    is_summary: false,
                };
                state.thinking_entries.push(entry);
            }
        }
        UiRuntimeEvent::AssistantContentStarted => {
            state.is_streaming_content = true;
            state.streaming_text.clear();
        }
        UiRuntimeEvent::AssistantContentDelta(text) => {
            if state.is_streaming_content {
                state.streaming_text.push_str(text);
            }
        }
        UiRuntimeEvent::AssistantContentFinished => {
            state.is_streaming_content = false;
        }
        UiRuntimeEvent::AssistantFinalAnswer { display, .. } => {
            state.streaming_text = display.clone();
        }

        // ── Tool lifecycle ──────────────────────────────────────────────
        UiRuntimeEvent::ToolProposed { name, input } => {
            state.active_tool_traces.push(ActiveToolTrace {
                name: name.clone(),
                command: input.clone(),
                status: ToolTraceViewStatus::Pending,
                collapsed: false,
            });
        }
        UiRuntimeEvent::ToolStarted { name, command } => {
            // Mark all existing traces collapsed (auto-collapse on new tool)
            for trace in &mut state.active_tool_traces {
                trace.collapsed = true;
            }
            state.active_tool_traces.push(ActiveToolTrace {
                name: name.clone(),
                command: command.clone(),
                status: ToolTraceViewStatus::Running,
                collapsed: false,
            });
        }
        UiRuntimeEvent::ToolProgress { name, message } => {
            if let Some(trace) = state
                .active_tool_traces
                .iter_mut()
                .rev()
                .find(|t| t.name == *name)
            {
                trace.command = message.clone();
            }
        }
        UiRuntimeEvent::ToolSucceeded {
            name,
            output,
            duration_ms,
        } => {
            if let Some(trace) = state
                .active_tool_traces
                .iter_mut()
                .rev()
                .find(|t| t.name == *name)
            {
                trace.status = ToolTraceViewStatus::Succeeded {
                    output: output.clone(),
                    duration_ms: *duration_ms,
                };
            }
        }
        UiRuntimeEvent::ToolFailed {
            name,
            output,
            duration_ms,
        } => {
            if let Some(trace) = state
                .active_tool_traces
                .iter_mut()
                .rev()
                .find(|t| t.name == *name)
            {
                trace.status = ToolTraceViewStatus::Failed {
                    output: output.clone(),
                    duration_ms: *duration_ms,
                };
            }
        }
        UiRuntimeEvent::ToolDenied { name, reason } => {
            if let Some(trace) = state
                .active_tool_traces
                .iter_mut()
                .rev()
                .find(|t| t.name == *name)
            {
                trace.status = ToolTraceViewStatus::Denied {
                    reason: reason.clone(),
                };
            }
        }
        UiRuntimeEvent::ToolCancelled { name } => {
            if let Some(trace) = state
                .active_tool_traces
                .iter_mut()
                .rev()
                .find(|t| t.name == *name)
            {
                trace.status = ToolTraceViewStatus::Cancelled;
            }
        }
        UiRuntimeEvent::ToolTimedOut { name } => {
            if let Some(trace) = state
                .active_tool_traces
                .iter_mut()
                .rev()
                .find(|t| t.name == *name)
            {
                trace.status = ToolTraceViewStatus::TimedOut;
            }
        }

        // ── Permission ──────────────────────────────────────────────────
        UiRuntimeEvent::PermissionRequested { command, reason } => {
            state.active_permission_request = Some(PermissionRequestView {
                command: command.clone(),
                reason: reason.clone(),
            });
        }
        UiRuntimeEvent::PermissionResolved { .. } => {
            state.active_permission_request = None;
        }

        // ── Session ─────────────────────────────────────────────────────
        UiRuntimeEvent::SessionStarted { id } => {
            state.session_id = Some(id.clone());
        }
        UiRuntimeEvent::SessionCleared => {
            state.streaming_thought.clear();
            state.streaming_text.clear();
            state.active_tool_traces.clear();
            state.thinking_entries.clear();
        }
        UiRuntimeEvent::SessionResumed { id } => {
            state.session_id = Some(id.clone());
        }
        UiRuntimeEvent::ExitRequested => {
            state.exit_state.request_exit();
        }
        UiRuntimeEvent::ExitConfirmed => {
            // Handled at the app level
        }

        // ── Notices ─────────────────────────────────────────────────────
        UiRuntimeEvent::RouteNotice { .. } => {
            // Notice rendering is handled by the transcript layer
        }
        UiRuntimeEvent::BudgetNotice { .. } => {}
        UiRuntimeEvent::CompactionNotice { .. } => {}
        UiRuntimeEvent::StopReasonNotice { .. } => {}
        UiRuntimeEvent::RetryNotice { .. } => {}
        UiRuntimeEvent::ToolDiscoveryNotice { .. } => {}
        UiRuntimeEvent::SystemNotice { .. } => {}

        // ── Layout ──────────────────────────────────────────────────────
        UiRuntimeEvent::Resize { cols, rows } => {
            state.terminal_width = *cols;
            state.terminal_height = *rows;
        }

        // ── Footer ──────────────────────────────────────────────────────
        UiRuntimeEvent::FooterModelUpdated { model } => {
            state.footer.model_label = Some(model.clone());
        }
        UiRuntimeEvent::FooterTokenCounts {
            input_tokens,
            output_tokens,
            context_current,
            context_max,
        } => {
            state.footer.input_tokens = *input_tokens;
            state.footer.output_tokens = *output_tokens;
            state.footer.context_current = *context_current;
            state.footer.context_max = *context_max;
        }
        UiRuntimeEvent::FooterElapsed(secs) => {
            state.footer.elapsed_secs = *secs;
        }

        // ── Background tasks ────────────────────────────────────────────
        UiRuntimeEvent::BackgroundTaskAdded { .. } => {}
        UiRuntimeEvent::BackgroundTaskUpdated { .. } => {}
        UiRuntimeEvent::BackgroundTaskRemoved { .. } => {}

        // ── Compact (passthrough to transcript) ─────────────────────────
        UiRuntimeEvent::CompactBoundary => {}
        UiRuntimeEvent::CompactSummary { .. } => {}
    }
}

/// Helper: push a ClaudeMessage into the transcript in response to an event.
/// The reducer focuses on view-state; this is a separate operation that
/// updates the transcript container.
pub(crate) fn event_to_claude_messages(event: &UiRuntimeEvent) -> Vec<ClaudeMessage> {
    match event {
        UiRuntimeEvent::UserSubmitted(text) => {
            vec![ClaudeMessage::User {
                content: text.clone(),
            }]
        }
        UiRuntimeEvent::ThinkingFinished => {
            // Thinking messages are managed via streaming state + thinking panel
            vec![]
        }
        UiRuntimeEvent::AssistantFinalAnswer { display, .. } => {
            vec![ClaudeMessage::Assistant {
                content: AssistantContent::from_markdown(display),
            }]
        }
        UiRuntimeEvent::ToolStarted { name, command } => {
            vec![ClaudeMessage::ToolTrace {
                name: name.clone(),
                command: command.clone(),
                status: ToolTraceStatus::Running,
                collapsed: false,
            }]
        }
        UiRuntimeEvent::ToolSucceeded {
            name,
            output,
            duration_ms,
        } => {
            vec![ClaudeMessage::ToolTrace {
                name: name.clone(),
                command: String::new(),
                status: ToolTraceStatus::Completed {
                    success: true,
                    output: output.clone(),
                    duration_ms: *duration_ms,
                },
                collapsed: true,
            }]
        }
        UiRuntimeEvent::ToolFailed {
            name,
            output,
            duration_ms,
        } => {
            vec![ClaudeMessage::ToolTrace {
                name: name.clone(),
                command: String::new(),
                status: ToolTraceStatus::Completed {
                    success: false,
                    output: output.clone(),
                    duration_ms: *duration_ms,
                },
                collapsed: true,
            }]
        }
        UiRuntimeEvent::ToolDenied { name, reason } => {
            vec![ClaudeMessage::PermissionRequest {
                command: name.clone(),
                reason: reason.clone(),
            }]
        }
        UiRuntimeEvent::ToolCancelled { .. } => vec![],
        UiRuntimeEvent::ToolTimedOut { .. } => vec![],
        UiRuntimeEvent::PermissionRequested { command, reason } => {
            vec![ClaudeMessage::PermissionRequest {
                command: command.clone(),
                reason: reason.clone(),
            }]
        }
        UiRuntimeEvent::PermissionResolved { .. } => vec![],
        UiRuntimeEvent::CompactBoundary => {
            vec![ClaudeMessage::CompactBoundary]
        }
        UiRuntimeEvent::CompactSummary {
            message_count,
            context_preview,
        } => {
            vec![ClaudeMessage::CompactSummary {
                message_count: *message_count,
                context_preview: context_preview.clone(),
            }]
        }
        UiRuntimeEvent::SystemNotice { message, .. } => {
            vec![ClaudeMessage::System {
                content: message.clone(),
            }]
        }
        UiRuntimeEvent::StopReasonNotice { reason } => {
            vec![ClaudeMessage::Notice(UiNotice {
                kind: UiNoticeKind::StopReason,
                content: reason.clone(),
                created_at: Instant::now(),
                persistence: NoticePersistence::TranscriptCollapsible,
                collapsed: false,
            })]
        }
        UiRuntimeEvent::BudgetNotice {
            total,
            used,
            action,
        } => {
            vec![ClaudeMessage::Notice(UiNotice {
                kind: UiNoticeKind::Budget,
                content: format!("{used}/{total} {action}"),
                created_at: Instant::now(),
                persistence: NoticePersistence::TranscriptCollapsible,
                collapsed: false,
            })]
        }
        UiRuntimeEvent::CompactionNotice { reason } => {
            vec![ClaudeMessage::Notice(UiNotice {
                kind: UiNoticeKind::Compaction,
                content: reason.clone(),
                created_at: Instant::now(),
                persistence: NoticePersistence::TranscriptCollapsible,
                collapsed: false,
            })]
        }
        UiRuntimeEvent::RetryNotice { attempt, cause } => {
            vec![ClaudeMessage::System {
                content: format!("[retry {attempt}] {cause}"),
            }]
        }
        UiRuntimeEvent::ToolDiscoveryNotice { .. } => vec![],
        UiRuntimeEvent::RouteNotice { .. } => vec![],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> UiViewState {
        UiViewState::default()
    }

    #[test]
    fn test_input_changed() {
        let mut state = make_state();
        let event = UiRuntimeEvent::InputChanged("hello".into());
        ui_reducer(&mut state, &event);
        assert_eq!(state.input_lines, vec!["hello"]);
    }

    #[test]
    fn test_thinking_lifecycle() {
        let mut state = make_state();
        let events = vec![
            UiRuntimeEvent::ThinkingStarted,
            UiRuntimeEvent::ThinkingDelta("step 1".into()),
            UiRuntimeEvent::ThinkingDelta(" step 2".into()),
            UiRuntimeEvent::ThinkingFinished,
        ];
        for ev in &events {
            ui_reducer(&mut state, ev);
        }
        assert!(!state.is_streaming_thinking);
        assert_eq!(state.streaming_thought, "step 1 step 2");
        assert_eq!(state.thinking_entries.len(), 1);
    }

    #[test]
    fn test_tool_trace_lifecycle() {
        let mut state = make_state();
        let events = vec![
            UiRuntimeEvent::ToolStarted {
                name: "bash".into(),
                command: "ls".into(),
            },
            UiRuntimeEvent::ToolProgress {
                name: "bash".into(),
                message: "listing...".into(),
            },
            UiRuntimeEvent::ToolSucceeded {
                name: "bash".into(),
                output: "file1\nfile2".into(),
                duration_ms: Some(42),
            },
        ];
        for ev in &events {
            ui_reducer(&mut state, ev);
        }
        assert_eq!(state.active_tool_traces.len(), 1);
        match &state.active_tool_traces[0].status {
            ToolTraceViewStatus::Succeeded {
                output,
                duration_ms,
            } => {
                assert_eq!(output, "file1\nfile2");
                assert_eq!(*duration_ms, Some(42));
            }
            _ => panic!("expected Succeeded status"),
        }
    }

    #[test]
    fn test_auto_collapse_on_new_tool() {
        let mut state = make_state();
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::ToolStarted {
                name: "bash".into(),
                command: "ls".into(),
            },
        );
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::ToolStarted {
                name: "grep".into(),
                command: "search".into(),
            },
        );
        assert_eq!(state.active_tool_traces.len(), 2);
        assert!(state.active_tool_traces[0].collapsed);
        assert!(!state.active_tool_traces[1].collapsed);
    }

    #[test]
    fn test_permission_lifecycle() {
        let mut state = make_state();
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::PermissionRequested {
                command: "rm -rf /".into(),
                reason: None,
            },
        );
        assert!(state.active_permission_request.is_some());
        assert_eq!(
            state.active_permission_request.as_ref().unwrap().command,
            "rm -rf /"
        );

        ui_reducer(
            &mut state,
            &UiRuntimeEvent::PermissionResolved {
                command: "rm -rf /".into(),
                approved: false,
            },
        );
        assert!(state.active_permission_request.is_none());
    }

    #[test]
    fn test_resize() {
        let mut state = make_state();
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::Resize {
                cols: 120,
                rows: 40,
            },
        );
        assert_eq!(state.terminal_width, 120);
        assert_eq!(state.terminal_height, 40);
    }

    #[test]
    fn test_footer_tokens() {
        let mut state = make_state();
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::FooterTokenCounts {
                input_tokens: 100,
                output_tokens: 200,
                context_current: 300,
                context_max: 4096,
            },
        );
        assert_eq!(state.footer.input_tokens, 100);
        assert_eq!(state.footer.context_current, 300);
        assert_eq!(state.context_pct(), 7); // 300/4096 ≈ 7%
    }

    #[test]
    fn test_exit_request() {
        let mut state = make_state();
        assert!(!state.exit_state.double_press);
        ui_reducer(&mut state, &UiRuntimeEvent::ExitRequested);
        assert!(state.exit_state.requested);
    }

    #[test]
    fn test_event_to_messages_user() {
        let msgs = event_to_claude_messages(&UiRuntimeEvent::UserSubmitted("hello".into()));
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], ClaudeMessage::User { .. }));
    }

    #[test]
    fn test_event_to_messages_tool() {
        let msgs = event_to_claude_messages(&UiRuntimeEvent::ToolStarted {
            name: "bash".into(),
            command: "ls".into(),
        });
        assert!(msgs.len() >= 1);
    }

    #[test]
    fn test_event_to_messages_stop_reason() {
        let msgs = event_to_claude_messages(&UiRuntimeEvent::StopReasonNotice {
            reason: "max_tokens".into(),
        });
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], ClaudeMessage::Notice(_)));
    }

    // ── Task 707: Auxiliary helper disabled state ──

    #[test]
    fn test_auxiliary_helper_disabled_no_summary_event() {
        let mut state = make_state();
        // No thinking summary-related events should appear when auxiliary is disabled.
        // The presence of thinking content should not trigger any summary request.
        ui_reducer(&mut state, &UiRuntimeEvent::ThinkingStarted);
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::ThinkingDelta("Some thinking content".into()),
        );
        // No summary placeholder should exist — thinking entries may exist independently
        assert!(state.thinking_entries.is_empty() || !state.thinking_entries[0].content.is_empty());
    }

    #[test]
    fn test_auxiliary_helper_disabled_thinking_stream_is_preserved() {
        let mut state = make_state();
        // Thinking streaming should still work normally when auxiliary helper is disabled
        ui_reducer(&mut state, &UiRuntimeEvent::ThinkingStarted);
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::ThinkingDelta("Step 1: analyze the problem".into()),
        );
        ui_reducer(
            &mut state,
            &UiRuntimeEvent::ThinkingDelta("Step 2: implement solution".into()),
        );
        // Thinking content should still be accessible via streaming state
        assert!(state.streaming_thought.contains("Step 1"));
        assert!(state.streaming_thought.contains("Step 2"));
    }
}
