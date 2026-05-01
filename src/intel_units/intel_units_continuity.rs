//! @efficiency-role: domain-logic
//!
//! Answer continuity intel unit — Task 380 post-execution check.
//! Verifies the final answer aligns with the user's original request.
//! Output: 3-field JSON compliant with Task 378.

use crate::intel_trait::*;
use crate::*;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// 3-field continuity verdict for post-execution alignment check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContinuityVerdictOutput {
    /// Whether the answer aligns with the original question
    pub aligned: bool,
    /// Confidence in the verdict (0.0–1.0)
    pub confidence: f64,
    /// Why the answer is or isn't aligned
    pub reason: String,
}

/// Checks whether a final answer addresses the user's original request.
pub(crate) struct AnswerContinuityUnit {
    profile: Profile,
}

impl AnswerContinuityUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }

    /// Convenience: verify alignment without building the full context.
    pub async fn verify_alignment(
        &self,
        client: &reqwest::Client,
        user_request: &str,
        final_answer: &str,
    ) -> ContinuityVerdictOutput {
        let narrative = format!(
            "USER REQUEST:\n{user_request}\n\nFINAL ANSWER:\n{final_answer}\n\n\
             Does the final answer address the user's original request?\n\
             Respond with JSON: aligned, confidence, reason"
        );
        let context = IntelContext::new(
            narrative,
            crate::RouteDecision::default(),
            String::new(),
            String::new(),
            Vec::new(),
            client.clone(),
        );
        match self.execute(&context).await {
            Ok(output) => serde_json::from_value(output.data).unwrap_or(ContinuityVerdictOutput {
                aligned: true,
                confidence: 0.5,
                reason: "fallback: failed to parse verdict".to_string(),
            }),
            Err(_) => ContinuityVerdictOutput {
                aligned: true,
                confidence: 0.5,
                reason: "fallback: execution error".to_string(),
            },
        }
    }
}

impl IntelUnit for AnswerContinuityUnit {
    fn name(&self) -> &'static str {
        "answer_continuity"
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
        let result: ContinuityVerdictOutput = execute_intel_json_from_user_content(
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
        if output.data.get("aligned").and_then(|v| v.as_bool()).is_none() {
            return Err(anyhow!("Missing aligned field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "aligned": true,
                "confidence": 0.5,
                "reason": format!("fallback: {}", error),
            }),
            &format!("answer continuity check failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_output_deserialization() {
        let json = serde_json::json!({
            "aligned": true,
            "confidence": 0.95,
            "reason": "Answer directly addresses the question"
        });
        let v: ContinuityVerdictOutput = serde_json::from_value(json).unwrap();
        assert!(v.aligned);
        assert!((v.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_verdict_output_requires_all_fields() {
        let json = serde_json::json!({
            "aligned": true,
            "confidence": 0.95
            // missing "reason"
        });
        let result: Result<ContinuityVerdictOutput, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_answer_continuity_unit_creation() {
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
            system_prompt: "Verify intent alignment.".to_string(),
        };
        let unit = AnswerContinuityUnit::new(profile);
        assert_eq!(unit.name(), "answer_continuity");
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
        let unit = AnswerContinuityUnit::new(profile);
        let ctx = IntelContext::new(
            "test".to_string(),
            crate::RouteDecision::default(),
            String::new(),
            String::new(),
            Vec::new(),
            reqwest::Client::new(),
        );
        let fallback = unit.fallback(&ctx, "error").unwrap();
        assert_eq!(fallback.get_bool("aligned"), Some(true));
        assert!(!fallback.get_str("reason").unwrap_or("").is_empty());
    }
}
