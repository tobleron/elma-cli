//! @efficiency-role: domain-logic
//!
//! Evidence Sufficiency Intel Unit
//!
//! One job: decide if current evidence is sufficient or more gathering is needed.
//! Output: SUFFICIENT or NEEDS_MORE with reason

use crate::intel_trait::*;
use crate::*;

pub(crate) struct EvidenceSufficiencyUnit {
    profile: Profile,
}

impl EvidenceSufficiencyUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceSufficiencyUnit {
    fn name(&self) -> &'static str {
        "evidence_sufficiency"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.workspace_facts.trim().is_empty() {
            return Err(anyhow::anyhow!("No evidence to evaluate"));
        }
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("No objective to evaluate against"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let narrative = format!(
            r#"OBJECTIVE:
{objective}

COLLECTED EVIDENCE:
{evidence}

TASK:
Determine if the collected evidence is sufficient to answer the objective. SUFFICIENT means the evidence directly supports a complete answer. NEEDS_MORE means additional evidence gathering is required.

Output contract:
{{"status": "SUFFICIENT|NEEDS_MORE", "reason": "one short sentence", "missing": "what evidence is still needed (if NEEDS_MORE)"}}"#,
            objective = context.user_message.trim(),
            evidence = context
                .workspace_facts
                .chars()
                .take(2000)
                .collect::<String>(),
        );

        let result: serde_json::Value =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("status").is_none() {
            return Err(anyhow::anyhow!("Missing 'status' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let has_evidence = !context.workspace_facts.trim().is_empty();

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "status": if has_evidence { "SUFFICIENT" } else { "NEEDS_MORE" },
                "reason": "fallback: evidence presence check",
                "missing": if has_evidence { "" } else { "any evidence" },
            }),
            &format!("evidence sufficiency check failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_sufficiency_unit_creation() {
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
        let unit = EvidenceSufficiencyUnit::new(profile);
        assert_eq!(unit.name(), "evidence_sufficiency");
    }
}
