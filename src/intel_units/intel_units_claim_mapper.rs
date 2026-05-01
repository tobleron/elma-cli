//! @efficiency-role: domain-logic
//!
//! Claim-Evidence Mapper Intel Unit
//!
//! One job: extract factual claims from a draft answer and map each to supporting evidence entries.
//! This is the enforcement gate that prevents unsupported claims from reaching the user.

use crate::intel_trait::*;
use crate::*;

pub(crate) struct ClaimMapperUnit {
    profile: Profile,
}

impl ClaimMapperUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ClaimMapperUnit {
    fn name(&self) -> &'static str {
        "claim_mapper"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty draft answer"));
        }
        if context.workspace_facts.trim().is_empty() {
            return Err(anyhow::anyhow!("No evidence available"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let narrative = format!(
            r#"DRAFT ANSWER:
{draft}

AVAILABLE EVIDENCE:
{evidence}

TASK:
Extract every factual claim from the draft answer. For each claim, identify which evidence entry (by ID) supports it. If no evidence supports a claim, mark it as UNGROUNDED.

Factual claims include: statements about file contents, existence, structure, command outputs, system state, specific values, counts, or configurations.

Skip: opinions, recommendations, general knowledge, restatements of the user's question, procedural descriptions.

Output DSL format:
CLAIM statement="the file exists" evidence_ids="e_001" status=GROUNDED
CLAIM statement="the value is 42" evidence_ids="e_002,e_003" status=GROUNDED
CLAIM statement="config is wrong" evidence_ids="" status=UNGROUNDED
REASON text="3 claims found, 2 grounded"
END"#,
            draft = context.user_message.trim(),
            evidence = context
                .workspace_facts
                .chars()
                .take(3000)
                .collect::<String>(),
        );

        let result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("claims").is_none() {
            return Err(anyhow::anyhow!("Missing 'claims' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "claims": [],
                "reason": format!("fallback: claim mapping failed, using heuristic: {}", error),
            }),
            &format!("claim mapper failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_mapper_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 512,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ClaimMapperUnit::new(profile);
        assert_eq!(unit.name(), "claim_mapper");
    }
}
