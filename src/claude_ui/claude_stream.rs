//! @efficiency-role: ui-component
//!
//! Streaming UI State for Claude Code-style streaming
//!
//! Handles real-time thinking and assistant text streaming:
//! - Shows "∴ Thinking" while reasoning streams
//! - Shows "● " as assistant text arrives
//! - Toggle transcript mode to see full content

use super::claude_state::ClaudeMessage;

// = : = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = =
// Streaming UI State
// = : = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = = =

pub(crate) fn strip_thinking_tags(content: &str) -> String {
    content
        .replace("<think>", "")
        .replace("[/CA]", "")
        .replace("[/MODEL]", "")
        .replace("<think>\n", "")
        .replace("[/CA]\n", "")
        .trim()
        .to_string()
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StreamingUI {
    pub thinking: String,
    pub content: String,
    pub is_streaming_thinking: bool,
    pub is_streaming_content: bool,
    pub finished: bool,
    pub error: Option<String>,
}

impl StreamingUI {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn start_thinking(&mut self) {
        self.is_streaming_thinking = true;
        self.thinking.clear();
    }

    pub(crate) fn append_thinking(&mut self, text: &str) {
        if !text.is_empty() {
            self.thinking.push_str(text);
        }
    }

    pub(crate) fn finish_thinking(&mut self) {
        self.is_streaming_thinking = false;
    }

    pub(crate) fn start_content(&mut self) {
        self.is_streaming_content = true;
        // Task 171: Do not clear thinking when content starts; preserve for transcript
        self.is_streaming_thinking = false;
    }

    pub(crate) fn append_content(&mut self, text: &str) {
        if !text.is_empty() {
            self.content.push_str(text);
        }
    }

    pub(crate) fn finish_content(&mut self) {
        self.is_streaming_content = false;
        self.finished = true;
    }

    pub(crate) fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.is_streaming_thinking = false;
        self.is_streaming_content = false;
    }

    pub(crate) fn to_messages(&self) -> Vec<ClaudeMessage> {
        let mut messages = Vec::new();

        if self.is_streaming_thinking || (!self.thinking.is_empty() && !self.is_streaming_content) {
            messages.push(ClaudeMessage::Thinking {
                content: self.thinking.clone(),
            });
        }

        if self.is_streaming_content || !self.content.is_empty() {
            messages.push(ClaudeMessage::Assistant {
                content: self.content.clone(),
            });
        }

        if let Some(ref err) = self.error {
            messages.push(ClaudeMessage::System {
                content: err.clone(),
            });
        }

        messages
    }

    pub(crate) fn state_indicator(&self) -> &'static str {
        if self.is_streaming_thinking {
            "∴ Thinking"
        } else if self.is_streaming_content {
            "…"
        } else if self.finished {
            ""
        } else if self.error.is_some() {
            "⚠ Error"
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_thinking() {
        let mut s = StreamingUI::new();
        s.start_thinking();
        s.append_thinking("Analyzing");
        s.append_thinking(" the request");
        s.finish_thinking();

        assert_eq!(s.thinking, "Analyzing the request");
        assert!(!s.is_streaming_thinking);
    }

    #[test]
    fn test_streaming_content() {
        let mut s = StreamingUI::new();
        s.start_content();
        s.append_content("Hello! I'm Elma.");
        s.finish_content();

        assert_eq!(s.content, "Hello! I'm Elma.");
        assert!(s.finished);
    }

    #[test]
    fn test_state_indicator() {
        let mut s = StreamingUI::new();
        assert_eq!(s.state_indicator(), "");

        s.start_thinking();
        assert_eq!(s.state_indicator(), "∴ Thinking");

        s.finish_thinking();
        s.start_content();
        assert_eq!(s.state_indicator(), "…");
    }
}
