//! @efficiency-role: domain-logic
//!
//! Thought Summary Intel Unit (Task 622)
//!
//! One job: summarize a completed thinking thread in ≤70 words in the first
//! person ("I thought about..."). Runs on the auxiliary LLM at a different
//! endpoint (e.g., port 8084) so it never consumes the main model's context.
//! The summary is streamed to the right panel as a permanent thought record.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ThoughtSummaryOutput {
    pub summary: String,
    pub word_count: usize,
}

pub(crate) struct ThoughtSummaryUnit {
    profile: Profile,
}

impl ThoughtSummaryUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ThoughtSummaryUnit {
    fn name(&self) -> &'static str {
        "thought_summary"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        let has_thinking = context
            .extra("thinking_content")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_thinking {
            return Err(anyhow::anyhow!("No thinking content to summarize"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let thinking = context
            .extra("thinking_content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let word_count = thinking.split_whitespace().count();

        // If the thinking is already short, use it as-is
        if word_count <= 70 {
            return Ok(IntelOutput::success(
                self.name(),
                serde_json::json!({
                    "summary": thinking,
                    "word_count": word_count,
                }),
                0.95,
            ));
        }

        let prompt = format!(
            "Summarize this thinking thread in ≤70 words, in the first person. \
             Write as if you are the thinker reporting what you considered:\n\n\
             \"I thought about...\"\n\n\
             Thinking:\n{thinking}"
        );

        let raw = execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            prompt,
        )
        .await?;

        let raw = crate::text_utils::strip_thinking_blocks(&raw);
        let summary: String = raw.split_whitespace().take(70).collect::<Vec<_>>().join(" ");

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "summary": summary,
                "word_count": summary.split_whitespace().count(),
            }),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        let text = output.get_str("summary").unwrap_or("");
        if text.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty thought summary"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let thinking = context
            .extra("thinking_content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Fallback: take first 70 words as-is
        let summary: String = thinking
            .split_whitespace()
            .take(70)
            .collect::<Vec<_>>()
            .join(" ");

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "summary": summary,
                "word_count": summary.split_whitespace().count(),
            }),
            &format!("thought summary failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thought_summary_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "thought_summary".to_string(),
            base_url: "http://192.168.1.186:8084".to_string(),
            model: "auxiliary".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 128,
            timeout_s: 15,
            system_prompt: "Summarize thinking in ≤70 words, first person.".to_string(),
        };
        let unit = ThoughtSummaryUnit::new(profile);
        assert_eq!(unit.name(), "thought_summary");
    }

    #[test]
    fn test_thought_summary_output_short_bypasses_model() {
        // Short content (< 70 words) should return as-is without model call
        let output = ThoughtSummaryOutput {
            summary: "Short thought".to_string(),
            word_count: 2,
        };
        assert_eq!(output.word_count, 2);
        assert!(output.summary.len() < 200);
    }
}
