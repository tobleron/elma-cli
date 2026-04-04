//! @efficiency-role: util-pure
//!
//! Intel Narrative Utility Functions
//!
//! Shared helpers for building intel narratives:
//! - JSON rendering
//! - Conversation excerpt formatting
//! - Text fallback and snippet utilities

use crate::ChatMessage;
use serde_json::Value;

/// Format conversation excerpt into plain text
///
/// Converts conversation messages into readable narrative format.
pub(crate) fn format_conversation_excerpt(messages: &[ChatMessage], max_items: usize) -> String {
    messages
        .iter()
        .skip(1) // Skip first system message
        .rev()
        .take(max_items)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content.replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_json_value(value: &Value) -> String {
    if value.is_null() {
        "-".to_string()
    } else if let Some(text) = value.as_str() {
        text.trim().to_string()
    } else {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    }
}

pub(crate) fn fallback_text<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

pub(crate) fn snippet(text: &str) -> String {
    let compact = text.replace('\n', " ");
    if compact.len() <= 240 {
        compact
    } else {
        format!("{}...", &compact[..240])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_json_value_null() {
        let result = render_json_value(&Value::Null);
        assert_eq!(result, "-");
    }

    #[test]
    fn test_render_json_value_string() {
        let result = render_json_value(&Value::String("  hello  ".to_string()));
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_render_json_value_object() {
        let obj = serde_json::json!({"key": "value"});
        let result = render_json_value(&obj);
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_format_conversation_excerpt_empty() {
        let messages: Vec<ChatMessage> = vec![];
        let result = format_conversation_excerpt(&messages, 5);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_conversation_excerpt_skips_first() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "System prompt".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
        ];
        let result = format_conversation_excerpt(&messages, 5);
        assert!(result.contains("user"));
        assert!(result.contains("Hello"));
        assert!(!result.contains("System prompt"));
    }

    #[test]
    fn test_fallback_text_empty_returns_fallback() {
        assert_eq!(fallback_text("  ", "default"), "default");
    }

    #[test]
    fn test_fallback_text_nonempty_returns_trimmed() {
        assert_eq!(fallback_text("  hello  ", "default"), "hello");
    }

    #[test]
    fn test_snippet_short() {
        let result = snippet("short text");
        assert_eq!(result, "short text");
    }

    #[test]
    fn test_snippet_long_truncated() {
        let long = "x".repeat(300);
        let result = snippet(&long);
        assert!(result.len() > 240);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_snippet_replaces_newlines() {
        let result = snippet("line1\nline2");
        assert_eq!(result, "line1 line2");
    }
}
