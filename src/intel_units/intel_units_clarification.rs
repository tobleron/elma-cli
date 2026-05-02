//! @efficiency-role: domain-logic
//!
//! Clarification Intel Unit — Task 452.
//!
//! Handles user clarification requests when required information is missing.
//! Produces structured JSON with needed, question, and reason fields.

use crate::intel_trait::*;
use crate::*;

/// Clarification needed response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClarificationResponse {
    pub needed: Option<String>,
    pub question: Option<String>,
    pub reason: Option<String>,
    pub can_proceed: bool,
}

impl Default for ClarificationResponse {
    fn default() -> Self {
        Self {
            needed: None,
            question: None,
            reason: None,
            can_proceed: true,
        }
    }
}

/// Clarification Needed Intel Unit
///
/// Assesses whether the user request has sufficient information to proceed.
/// Returns clarification_needed if critical information is missing.
pub struct ClarificationNeededUnit {
    profile: Profile,
}

impl ClarificationNeededUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ClarificationNeededUnit {
    fn name(&self) -> &'static str {
        "clarification_needed"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(IntelError::EmptyUserMessage.into());
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let objective = context
            .extra("objective")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let workspace_facts = &context.workspace_facts;

        let result: ClarificationResponse = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            format!(
                "Assess whether the user request has sufficient information to proceed.\n\
                User request: {}\n\
                Current objective: {}\n\
                Workspace context: {}\n\
                \n\
                If the request is clear and actionable, return {{can_proceed: true}}.\n\
                If critical information is missing (e.g., target file, specific behavior, \
                constraints), return {{needed: \"...\", question: \"...\", reason: \"...\", can_proceed: false}}.",
                context.user_message,
                objective,
                workspace_facts.chars().take(500).collect::<String>()
            ),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.85,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("can_proceed").is_none() {
            return Err(IntelError::MissingField("can_proceed".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "needed": null,
                "question": null,
                "reason": "fallback: assumed sufficient",
                "can_proceed": true,
            }),
            &format!("clarification_needed failed: {}", error),
        ))
    }
}

/// Completion Check Intel Unit
///
/// Verifies whether a response satisfies the original objective.
pub struct CompletionCheckUnit {
    profile: Profile,
}

impl CompletionCheckUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for CompletionCheckUnit {
    fn name(&self) -> &'static str {
        "completion_check"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(IntelError::EmptyUserMessage.into());
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let objective = context
            .extra("objective")
            .cloned()
            .unwrap_or(serde_json::json!(context.user_message));
        let current_response = &context.user_message;

        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            format!(
                "Verify whether the response satisfies the original objective.\n\
                Original objective: {}\n\
                Current response: {}\n\
                \n\
                Return {{satisfied: true/false, gap: \"...\" if not satisfied}}.",
                objective,
                current_response.chars().take(500).collect::<String>()
            ),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            result,
            0.85,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("satisfied").is_none() {
            return Err(IntelError::MissingField("satisfied".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "satisfied": true,
                "reason": "fallback: assumed satisfied",
            }),
            &format!("completion_check failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clarification_needed_unit_name() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ClarificationNeededUnit::new(profile);
        assert_eq!(unit.name(), "clarification_needed");
    }

    #[test]
    fn test_completion_check_unit_name() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = CompletionCheckUnit::new(profile);
        assert_eq!(unit.name(), "completion_check");
    }

    #[test]
    fn test_clarification_response_defaults() {
        let response = ClarificationResponse::default();
        assert!(response.can_proceed);
        assert!(response.needed.is_none());
    }
}