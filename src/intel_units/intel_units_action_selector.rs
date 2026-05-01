//! @efficiency-role: domain-logic
//!
//! Action Selector Intel Unit
//!
//! One job: choose exactly one action type (R, L, S, Y, E, X, ASK, DONE)
//! for the next tool loop turn. Emits a single-field DSL:
//!   SELECT action=R reason="short justification"
//!
//! This reduces cognitive load by separating the "what action?" decision
//! from the "format the action DSL" job (which belongs to the action DSL
//! prompt in the tool loop).

use crate::intel_trait::*;
use crate::*;

pub(crate) struct ActionSelectorUnit {
    profile: Profile,
}

impl ActionSelectorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ActionSelectorUnit {
    fn name(&self) -> &'static str {
        "action_selector"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let user_message = &context.user_message;
        let objective = context
            .extra("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("none");
        let last_evidence = context
            .extra("last_evidence")
            .and_then(|v| v.as_str())
            .unwrap_or("none");

        let narrative = format!(
            r#"USER REQUEST: {user_message}
CURRENT OBJECTIVE: {objective}
LAST EVIDENCE: {last_evidence}

TASK:
Choose the single best action type for the next tool turn.

Output DSL format (single line):
SELECT action=R reason="short justification"

Valid action types:
R  = read a file
L  = list directory contents
S  = search file contents
Y  = search for symbols
E  = edit a file
X  = run shell command
ASK = ask user for clarification
DONE = mark task as complete

Output ONLY the raw SELECT line. No backticks, markdown, or prose."#
        );

        let dsl_result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, narrative).await?;

        let action = dsl_result
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("R")
            .to_string();
        let reason = dsl_result
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "action": action,
                "reason": reason,
            }),
            0.8,
        ))
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "action": "R",
                "reason": format!("selector failed: {}", error),
            }),
            &format!("action_selector fallback: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_selector_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 32,
            timeout_s: 15,
            system_prompt: "test".to_string(),
        };
        let unit = ActionSelectorUnit::new(profile);
        assert_eq!(unit.name(), "action_selector");
    }
}
