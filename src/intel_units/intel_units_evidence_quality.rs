//! @efficiency-role: domain-logic
//!
//! Evidence Quality Intel Unit
//!
//! One job: classify a tool result's evidence quality.
//! Output: DIRECT, INDIRECT, or WEAK

use crate::intel_trait::*;
use crate::*;

pub(crate) struct EvidenceQualityUnit {
    profile: Profile,
}

impl EvidenceQualityUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceQualityUnit {
    fn name(&self) -> &'static str {
        "evidence_quality"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.workspace_facts.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty evidence to classify"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let narrative = format!(
            r#"EVIDENCE:
{evidence}

TOOL: {tool_name}
EXIT CODE: {exit_code}

TASK:
Classify the evidence quality. DIRECT means the evidence directly answers the question. INDIRECT means it supports the answer but requires inference. WEAK means the evidence is insufficient or unreliable.

Output contract:
{{"quality": "DIRECT|INDIRECT|WEAK", "reason": "one short sentence"}}"#,
            evidence = context
                .workspace_facts
                .chars()
                .take(2000)
                .collect::<String>(),
            tool_name = context
                .extra("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            exit_code = context
                .extra("exit_code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
        );

        let result: serde_json::Value =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("quality").is_none() {
            return Err(anyhow::anyhow!("Missing 'quality' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let exit_code = context
            .extra("exit_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        let has_content = !context.workspace_facts.trim().is_empty();

        let quality = if exit_code == 0 && has_content {
            "DIRECT"
        } else if has_content {
            "INDIRECT"
        } else {
            "WEAK"
        };

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "quality": quality,
                "reason": format!("fallback: exit_code={exit_code}, has_content={has_content}"),
            }),
            &format!("evidence quality classification failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_quality_unit_creation() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = EvidenceQualityUnit::new(profile);
        assert_eq!(unit.name(), "evidence_quality");
    }
}
