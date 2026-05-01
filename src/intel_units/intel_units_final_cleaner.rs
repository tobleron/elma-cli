//! @efficiency-role: domain-logic
//!
//! Final answer cleaner intel unit — Task 384.
//! Rewrites internal-model answers into clean user-facing responses.
//! 3-field JSON output compliant with Task 378.

use crate::intel_trait::*;
use crate::*;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// 3-field output for the final cleaner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CleanedAnswer {
    /// The cleaned, user-facing answer text.
    pub cleaned_text: String,
    /// What was cleaned from the original.
    pub cleaned_aspects: String,
    /// Confidence in the cleaned output.
    pub confidence: f64,
}

/// Rewrites internal-framed answers into clean user-facing responses.
pub(crate) struct FinalCleanerUnit {
    profile: Profile,
}

impl FinalCleanerUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }

    /// Convenience method: clean a raw answer with the intel unit.
    pub async fn clean(
        &self,
        client: &reqwest::Client,
        raw_answer: &str,
        original_request: &str,
    ) -> String {
        let narrative = format!(
            "ORIGINAL REQUEST:\n{original_request}\n\nORIGINAL ANSWER (with internal framing):\n{raw_answer}\n\n\
             Rewrite the answer as a clean, direct response to the user. \
             Remove all internal framing, evidence formatting, analysis headers, \
             step markers, and system metadata. Output ONLY the cleaned JSON."
        );
        let context = IntelContext::new(
            narrative,
            crate::RouteDecision::default(),
            String::new(),
            String::new(),
            Vec::new(),
            client.clone(),
        );
        match self.execute_with_fallback(&context).await {
            Ok(output) => {
                output
                    .get_str("cleaned_text")
                    .unwrap_or(raw_answer)
                    .to_string()
            }
            Err(_) => raw_answer.to_string(),
        }
    }
}

impl IntelUnit for FinalCleanerUnit {
    fn name(&self) -> &'static str {
        "final_cleaner"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result: CleanedAnswer = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            context.user_message.clone(),
        )
        .await?;
        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.85,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get_str("cleaned_text").is_none_or(|s| s.is_empty()) {
            return Err(anyhow!("Missing cleaned_text field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "cleaned_text": "",
                "cleaned_aspects": "fallback",
                "confidence": 0.0,
            }),
            &format!("final cleaner failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleaned_answer_deserialization() {
        let json = serde_json::json!({
            "cleaned_text": "The time is 5:35 PM.",
            "cleaned_aspects": "removed Analysis header",
            "confidence": 0.95
        });
        let c: CleanedAnswer = serde_json::from_value(json).unwrap();
        assert_eq!(c.cleaned_text, "The time is 5:35 PM.");
        assert!((c.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_cleaned_answer_requires_all_fields() {
        let json = serde_json::json!({
            "cleaned_text": "hello",
            "confidence": 0.5
            // missing cleaned_aspects
        });
        let result: Result<CleanedAnswer, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_final_cleaner_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 128,
            timeout_s: 30,
            system_prompt: "Clean internal framing.".to_string(),
        };
        let unit = FinalCleanerUnit::new(profile);
        assert_eq!(unit.name(), "final_cleaner");
    }

    #[test]
    fn test_fallback_defaults() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 128,
            timeout_s: 30,
            system_prompt: "test".to_string(),
        };
        let unit = FinalCleanerUnit::new(profile);
        let ctx = IntelContext::new(
            "test".to_string(),
            crate::RouteDecision::default(),
            String::new(),
            String::new(),
            Vec::new(),
            reqwest::Client::new(),
        );
        let fallback = unit.fallback(&ctx, "error").unwrap();
        assert_eq!(fallback.get_str("cleaned_text"), Some(""));
    }
}
