//! @efficiency-role: domain-logic
//!
//! Effective History Module
//!
//! Computes the effective message history for the next LLM call by:
//! - Excluding messages marked as `summarized = true`
//! - Injecting turn summaries as system messages at turn boundaries
//!
//! This is the core of the deferred pre-turn summary system (Task 310).
//! It replaces raw turn messages with compact summaries to save context window.

use crate::intel_units::TurnSummaryOutput;
use crate::types_api::ChatMessage;

/// Compute the effective message history for the next LLM call.
/// Messages marked `summarized = true` are excluded from the result.
/// The remaining messages preserve their original order.
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
    let content = if summary.artifacts_created.is_empty() {
        format!(
            "Previous turn summary: {}\nStatus: {}\nTools used: {}",
            summary.summary_narrative,
            summary.status_category,
            summary.tools_used.join(", "),
        )
    } else {
        format!(
            "Previous turn summary: {}\nStatus: {}\nTools used: {}\nArtifacts: {}",
            summary.summary_narrative,
            summary.status_category,
            summary.tools_used.join(", "),
            summary.artifacts_created.join(", "),
        )
    };

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
            summary_narrative: "User said hello, Elma said hi".to_string(),
            status_category: "completed".to_string(),
            noteworthy: false,
            tools_used: vec![],
            tool_call_count: 0,
            errors: vec![],
            artifacts_created: vec![],
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
            summary_narrative: "test".to_string(),
            status_category: "completed".to_string(),
            noteworthy: false,
            tools_used: vec![],
            tool_call_count: 0,
            errors: vec![],
            artifacts_created: vec![],
        };

        inject_turn_summary(&mut messages, &summary);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
    }

    #[test]
    fn test_inject_turn_summary_with_artifacts() {
        let mut messages = vec![make_msg("assistant", "done", false)];

        let summary = TurnSummaryOutput {
            summary_narrative: "Edited Cargo.toml".to_string(),
            status_category: "completed".to_string(),
            noteworthy: true,
            tools_used: vec!["edit".to_string()],
            tool_call_count: 1,
            errors: vec![],
            artifacts_created: vec!["Cargo.toml".to_string()],
        };

        inject_turn_summary(&mut messages, &summary);
        assert!(messages[1].content.contains("Cargo.toml"));
    }
}
