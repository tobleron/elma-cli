//! @efficiency-role: domain-logic
//!
//! Turn Summary Intel Unit
//!
//! One job: summarize a single conversation turn into one concise sentence
//! (under 100 words). Output is plain text — no JSON schema at all.
//! The returned sentence is stored as `summary_narrative` in session.json.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TurnSummaryOutput {
    pub uid: String,
    pub summary_narrative: String,
    pub artifact_path: String,
}

pub(crate) struct TurnSummaryUnit {
    profile: Profile,
}

impl TurnSummaryUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for TurnSummaryUnit {
    fn name(&self) -> &'static str {
        "turn_summary"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        let has_final = context
            .extra("final_text")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_final {
            return Err(anyhow::anyhow!("No final text to summarize"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let user_message = &context.user_message;
        let final_text = context
            .extra("final_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let uid = context
            .extra("uid")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let tool_results = context
            .extra("tool_results")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        // Task 619: Include tool result summary so the model can report
        // accurately whether operations actually succeeded or failed.
        let tool_block = if let Some(tr) = tool_results {
            format!("\n\nTool results:\n{}", tr)
        } else {
            String::new()
        };

        let prompt = format!(
            "Summarize THIS turn in ONE sentence under 30 words. \
             NO markdown, NO prefixes like 'Summary:', just the fact.\n\
             Focus ONLY on THIS turn (not previous).\n\
             Format: User asked X → outcome (succeeded/failed).\n\n\
             Examples:\n\
             - User asked about capabilities → confirmed file deletion via trash tool\n\
             - User requested delete scan_log.txt → file moved to trash\n\
             - User asked to edit file → edit completed successfully\n\n\
             User asked: {user_message}\n\
             Tools: {tool_block}\n\
             Response: {final_text}"
        );

        let raw = execute_intel_text_from_user_content(&context.client, &self.profile, prompt).await?;
        let raw = crate::text_utils::strip_thinking_blocks(&raw);

        let summary = raw
            .split_whitespace()
            .take(100)
            .collect::<Vec<_>>()
            .join(" ");

        // Task 604: If the LLM returned empty content, construct a minimal
        // narrative from the available context rather than returning an empty
        // summary that fails post_flight validation.
        let summary = if summary.trim().is_empty() {
            let user_preview = user_message.chars().take(120).collect::<String>();
            let outcome_preview = final_text.chars().take(200).collect::<String>();
            format!(
                "User asked: \"{user_preview}\". Outcome: {outcome_preview}"
            )
        } else {
            summary
        };

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "uid": uid,
                "summary_narrative": summary,
                "artifact_path": "",
            }),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        let text = output
            .get_str("summary_narrative")
            .unwrap_or("");
        if text.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty summary narrative"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let uid = context
            .extra("uid")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let user_msg = context.user_message.chars().take(200).collect::<String>();
        let final_excerpt = context
            .extra("final_text")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(300).collect::<String>())
            .unwrap_or_default();

        let narrative = if !final_excerpt.is_empty() {
            format!("User asked: \"{user_msg}\". Outcome: {final_excerpt}")
        } else {
            format!("User asked: \"{user_msg}\". Summary generation failed: {error}")
        };

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "uid": uid,
                "summary_narrative": narrative,
                "artifact_path": "",
            }),
            &format!("turn summary failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_summary_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 256,
            timeout_s: 15,
            system_prompt: "test".to_string(),
        };
        let unit = TurnSummaryUnit::new(profile);
        assert_eq!(unit.name(), "turn_summary");
    }

    #[test]
    fn test_turn_summary_output_fields() {
        let data = serde_json::json!({
            "uid": "s_test_0:0",
            "summary_narrative": "User asked to find unused deps. Elma searched and found serde_json.",
            "artifact_path": "",
        });
        let output = IntelOutput::success("turn_summary", data, 0.9);
        assert_eq!(
            output.get_str("summary_narrative"),
            Some("User asked to find unused deps. Elma searched and found serde_json.")
        );
        assert_eq!(output.get_str("uid"), Some("s_test_0:0"));
        assert_eq!(output.get_str("artifact_path"), Some(""));
    }
}
