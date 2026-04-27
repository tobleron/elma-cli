//! @efficiency-role: domain-logic
//!
//! Evidence Staleness Intel Unit
//!
//! One job: determine if existing evidence is stale given new actions.
//! Output: FRESH, POTENTIALLY_STALE, or STALE

use crate::intel_trait::*;
use crate::*;

pub(crate) struct EvidenceStalenessUnit {
    profile: Profile,
}

impl EvidenceStalenessUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceStalenessUnit {
    fn name(&self) -> &'static str {
        "evidence_staleness"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.workspace_facts.trim().is_empty() {
            return Err(anyhow::anyhow!("No evidence to check"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let narrative = format!(
            r#"EXISTING EVIDENCE:
{evidence}

NEW ACTION:
{action}

TASK:
Determine if the existing evidence is stale after the new action. FRESH means the evidence is still valid. POTENTIALLY_STALE means it might be outdated. STALE means the evidence is definitely outdated.

Output contract:
{{"staleness": "FRESH|POTENTIALLY_STALE|STALE", "reason": "one short sentence"}}"#,
            evidence = context
                .workspace_facts
                .chars()
                .take(2000)
                .collect::<String>(),
            action = context
                .extra("new_action")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
        );

        let result: serde_json::Value =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("staleness").is_none() {
            return Err(anyhow::anyhow!("Missing 'staleness' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let action = context
            .extra("new_action")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let is_write =
            action.contains("write") || action.contains("edit") || action.contains("modify");

        let staleness = if is_write { "STALE" } else { "FRESH" };

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "staleness": staleness,
                "reason": format!("fallback: action={action}"),
            }),
            &format!("evidence staleness check failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_staleness_unit_creation() {
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
        let unit = EvidenceStalenessUnit::new(profile);
        assert_eq!(unit.name(), "evidence_staleness");
    }
}
