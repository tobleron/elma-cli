//! @efficiency-role: infra-adapter
//!
//! Session Persistence Adapter — subscribes to UI runtime events and writes
//! session artifacts (terminal transcript + session markdown).
//!
//! Task 635: Extracted from ClaudeRenderer::push_message and
//! ClaudeRenderer::flush_transcript_buffer.  This adapter owns no view state;
//! it only persists to disk.

use crate::claude_ui::{ClaudeMessage, UiNoticeKind};
use crate::session_write;
use crate::ui_runtime_event::UiRuntimeEvent;
use std::path::PathBuf;

/// Buffered session writer that flushes on a cadence.
#[derive(Clone, Debug)]
pub(crate) struct SessionPersistenceAdapter {
    session_root: Option<PathBuf>,
    transcript_buffer: Vec<String>,
    markdown_buffer: Vec<session_write::MdEntry>,
    last_flush: std::time::Instant,
}

impl SessionPersistenceAdapter {
    pub(crate) fn new(session_root: Option<PathBuf>) -> Self {
        Self {
            session_root,
            transcript_buffer: Vec::new(),
            markdown_buffer: Vec::new(),
            last_flush: std::time::Instant::now(),
        }
    }

    /// Update the session root (called on session init / resume).
    pub(crate) fn set_session_root(&mut self, path: Option<PathBuf>) {
        self.session_root = path;
    }

    /// Process an event, buffering persistence data.
    pub(crate) fn handle_event(&mut self, event: &UiRuntimeEvent) {
        let messages = crate::ui_reducer::event_to_claude_messages(event);
        for msg in &messages {
            let line = claude_message_to_terminal_line(msg);
            let entry = claude_message_to_md_entry(msg);
            self.transcript_buffer.push(line);
            if let Some(entry) = entry {
                self.markdown_buffer.push(entry);
            }
        }
        // Auto-flush every 10 events or 500ms
        if self.transcript_buffer.len() >= 10
            || self.last_flush.elapsed() >= std::time::Duration::from_millis(500)
        {
            self.flush();
        }
    }

    /// Force flush all buffered data to disk.
    pub(crate) fn flush(&mut self) {
        if self.transcript_buffer.is_empty() {
            return;
        }
        if let Some(ref session_root) = self.session_root {
            let lines: Vec<String> = self.transcript_buffer.drain(..).collect();
            let entries: Vec<session_write::MdEntry> = self.markdown_buffer.drain(..).collect();
            for line in &lines {
                session_write::append_terminal_transcript(session_root, line);
            }
            for entry in &entries {
                session_write::append_session_markdown(session_root, &entry);
            }
        } else {
            self.transcript_buffer.clear();
            self.markdown_buffer.clear();
        }
        self.last_flush = std::time::Instant::now();
    }
}

/// Convert a ClaudeMessage to a terminal transcript line (matching the old
/// claude_message_to_transcript_line behavior).
fn claude_message_to_terminal_line(msg: &ClaudeMessage) -> String {
    match msg {
        ClaudeMessage::User { content } => {
            format!("> {}", content.lines().next().unwrap_or(""))
        }
        ClaudeMessage::Assistant { content, .. } => {
            let first = content.raw_markdown.lines().next().unwrap_or("");
            format!("● {}", first)
        }
        ClaudeMessage::ToolStart { name, .. } => {
            format!("▸ {} started", name)
        }
        ClaudeMessage::ToolTrace { name, status, .. } => match status {
            crate::claude_ui::ToolTraceStatus::Running => {
                format!("▸ {} running", name)
            }
            crate::claude_ui::ToolTraceStatus::Completed { success, .. } => {
                if *success {
                    format!("✓ {}", name)
                } else {
                    format!("✗ {}", name)
                }
            }
        },
        ClaudeMessage::ToolProgress { name, message } => {
            format!("  {}: {}", name, message)
        }
        ClaudeMessage::ToolResult { name, success, .. } => {
            if *success {
                format!("✓ {}", name)
            } else {
                format!("✗ {}", name)
            }
        }
        ClaudeMessage::PermissionRequest { command, .. } => {
            format!("🔒 Permission: {}", command)
        }
        ClaudeMessage::Thinking { .. } => String::new(),
        ClaudeMessage::CompactBoundary => "✻ Conversation compacted".to_string(),
        ClaudeMessage::CompactSummary { message_count, .. } => {
            format!("✻ Compacted {} messages", message_count)
        }
        ClaudeMessage::System { content } => content.clone(),
        ClaudeMessage::Notice(notice) => {
            format!("[{}] {}", notice_label(&notice.kind), notice.content)
        }
    }
}

fn notice_label(kind: &UiNoticeKind) -> &'static str {
    match kind {
        UiNoticeKind::Budget => "Budget",
        UiNoticeKind::Queue => "Queue",
        UiNoticeKind::Compaction => "Compaction",
        UiNoticeKind::StopReason => "Stop",
        UiNoticeKind::InputHint => "Hint",
        UiNoticeKind::Session => "Session",
    }
}

/// Convert a ClaudeMessage to an MdEntry for session.md persistence.
fn claude_message_to_md_entry(msg: &ClaudeMessage) -> Option<session_write::MdEntry> {
    use session_write::MdEntry;
    match msg {
        ClaudeMessage::User { content } => Some(MdEntry::User {
            content: content.clone(),
        }),
        ClaudeMessage::Assistant { content, .. } => Some(MdEntry::Assistant {
            content: content.raw_markdown.clone(),
        }),
        ClaudeMessage::ToolStart { name, input } => Some(MdEntry::ToolStart {
            name: name.clone(),
            input: input.clone().unwrap_or_default(),
        }),
        ClaudeMessage::ToolTrace { name, status, .. } => match status {
            crate::claude_ui::ToolTraceStatus::Completed {
                success, output, ..
            } => Some(MdEntry::ToolResult {
                name: name.clone(),
                success: *success,
                output: output.clone(),
                duration_ms: None,
            }),
            _ => None,
        },
        ClaudeMessage::ToolResult {
            name,
            success,
            output,
            duration_ms,
        } => Some(MdEntry::ToolResult {
            name: name.clone(),
            success: *success,
            output: output.clone(),
            duration_ms: *duration_ms,
        }),
        ClaudeMessage::PermissionRequest { command, reason } => Some(MdEntry::Meta {
            label: "permission".into(),
            detail: format!("{} reason={:?}", command, reason),
        }),
        ClaudeMessage::Thinking { content, .. } => Some(MdEntry::Thinking {
            content: content.clone(),
        }),
        ClaudeMessage::CompactBoundary => Some(MdEntry::Meta {
            label: "compact".into(),
            detail: String::new(),
        }),
        ClaudeMessage::CompactSummary {
            message_count,
            context_preview,
        } => Some(MdEntry::Meta {
            label: "compact".into(),
            detail: format!("{} messages, preview={:?}", message_count, context_preview),
        }),
        ClaudeMessage::System { content } => Some(MdEntry::Meta {
            label: "system".into(),
            detail: content.clone(),
        }),
        ClaudeMessage::Notice(notice) => Some(MdEntry::Meta {
            label: "notice".into(),
            detail: format!("{:?} {}", notice.kind, notice.content),
        }),
        ClaudeMessage::ToolProgress { name, message } => Some(MdEntry::Meta {
            label: name.clone(),
            detail: message.clone(),
        }),
    }
}
