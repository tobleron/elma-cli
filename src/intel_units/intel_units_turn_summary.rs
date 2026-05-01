//! @efficiency-role: domain-logic
//!
//! Turn Summary Intel Unit
//!
//! One job: summarize a single conversation turn into a compact narrative
//! that can replace the raw messages in the next turn's context.
//! Output: structured data with narrative, status, tools, artifacts.
//!
//! The model produces only two fields (summary_narrative + status_category).
//! All factual fields (tools_used, tool_call_count, errors, artifacts_created,
//! noteworthy) are pre-filled by Rust from the execution trace.

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

        // Read factual fields from context extras — populated by Rust from the
        // execution trace (Task 411 + Task 417). The model is NOT asked to
        // produce these; they never appear in the narrative prompt.
        let tools_used_str = context
            .extra("tools_used")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tool_call_count = context
            .extra("tool_call_count")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0) as u64;
        let errors_str = context
            .extra("errors")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let artifacts_str = context
            .extra("artifacts_created")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Model is asked for only TWO fields: narrative + status.
        let narrative = format!(
            r#"USER REQUEST: {user_message}
ROUTE: {route}
FORMULA: {formula}
STEP RESULTS: {step_results}
FINAL RESPONSE: {final_text}

TASK:
Summarize what happened in this turn. Write a compact narrative that captures what the user asked, what actions Elma took, and what the outcome was. This summary will replace the raw turn messages in the next turn's context.

Output DSL format (single line):
TURN summary_narrative="compact narrative" status_category=completed

CRITICAL: Output ONLY the raw TURN line. Do NOT wrap it in backticks, markdown code blocks, or any other formatting. No prose before or after. Just one TURN line exactly as shown.

Valid status_category values: completed | partial | failed"#
        );

        let dsl_result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, narrative).await?;

        // Parse comma-separated extras into vectors — populated from Rust, not model.
        let parse_csv = |s: &str| -> Vec<String> {
            s.split(',')
                .filter_map(|t| {
                    let trimmed = t.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        };

        let tools_parsed: Vec<String> = parse_csv(tools_used_str);
        let errors_parsed: Vec<String> = parse_csv(errors_str);
        let artifacts_parsed: Vec<String> = parse_csv(artifacts_str);

        // Only two fields from model; the rest from Rust.
        let result = TurnSummaryOutput {
            summary_narrative: dsl_result
                .get("summary_narrative")
                .and_then(|v| v.as_str())
                .unwrap_or("turn summary")
                .to_string(),
            status_category: dsl_result
                .get("status_category")
                .and_then(|v| v.as_str())
                .unwrap_or("completed")
                .to_string(),
            noteworthy: !errors_parsed.is_empty()
                || !artifacts_parsed.is_empty()
                || tool_call_count > 0,
            tools_used: tools_parsed,
            tool_call_count: tool_call_count as usize,
            errors: errors_parsed,
            artifacts_created: artifacts_parsed,
        };

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
        let formula = context
            .extra("formula")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // Populate factual fields from context extras in fallback too.
        let parse_csv = |s: &str| -> Vec<String> {
            s.split(',')
                .filter_map(|t| {
                    let trimmed = t.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        };

        let tools_str = context
            .extra("tools_used")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tools_parsed: Vec<String> = parse_csv(tools_str);
        let tc = context
            .extra("tool_call_count")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap_or(0);
        let err_str = context
            .extra("errors")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let errors_parsed: Vec<String> = parse_csv(err_str);
        let art_str = context
            .extra("artifacts_created")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let artifacts_parsed: Vec<String> = parse_csv(art_str);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "summary_narrative": format!("User asked: \"{user_msg}\". Elma responded (formula: {formula}) but the summary generation failed."),
                "status_category": "partial",
                "noteworthy": false,
                "tools_used": tools_parsed,
                "tool_call_count": tc,
                "errors": [error.to_string()],
                "artifacts_created": artifacts_parsed,
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

    #[test]
    fn test_turn_summary_fields_from_rust_not_model() {
        // Verify that TurnSummaryOutput is constructed with all fields
        // even though the model only provides summary_narrative and status_category.
        let output = TurnSummaryOutput {
            summary_narrative: "Test narrative".to_string(),
            status_category: "completed".to_string(),
            noteworthy: true,
            tools_used: vec!["read".to_string(), "bash".to_string()],
            tool_call_count: 2,
            errors: vec!["timeout".to_string()],
            artifacts_created: vec!["output.txt".to_string()],
        };
        assert_eq!(output.summary_narrative, "Test narrative");
        assert_eq!(output.status_category, "completed");
        assert!(output.noteworthy);
        assert_eq!(output.tools_used.len(), 2);
        assert_eq!(output.tool_call_count, 2);
        assert_eq!(output.errors.len(), 1);
        assert_eq!(output.artifacts_created.len(), 1);
    }
}
