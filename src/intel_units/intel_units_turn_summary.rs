//! @efficiency-role: domain-logic
//!
//! Turn Summary Intel Unit
//!
//! One job: summarize a single conversation turn into a compact narrative
//! that can replace the raw messages in the next turn's context.
//! Output: structured JSON with narrative, status, tools, artifacts.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TurnSummaryOutput {
    pub summary_narrative: String,
    pub status_category: String,
    pub noteworthy: bool,
    pub tools_used: Vec<String>,
    pub tool_call_count: usize,
    pub errors: Vec<String>,
    pub artifacts_created: Vec<String>,
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
        let has_steps = context
            .extra("step_results")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if !has_final && !has_steps {
            return Err(anyhow::anyhow!("No turn data to summarize"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let user_message = &context.user_message;
        let route = &context.route_decision.route;
        let formula = context
            .extra("formula")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let final_text = context
            .extra("final_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tools_used = context
            .extra("tools_used")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let step_results = context
            .extra("step_results")
            .and_then(|v| v.as_array())
            .map(|a| {
                serde_json::to_string_pretty(&a)
                    .unwrap_or_default()
                    .chars()
                    .take(3000)
                    .collect::<String>()
            })
            .unwrap_or_default();

        let narrative = format!(
            r#"USER REQUEST: {user_message}
ROUTE: {route}
FORMULA: {formula}
TOOLS USED: {tools_used}
STEP RESULTS: {step_results}
FINAL RESPONSE: {final_text}

TASK:
Summarize what happened in this turn. Write a compact narrative that captures what the user asked, what actions Elma took, and what the outcome was. This summary will replace the raw turn messages in the next turn's context.

Output contract:
{{"summary_narrative": "...", "status_category": "completed|blocked|failed|waiting|partial", "noteworthy": true/false, "tools_used": ["read","bash"], "tool_call_count": 4, "errors": [], "artifacts_created": ["path/to/file"]}}"#
        );

        let result: TurnSummaryOutput =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("summary_narrative").is_none() {
            return Err(anyhow::anyhow!("Missing 'summary_narrative' field"));
        }
        if output.get("status_category").is_none() {
            return Err(anyhow::anyhow!("Missing 'status_category' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let user_msg = context.user_message.chars().take(120).collect::<String>();
        let tools = context
            .extra("tools_used")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let formula = context
            .extra("formula")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "summary_narrative": format!("User asked: \"{user_msg}\". Elma responded (formula: {formula}, tools: {tools}) but the summary generation failed."),
                "status_category": "partial",
                "noteworthy": false,
                "tools_used": [],
                "tool_call_count": 0,
                "errors": [error.to_string()],
                "artifacts_created": [],
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
            "summary_narrative": "User asked to find unused deps. Elma searched and found serde_json.",
            "status_category": "completed",
            "noteworthy": false,
            "tools_used": ["read", "bash"],
            "tool_call_count": 3,
            "errors": [],
            "artifacts_created": []
        });
        let output = IntelOutput::success("turn_summary", data, 0.9);
        assert_eq!(
            output.get_str("summary_narrative"),
            Some("User asked to find unused deps. Elma searched and found serde_json.")
        );
        assert_eq!(output.get_str("status_category"), Some("completed"));
        assert_eq!(output.get_bool("noteworthy"), Some(false));
        assert_eq!(
            output
                .get("tools_used")
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(2)
        );
    }
}
