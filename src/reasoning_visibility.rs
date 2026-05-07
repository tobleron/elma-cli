//! @efficiency-role: util-pure
//!
//! Reasoning Visibility Policy for thinking/chain-of-thought display
//!
//! Controls how reasoning content is surfaced to the user:
//! - Hidden: suppress all reasoning
//! - Summarized: show truncated preview with word count
//! - Expanded: show full reasoning with sanitization
//! - RawDebug: show full reasoning as-is (for debugging)

/// Controls how reasoning/thinking content is displayed to the user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReasoningVisibilityMode {
    /// Suppress all reasoning content from display
    Hidden,
    /// Show a brief summary: first 200 chars + word count
    Summarized,
    /// Show full reasoning with basic sanitization (remove think tags)
    Expanded,
    /// Show full reasoning with no sanitization (debugging use)
    RawDebug,
}

impl Default for ReasoningVisibilityMode {
    fn default() -> Self {
        Self::Summarized
    }
}

/// Configuration for reasoning content redaction and display
#[derive(Debug, Clone)]
pub(crate) struct ReasoningVisibilityPolicy {
    /// Current display mode for reasoning content
    pub mode: ReasoningVisibilityMode,
    /// Whether to redact sensitive patterns from reasoning
    pub redact_sensitive: bool,
}

impl Default for ReasoningVisibilityPolicy {
    fn default() -> Self {
        Self {
            mode: ReasoningVisibilityMode::default(),
            redact_sensitive: true,
        }
    }
}

impl ReasoningVisibilityPolicy {
    pub(crate) fn new(mode: ReasoningVisibilityMode) -> Self {
        Self {
            mode,
            redact_sensitive: true,
        }
    }

    pub(crate) fn with_redaction(mut self, redact: bool) -> Self {
        self.redact_sensitive = redact;
        self
    }
}

/// Normalize reasoning content according to the specified visibility mode.
///
/// - Hidden: returns empty string
/// - Summarized: returns first 200 characters + word count
/// - Expanded: returns full content with think/model tags removed
/// - RawDebug: returns full content as-is
pub(crate) fn normalize_reasoning_content(raw: &str, mode: &ReasoningVisibilityMode) -> String {
    match mode {
        ReasoningVisibilityMode::Hidden => String::new(),
        ReasoningVisibilityMode::Summarized => {
            let word_count = raw.split_whitespace().count();
            let preview: String = raw.chars().take(200).collect();
            format!("{} ... ({} words total)", preview, word_count)
        }
        ReasoningVisibilityMode::Expanded => sanitize_reasoning(raw),
        ReasoningVisibilityMode::RawDebug => raw.to_string(),
    }
}

/// Remove think/model tags from reasoning content.
fn sanitize_reasoning(content: &str) -> String {
    let cleaned = content
        .replace("<think>", " ")
        .replace("</think>", " ")
        .replace("<thinking>", " ")
        .replace("</thinking>", " ")
        .replace("<thought>", " ")
        .replace("</thought>", " ")
        .replace("<reasoning>", " ")
        .replace("</reasoning>", " ")
        .replace("[/CA]", " ")
        .replace("[/MODEL]", " ");
    // Collapse multiple spaces
    let mut result = String::with_capacity(cleaned.len());
    let mut prev_space = false;
    for ch in cleaned.chars() {
        if ch.is_whitespace() && ch != '\n' {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

/// Sanitize final answer text by removing think/model tags and reasoning markers.
///
/// This ensures no leftover thinking markers bleed into the user-visible answer.
pub(crate) fn sanitize_final_answer(text: &str) -> String {
    let cleaned = text
        .replace("<think>", " ")
        .replace("</think>", " ")
        .replace("<thinking>", " ")
        .replace("</thinking>", " ")
        .replace("<thought>", " ")
        .replace("</thought>", " ")
        .replace("<reasoning>", " ")
        .replace("</reasoning>", " ")
        .replace("[/CA]", " ")
        .replace("[/MODEL]", " ")
        .replace("<<<reasoning_content_start>>>", " ")
        .replace("<<<reasoning_content_end>>>", " ");
    // Collapse multiple spaces
    let mut result = String::with_capacity(cleaned.len());
    let mut prev_space = false;
    for ch in cleaned.chars() {
        if ch.is_whitespace() && ch != '\n' {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_returns_empty() {
        let result =
            normalize_reasoning_content("deep thinking content", &ReasoningVisibilityMode::Hidden);
        assert_eq!(result, "");
    }

    #[test]
    fn summarized_shows_preview_and_word_count() {
        let content = "this is a short reasoning block";
        let result = normalize_reasoning_content(content, &ReasoningVisibilityMode::Summarized);
        assert!(result.contains("this is a short reasoning block"));
        assert!(result.contains("(6 words total)"));
    }

    #[test]
    fn summarized_truncates_long_content() {
        let content = "a b c d e f g h i j";
        let result = normalize_reasoning_content(content, &ReasoningVisibilityMode::Summarized);
        assert!(result.contains("a b c d e f g h i j"));
        assert!(result.contains("(10 words total)"));
    }

    #[test]
    fn expanded_removes_think_tags() {
        let content = "<think>reasoning</think>content";
        let result = normalize_reasoning_content(content, &ReasoningVisibilityMode::Expanded);
        assert!(!result.contains("<think>"));
        assert!(!result.contains("</think>"));
        assert!(result.contains("reasoning"));
    }

    #[test]
    fn expanded_removes_thinking_tags() {
        let content = "<thinking>reasoning</thinking>content";
        let result = normalize_reasoning_content(content, &ReasoningVisibilityMode::Expanded);
        assert!(!result.contains("<thinking>"));
        assert!(!result.contains("</thinking>"));
        assert!(result.contains("reasoning"));
    }

    #[test]
    fn expanded_removes_model_tags() {
        let content = "some text [/CA] more text [/MODEL] end";
        let result = normalize_reasoning_content(content, &ReasoningVisibilityMode::Expanded);
        assert!(!result.contains("[/CA]"));
        assert!(!result.contains("[/MODEL]"));
    }

    #[test]
    fn raw_debug_returns_as_is() {
        let content = "<think>raw</think>[/CA]content";
        let result = normalize_reasoning_content(content, &ReasoningVisibilityMode::RawDebug);
        assert_eq!(result, content);
    }

    #[test]
    fn sanitize_final_answer_removes_tags() {
        let text = "<think>thinking</think>the answer is 42[/CA]";
        let result = sanitize_final_answer(text);
        assert!(!result.contains("<think>"));
        assert!(!result.contains("</think>"));
        assert!(!result.contains("[/CA]"));
        assert_eq!(result, "thinking the answer is 42");
    }

    #[test]
    fn sanitize_final_answer_trims_whitespace() {
        let text = "  <think>thinking</think>  answer  ";
        let result = sanitize_final_answer(text);
        assert_eq!(result, "thinking answer");
    }

    #[test]
    fn sanitize_final_answer_removes_llama_markers() {
        let text = "<<<reasoning_content_start>>>think<<<reasoning_content_end>>>answer";
        let result = sanitize_final_answer(text);
        assert!(!result.contains("<<<reasoning_content_start>>>"));
        assert!(!result.contains("<<<reasoning_content_end>>>"));
        assert_eq!(result, "think answer");
    }

    #[test]
    fn sanitize_final_answer_removes_thought_tags() {
        let text = "<thought>inner</thought>reply";
        let result = sanitize_final_answer(text);
        assert!(!result.contains("<thought>"));
        assert!(!result.contains("</thought>"));
        assert_eq!(result, "inner reply");
    }

    #[test]
    fn sanitize_final_answer_removes_reasoning_tags() {
        let text = "<reasoning>deep</reasoning>answer";
        let result = sanitize_final_answer(text);
        assert!(!result.contains("<reasoning>"));
        assert!(!result.contains("</reasoning>"));
        assert_eq!(result, "deep answer");
    }

    #[test]
    fn default_mode_is_summarized() {
        let policy = ReasoningVisibilityPolicy::default();
        assert_eq!(policy.mode, ReasoningVisibilityMode::Summarized);
    }

    #[test]
    fn policy_new_sets_mode() {
        let policy = ReasoningVisibilityPolicy::new(ReasoningVisibilityMode::Hidden);
        assert_eq!(policy.mode, ReasoningVisibilityMode::Hidden);
        assert!(policy.redact_sensitive);
    }

    #[test]
    fn policy_with_redaction_chains() {
        let policy =
            ReasoningVisibilityPolicy::new(ReasoningVisibilityMode::Expanded).with_redaction(false);
        assert_eq!(policy.mode, ReasoningVisibilityMode::Expanded);
        assert!(!policy.redact_sensitive);
    }

    #[test]
    fn empty_content_handling() {
        assert_eq!(
            normalize_reasoning_content("", &ReasoningVisibilityMode::Hidden),
            ""
        );
        let result = normalize_reasoning_content("", &ReasoningVisibilityMode::Summarized);
        assert!(result.contains("(0 words total)"));

        assert_eq!(
            normalize_reasoning_content("", &ReasoningVisibilityMode::Expanded),
            ""
        );
        assert_eq!(
            normalize_reasoning_content("", &ReasoningVisibilityMode::RawDebug),
            ""
        );
    }
}
