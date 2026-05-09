//! @efficiency-role: test-support
//!
//! UiSnapshotHarness — deterministic UI snapshot/regression capture.
//!
//! This module provides a simplified renderer that produces plain-text
//! lines from UiViewState, enabling snapshot testing of UI state
//! transformations without a terminal or ratatui dependency at test time.
//!
//! Task 639: UI snapshot harness for regression detection.

use crate::claude_ui::{AssistantBlock, AssistantContent, ClaudeMessage, ToolTraceStatus};
use crate::ui_view_state::{
    ActiveToolTrace, FooterState, ThinkingEntry, ToolTraceViewStatus, UiViewState,
};
use std::path::PathBuf;
use std::time::Instant;

/// A named snapshot fixture: input state + expected output lines.
#[derive(Clone, Debug)]
pub(crate) struct UiSnapshotFixture {
    pub name: &'static str,
    pub view_state: UiViewState,
    pub expected_lines: Vec<&'static str>,
}

/// Harness for rendering, comparing, and generating UI snapshots.
#[derive(Clone, Debug, Default)]
pub(crate) struct UiSnapshotHarness;

impl UiSnapshotHarness {
    /// Render a view state into plain text lines (no TTY, no ratatui).
    pub(crate) fn render(&self, state: &UiViewState) -> Vec<String> {
        render_fixture_lines(state)
    }

    /// Compare rendered output against stored expected lines.
    /// Returns `None` if they match, or `Some(diff_lines)` describing mismatches.
    pub(crate) fn compare(&self, rendered: &[String], expected: &[String]) -> Option<Vec<String>> {
        let mut diffs = Vec::new();
        let max_len = rendered.len().max(expected.len());

        for i in 0..max_len {
            match (rendered.get(i), expected.get(i)) {
                (Some(r), Some(e)) if r != e => {
                    diffs.push(format!(
                        "line {}:\n  expected: {:?}\n  got:      {:?}",
                        i, e, r
                    ));
                }
                (Some(r), None) => {
                    diffs.push(format!("line {}: extra output {:?}", i, r));
                }
                (None, Some(e)) => {
                    diffs.push(format!("line {}: missing expected {:?}", i, e));
                }
                _ => {}
            }
        }

        if diffs.is_empty() {
            None
        } else {
            Some(diffs)
        }
    }

    /// Generate a snapshot file content (lines joined by newline).
    pub(crate) fn generate_snapshot(&self, state: &UiViewState) -> String {
        render_fixture_lines(state).join("\n")
    }

    /// Write a new snapshot to disk under `_testing_reports/ui_snapshots/`.
    pub(crate) fn write_snapshot(&self, name: &str, state: &UiViewState) -> std::io::Result<()> {
        let path = snapshot_path(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = self.generate_snapshot(state);
        std::fs::write(&path, content)?;
        Ok(())
    }
}

/// Render a UiViewState into plain text lines.
///
/// Produces a simplified text representation of what a user would see:
/// transcript messages, streaming text, thinking entries, input area, and footer.
pub(crate) fn render_fixture_lines(state: &UiViewState) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // ── Transcript ──────────────────────────────────────────────────────
    for msg in &state.transcript.messages {
        match msg {
            ClaudeMessage::User { content } => {
                lines.push(format!("> {}", content));
            }
            ClaudeMessage::Assistant { content, .. } => {
                let text = render_assistant_content_plain(content);
                for line in text.lines() {
                    lines.push(line.to_string());
                }
            }
            ClaudeMessage::ToolStart { name, input } => {
                let input_str = input.as_deref().unwrap_or("");
                lines.push(format!("▸ {} {}", name, input_str));
            }
            ClaudeMessage::ToolProgress { name, message } => {
                lines.push(format!("  {} … {}", name, message));
            }
            ClaudeMessage::ToolResult {
                name,
                success,
                output,
                ..
            } => {
                let icon = if *success { "✓" } else { "✗" };
                lines.push(format!("{} {}", icon, name));
                if !output.is_empty() {
                    for out_line in output.lines() {
                        lines.push(format!("  │ {}", out_line));
                    }
                }
            }
            ClaudeMessage::ToolTrace {
                name: _name,
                command,
                status,
                collapsed,
            } => {
                let status_char = match status {
                    ToolTraceStatus::Running => "◌",
                    ToolTraceStatus::Completed { success: true, .. } => "✓",
                    ToolTraceStatus::Completed { success: false, .. } => "✗",
                };
                let collapse = if *collapsed { " [collapsed]" } else { "" };
                lines.push(format!("{} {}{}", status_char, command, collapse));
                if let ToolTraceStatus::Completed { output, .. } = status {
                    if !output.is_empty() && !collapsed {
                        for out_line in output.lines() {
                            lines.push(format!("  │ {}", out_line));
                        }
                    }
                }
            }
            ClaudeMessage::Thinking {
                content,
                is_streaming,
                ..
            } => {
                if *is_streaming {
                    lines.push("∴ Thinking…".to_string());
                } else {
                    lines.push("∴ Thought".to_string());
                    for line in content.lines() {
                        lines.push(format!("  {}", line));
                    }
                }
            }
            ClaudeMessage::CompactBoundary => {
                lines.push("✻ Conversation compacted".to_string());
            }
            ClaudeMessage::CompactSummary {
                message_count,
                context_preview,
            } => {
                let preview = context_preview.as_deref().unwrap_or("");
                lines.push(format!(
                    "✻ Compacted {} messages{}",
                    message_count,
                    if preview.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", preview)
                    }
                ));
            }
            ClaudeMessage::System { content } => {
                for line in content.lines() {
                    lines.push(format!("# {}", line));
                }
            }
            ClaudeMessage::Notice(notice) => {
                lines.push(format!("! {}", notice.content));
            }
            ClaudeMessage::PermissionRequest { command, reason } => {
                let reason_str = reason.as_deref().unwrap_or("permission required");
                lines.push(format!("? {} ({})", command, reason_str));
            }
        }
    }

    // ── Streaming text ──────────────────────────────────────────────────
    if !state.streaming_text.is_empty() {
        for line in state.streaming_text.lines() {
            lines.push(line.to_string());
        }
    }
    if state.is_streaming_thinking && !state.streaming_thought.is_empty() {
        lines.push("∴ Thinking…".to_string());
    }

    // ── Thinking panel entries ─────────────────────────────────────────
    for entry in &state.thinking_entries {
        let status = if entry.collapsed { " [collapsed]" } else { "" };
        lines.push(format!(
            "∴ {} words{}{}",
            entry.word_count,
            status,
            if entry.is_summary { " [summary]" } else { "" }
        ));
        if !entry.collapsed {
            for line in entry.content.lines() {
                lines.push(format!("  {}", line));
            }
        }
    }

    // ── Tool traces ──────────────────────────────────────────────────────
    for trace in &state.active_tool_traces {
        let status_icon = match &trace.status {
            ToolTraceViewStatus::Pending => "◌",
            ToolTraceViewStatus::Running => "◌",
            ToolTraceViewStatus::Succeeded { .. } => "✓",
            ToolTraceViewStatus::Failed { .. } => "✗",
            ToolTraceViewStatus::Denied { .. } => "!",
            ToolTraceViewStatus::Cancelled => "—",
            ToolTraceViewStatus::TimedOut => "⏱",
        };
        let collapse = if trace.collapsed { " [collapsed]" } else { "" };
        lines.push(format!("{} {}{}", status_icon, trace.name, collapse));
        if let ToolTraceViewStatus::Succeeded { output, .. }
        | ToolTraceViewStatus::Failed { output, .. } = &trace.status
        {
            if !output.is_empty() && !trace.collapsed {
                for out_line in output.lines() {
                    lines.push(format!("  │ {}", out_line));
                }
            }
        }
    }

    // ── Input area ───────────────────────────────────────────────────────
    if state.input_lines.is_empty() || state.input_lines.iter().all(|l| l.is_empty()) {
        lines.push("> ".to_string());
    } else {
        for input_line in &state.input_lines {
            lines.push(format!("> {}", input_line));
        }
    }

    // ── Footer ───────────────────────────────────────────────────────────
    let model = state.footer.model_label.as_deref().unwrap_or("no-model");
    let ctx_pct = state.context_pct();
    lines.push(format!(
        "{} │ in:{} out:{} ctx:{}% ({}/{}) {}s",
        model,
        state.footer.input_tokens,
        state.footer.output_tokens,
        ctx_pct,
        state.footer.context_current,
        state.footer.context_max,
        state.footer.elapsed_secs,
    ));

    lines
}

/// Path for a named snapshot under `_testing_reports/ui_snapshots/`.
pub(crate) fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from("_testing_reports")
        .join("ui_snapshots")
        .join(format!("{}.snap", name))
}

// ─── Plain-text assistant content extraction ───────────────────────────────

fn render_assistant_content_plain(content: &AssistantContent) -> String {
    let mut out = String::new();
    for block in &content.blocks {
        match block {
            AssistantBlock::Paragraph(text) => {
                out.push_str(text);
                out.push('\n');
            }
            AssistantBlock::List(text) => {
                out.push_str(text);
                out.push('\n');
            }
            AssistantBlock::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    out.push_str(&format!("```{}\n", lang));
                } else {
                    out.push_str("```\n");
                }
                out.push_str(code);
                out.push('\n');
                out.push_str("```\n");
            }
            AssistantBlock::CommandSuggestion { language, commands } => {
                out.push_str(&format!("```{}\n", language));
                for cmd in commands {
                    out.push_str(cmd);
                    out.push('\n');
                }
                out.push_str("```\n");
            }
            AssistantBlock::Table(text) => {
                out.push_str(text);
                out.push('\n');
            }
            AssistantBlock::Rule => {
                out.push_str("---\n");
            }
            AssistantBlock::Callout(text) => {
                out.push_str(&format!("> {}\n", text));
            }
        }
    }
    out.trim().to_string()
}

// ============================================================================
// Test fixtures
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_ui::ClaudeMessage;

    /// Fixture: startup — empty transcript, one empty input line.
    #[test]
    fn fixture_startup() {
        let state = UiViewState::default();
        let fixture = UiSnapshotFixture {
            name: "startup",
            view_state: state,
            expected_lines: vec!["> ", "no-model │ in:0 out:0 ctx:0% (0/0) 0s"],
        };

        let harness = UiSnapshotHarness::default();
        let rendered = harness.render(&fixture.view_state);
        assert_eq!(
            rendered, fixture.expected_lines,
            "snapshot mismatch for fixture '{}'",
            fixture.name
        );
    }

    /// Fixture: user message in transcript.
    #[test]
    fn fixture_user_message() {
        let mut transcript = crate::claude_ui::ClaudeTranscript::new();
        transcript.push(ClaudeMessage::User {
            content: "What files are in the current directory?".to_string(),
        });

        let state = UiViewState {
            transcript,
            ..UiViewState::default()
        };

        let fixture = UiSnapshotFixture {
            name: "user_message",
            view_state: state,
            expected_lines: vec![
                "> What files are in the current directory?",
                "> ",
                "no-model │ in:0 out:0 ctx:0% (0/0) 0s",
            ],
        };

        let harness = UiSnapshotHarness::default();
        let rendered = harness.render(&fixture.view_state);
        assert_eq!(
            rendered, fixture.expected_lines,
            "snapshot mismatch for fixture '{}'",
            fixture.name
        );
    }

    /// Fixture: assistant streaming response.
    #[test]
    fn fixture_assistant_streaming() {
        let mut transcript = crate::claude_ui::ClaudeTranscript::new();
        transcript.push(ClaudeMessage::User {
            content: "List files".to_string(),
        });
        transcript.push(ClaudeMessage::Assistant { ephemeral_deadline: None,
            content: AssistantContent {
                raw_markdown: "Here are the files:\n\n- README.md\n- main.rs".to_string(),
                blocks: vec![
                    AssistantBlock::Paragraph("Here are the files:".to_string()),
                    AssistantBlock::List("- README.md\n- main.rs".to_string()),
                ],
                precomputed_blocks: None,
            },
        });

        let state = UiViewState {
            transcript,
            streaming_text: " streaming more...".to_string(),
            is_streaming_content: true,
            ..UiViewState::default()
        };

        let fixture = UiSnapshotFixture {
            name: "assistant_streaming",
            view_state: state,
            expected_lines: vec![
                "> List files",
                "Here are the files:",
                "- README.md",
                "- main.rs",
                " streaming more...",
                "> ",
                "no-model │ in:0 out:0 ctx:0% (0/0) 0s",
            ],
        };

        let harness = UiSnapshotHarness::default();
        let rendered = harness.render(&fixture.view_state);
        assert_eq!(
            rendered, fixture.expected_lines,
            "snapshot mismatch for fixture '{}'",
            fixture.name
        );
    }

    /// Fixture: tool lifecycle — start → progress → success.
    #[test]
    fn fixture_tool_lifecycle() {
        let mut transcript = crate::claude_ui::ClaudeTranscript::new();
        transcript.push(ClaudeMessage::User {
            content: "Run tests".to_string(),
        });
        transcript.push(ClaudeMessage::ToolTrace {
            name: "bash".to_string(),
            command: "cargo test".to_string(),
            status: ToolTraceStatus::Completed {
                success: true,
                output: "test result: ok. 42 passed".to_string(),
                duration_ms: Some(1200),
            },
            collapsed: false,
        });

        let state = UiViewState {
            transcript,
            ..UiViewState::default()
        };

        let fixture = UiSnapshotFixture {
            name: "tool_lifecycle",
            view_state: state,
            expected_lines: vec![
                "> Run tests",
                "✓ cargo test",
                "  │ test result: ok. 42 passed",
                "> ",
                "no-model │ in:0 out:0 ctx:0% (0/0) 0s",
            ],
        };

        let harness = UiSnapshotHarness::default();
        let rendered = harness.render(&fixture.view_state);
        assert_eq!(
            rendered, fixture.expected_lines,
            "snapshot mismatch for fixture '{}'",
            fixture.name
        );
    }

    /// Fixture: thinking panel entry.
    #[test]
    fn fixture_thinking_panel() {
        let now = Instant::now();
        let entry = ThinkingEntry {
            content: "The user wants to list files in the current directory. \
                       I should use the glob tool to search for relevant files."
                .to_string(),
            word_count: 22,
            created_at: now,
            collapse_deadline: now + std::time::Duration::from_secs(10),
            collapsed: false,
            reveal_chars: 120,
            is_summary: false,
        };

        let state = UiViewState {
            thinking_entries: vec![entry],
            ..UiViewState::default()
        };

        let fixture = UiSnapshotFixture {
            name: "thinking_panel",
            view_state: state,
            expected_lines: vec![
                "∴ 22 words",
                "  The user wants to list files in the current directory. I should use the glob tool to search for relevant files.",
                "> ",
                "no-model │ in:0 out:0 ctx:0% (0/0) 0s",
            ],
        };

        let harness = UiSnapshotHarness::default();
        let rendered = harness.render(&fixture.view_state);
        assert_eq!(
            rendered, fixture.expected_lines,
            "snapshot mismatch for fixture '{}'",
            fixture.name
        );
    }

    /// Fixture: footer with model/token info.
    #[test]
    fn fixture_footer() {
        let state = UiViewState {
            footer: FooterState {
                model_label: Some("claude-3-5-sonnet-20241022".to_string()),
                input_tokens: 142,
                output_tokens: 89,
                context_current: 45000,
                context_max: 200000,
                elapsed_secs: 37,
            },
            ..UiViewState::default()
        };

        let fixture = UiSnapshotFixture {
            name: "footer",
            view_state: state,
            expected_lines: vec![
                "> ",
                "claude-3-5-sonnet-20241022 │ in:142 out:89 ctx:22% (45000/200000) 37s",
            ],
        };

        let harness = UiSnapshotHarness::default();
        let rendered = harness.render(&fixture.view_state);
        assert_eq!(
            rendered, fixture.expected_lines,
            "snapshot mismatch for fixture '{}'",
            fixture.name
        );
    }
}
