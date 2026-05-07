//! @efficiency-role: domain-logic
//!
//! Effective History Module
//!
//! Computes the effective message history for the next LLM call by:
//! - Excluding messages marked as `summarized = true`
//! - Injecting turn summaries as system messages at turn boundaries
//! - Task 768: Relevance and expiry — irrelevant artifacts, failed tools,
//!   and stale evidence can be removed from live context while staying in trace.
//!
//! This is the core of the deferred pre-turn summary system (Task 310).
//! It replaces raw turn messages with compact summaries to save context window.

use crate::intel_units::TurnSummaryOutput;
use crate::types_api::ChatMessage;

/// Task 768: Relevance label for evidence and tool messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relevance {
    /// Directly relevant to the current objective.
    Relevant,
    /// Related but not essential.
    Ancillary,
    /// Not relevant to the current objective — candidate for expiry.
    Irrelevant,
}

/// Task 768: Expire irrelevant messages from effective history.
/// Keeps trace/session artifacts complete but filters the live packet.
///
/// Messages are considered for expiry if:
/// - They are tool messages with no current-objective relevance
/// - They are failed tool calls that resolved into a different approach
/// - They reference artifacts or files not in the current scope
pub(crate) fn compute_effective_history_with_relevance(
    messages: &[ChatMessage],
    current_objective: &str,
    prior_turn_artifact_paths: &[String],
) -> Vec<ChatMessage> {
    let objective_lower = current_objective.to_lowercase();

    messages
        .iter()
        .filter(|m| {
            // Always keep non-summarized user/assistant messages
            if m.role == "user" || m.role == "system" {
                return !m.is_summarized();
            }

            // Assistant messages with tool calls: keep
            if m.role == "assistant" && m.tool_calls.is_some() {
                return !m.is_summarized();
            }

            // Tool messages: keep if relevant to current objective,
            // or if the tool name appears in current-objective context
            if m.role == "tool" {
                if m.is_summarized() {
                    return false;
                }

                // Check if the tool name appears in the objective
                if let Some(ref name) = m.name {
                    if objective_lower.contains(&name.to_lowercase()) {
                        return true;
                    }
                }

                // Check if the content references a current-scope path
                let content_lower = m.content.to_lowercase();
                if prior_turn_artifact_paths
                    .iter()
                    .any(|p| content_lower.contains(&p.to_lowercase()))
                {
                    return true;
                }

                // Keep tool results that contain error info (important for debugging)
                if content_lower.contains("error") || content_lower.contains("failed") {
                    return true;
                }

                // If the result references current-objective keywords, keep it
                let obj_words: Vec<&str> = objective_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 3)
                    .collect();
                if obj_words
                    .iter()
                    .any(|w| content_lower.contains(w))
                {
                    return true;
                }

                // Default: expire tool messages that don't match any relevance signal
                return false;
            }

            !m.is_summarized()
        })
        .cloned()
        .collect()
}

/// Compute the effective message history for the next LLM call.
/// Messages marked `summarized = true` are excluded from the result.
/// The remaining messages preserve their original order.
///
/// Task 768: Uses relevance filtering when current_objective is provided.
pub(crate) fn compute_effective_history(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|m| !m.is_summarized())
        .cloned()
        .collect()
}

/// Inject a turn summary as a system message into the message list.
/// The summary is inserted after the last message of the summarized turn.
pub(crate) fn inject_turn_summary(messages: &mut Vec<ChatMessage>, summary: &TurnSummaryOutput) {
    let artifact_line = if summary.artifact_path.is_empty() {
        String::new()
    } else {
        format!("\nArtifact: {}", summary.artifact_path)
    };
    let content = format!(
        "Previous turn summary: {}{}",
        summary.summary_narrative, artifact_line,
    );

    let summary_msg = ChatMessage {
        role: "system".to_string(),
        content,
        name: Some("turn_summary".to_string()),
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
        summarized: false,
    };

    let insert_pos = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(_, m)| m.role == "assistant")
        .map(|(i, _)| i + 1)
        .unwrap_or(0);

    messages.insert(insert_pos, summary_msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: &str, content: &str, summarized: bool) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
            summarized,
        }
    }

    fn make_tool_msg(content: &str, name: &str) -> ChatMessage {
        ChatMessage {
            role: "tool".to_string(),
            content: content.to_string(),
            name: Some(name.to_string()),
            tool_calls: None,
            tool_call_id: Some("t1".to_string()),
            reasoning_content: None,
            summarized: false,
        }
    }

    #[test]
    fn test_compute_effective_history_excludes_summarized() {
        let messages = vec![
            make_msg("user", "hello", false),
            make_msg("assistant", "hi there", false),
            make_msg("user", "next turn", false),
            make_msg("assistant", "response", true),
            make_msg("user", "third turn", false),
        ];

        let effective = compute_effective_history(&messages);
        assert_eq!(effective.len(), 4);
        assert_eq!(effective[0].content, "hello");
        assert_eq!(effective[1].content, "hi there");
        assert_eq!(effective[2].content, "next turn");
        assert_eq!(effective[3].content, "third turn");
    }

    #[test]
    fn test_compute_effective_history_all_summarized() {
        let messages = vec![
            make_msg("user", "old", true),
            make_msg("assistant", "old response", true),
        ];
        let effective = compute_effective_history(&messages);
        assert!(effective.is_empty());
    }

    #[test]
    fn test_compute_effective_history_none_summarized() {
        let messages = vec![
            make_msg("user", "hello", false),
            make_msg("assistant", "hi", false),
        ];
        let effective = compute_effective_history(&messages);
        assert_eq!(effective.len(), 2);
    }

    #[test]
    fn test_inject_turn_summary_after_assistant() {
        let mut messages = vec![
            make_msg("user", "hello", false),
            make_msg("assistant", "hi there", false),
        ];

        let summary = TurnSummaryOutput {
            uid: "s_test:0".to_string(),
            summary_narrative: "User said hello, Elma said hi".to_string(),
            artifact_path: String::new(),
        };

        inject_turn_summary(&mut messages, &summary);

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[2].role, "system");
        assert_eq!(messages[2].name, Some("turn_summary".to_string()));
        assert!(messages[2].content.contains("User said hello"));
    }

    #[test]
    fn test_inject_turn_summary_no_assistant() {
        let mut messages = vec![make_msg("user", "hello", false)];

        let summary = TurnSummaryOutput {
            uid: "s_test:0".to_string(),
            summary_narrative: "test".to_string(),
            artifact_path: String::new(),
        };

        inject_turn_summary(&mut messages, &summary);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
    }

    #[test]
    fn test_inject_turn_summary_with_artifacts() {
        let mut messages = vec![make_msg("assistant", "done", false)];

        let summary = TurnSummaryOutput {
            uid: "s_test:0".to_string(),
            summary_narrative: "Edited Cargo.toml".to_string(),
            artifact_path: "Cargo.toml".to_string(),
        };

        inject_turn_summary(&mut messages, &summary);
        assert!(messages[1].content.contains("Cargo.toml"));
    }

    // ── Task 768: Relevance and expiry tests ──

    #[test]
    fn test_relevance_filter_keeps_user_msgs() {
        let msgs = vec![
            make_msg("user", "find AGENTS.md", false),
            make_msg("assistant", "looking...", false),
        ];
        let filtered = compute_effective_history_with_relevance(&msgs, "find AGENTS.md", &[]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_relevance_expires_irrelevant_tool_msg() {
        let msgs = vec![
            make_msg("user", "find AGENTS.md", false),
            make_tool_msg("Found 3 files in _testing_prompts", "search"),
        ];
        let filtered = compute_effective_history_with_relevance(&msgs, "find AGENTS.md", &[]);
        // Tool msg about _testing_prompts not relevant to AGENTS.md — should expire
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_relevance_keeps_relevant_tool_msg() {
        let msgs = vec![
            make_msg("user", "find AGENTS.md", false),
            make_tool_msg("Found AGENTS.md in workspace root", "search"),
        ];
        let filtered = compute_effective_history_with_relevance(&msgs, "find AGENTS.md", &[]);
        // Tool msg contains "AGENTS.md" which appears in objective — keep
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_relevance_keeps_error_tool_msg() {
        let msgs = vec![
            make_msg("user", "read file", false),
            make_tool_msg("error: file not found", "read"),
        ];
        let filtered = compute_effective_history_with_relevance(&msgs, "read file", &[]);
        // Error messages are always kept
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_relevance_keeps_tool_with_scoped_path() {
        let msgs = vec![
            make_msg("user", "verify completed tasks", false),
            make_tool_msg("Contents of _tasks/completed/001_done.md", "read"),
        ];
        let prior = vec!["_tasks/completed".to_string()];
        let filtered =
            compute_effective_history_with_relevance(&msgs, "verify completed tasks", &prior);
        // Tool msg references a path from prior_turn_artifact_paths — keep
        assert_eq!(filtered.len(), 2);
    }
}
