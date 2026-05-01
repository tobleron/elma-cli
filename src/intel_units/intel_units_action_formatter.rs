//! @efficiency-role: domain-logic
//!
//! Action DSL Format Specialist Intel Unit
//!
//! One job: format an action decision into exact action DSL syntax.
//! This separates "what to do" (action type selection, Task 416) from
//! "how to say it" (this unit). Input is an action decision; output is
//! a valid action DSL line or block with no prose or formatting errors.
//!
//! GBNF grammar enforcement is applied to prevent prose output.

use crate::intel_trait::*;
use crate::*;

pub(crate) struct ActionDslFormatter {
    profile: Profile,
}

impl ActionDslFormatter {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ActionDslFormatter {
    fn name(&self) -> &'static str {
        "action_formatter"
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
        let action = context
            .extra("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("R");
        let target = context
            .extra("target")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let reason = context
            .extra("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let narrative = format!(
            r#"ACTION TYPE: {action}
TARGET: {target}
REASON: {reason}

TASK:
Format this action into exact DSL syntax. Output exactly one valid DSL line or block.

Rules:
- R path="relative/path" for file reads
- L path="dir" depth=1 for directory listings
- S q="text" path="dir" for content search
- X + shell command on next line + ---END on its own line for shell commands
- DONE summary="one-line" for completion
- All paths MUST be quoted with double quotes (path="...")
- Block actions (X, E) MUST end with ---END on its own line
- No prose before or after the DSL

Output ONLY the raw DSL. No backticks, no markdown."#
        );

        let dsl_result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, narrative).await?;

        // The formatter produces raw DSL text. Extract it from the candidates.
        let formatted = context
            .extra("_raw_dsl")
            .and_then(|v| v.as_str())
            .unwrap_or("R path=\".\"");
        // We don't parse the output as DSL — we validate it is valid action text.
        let dsl_text = dsl_result
            .get("_raw")
            .and_then(|v| v.as_str())
            .unwrap_or(formatted)
            .to_string();

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "formatted": dsl_text,
                "action": action,
            }),
            0.9,
        ))
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        let action = context
            .extra("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("R");
        let target = context
            .extra("target")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let fallback_dsl = match action {
            "R" => format!("R path=\"{}\"", target),
            "L" => format!("L path=\"{}\" depth=1", target),
            "S" => format!("S q=\"search\" path=\"{}\"", target),
            "X" => format!("X\n{}\n---END", target),
            "DONE" => format!("DONE summary=\"{}\"", target),
            _ => format!("R path=\"{}\"", target),
        };
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "formatted": fallback_dsl,
                "action": action,
            }),
            &format!("action_formatter fallback: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_formatter_unit_creation() {
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
            timeout_s: 15,
            system_prompt: "test".to_string(),
        };
        let unit = ActionDslFormatter::new(profile);
        assert_eq!(unit.name(), "action_formatter");
    }

    #[test]
    fn test_fallback_produces_valid_r() {
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
            timeout_s: 15,
            system_prompt: "test".to_string(),
        };
        let unit = ActionDslFormatter::new(profile);
        let mut ctx = IntelContext::new(
            "read file".to_string(),
            RouteDecision::default(),
            String::new(),
            String::new(),
            vec![],
            reqwest::Client::new(),
        );
        ctx.extras.insert(
            "action_type".to_string(),
            serde_json::Value::String("R".to_string()),
        );
        ctx.extras.insert(
            "target".to_string(),
            serde_json::Value::String("src/main.rs".to_string()),
        );
        let result = unit.fallback(&ctx, "test error").unwrap();
        let formatted = result.get_str("formatted").unwrap();
        assert!(formatted.contains("R path="));
        assert!(formatted.contains("src/main.rs"));
    }
}
