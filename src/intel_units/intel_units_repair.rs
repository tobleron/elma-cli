//! @efficiency-role: domain-logic
//!
//! Repair intel units: CommandRepair, JsonRepair.

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Command Repair Unit
// ============================================================================

/// Command Repair Intel Unit
///
/// Repairs malformed or failed shell commands.
pub(crate) struct CommandRepairUnit {
    profile: Profile,
}

impl CommandRepairUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for CommandRepairUnit {
    fn name(&self) -> &'static str {
        "command_repair"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        // No specific pre-flight checks
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let objective = context
            .extra("objective")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let purpose = context
            .extra("purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let output = context
            .extra("output")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: CommandRepair = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_command_repair_narrative(
                &objective,
                &purpose,
                &context.user_message,
                &output,
            ),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("repaired_command").is_none() && output.get("cmd").is_none() {
            return Err(anyhow::anyhow!("Missing 'repaired_command' or 'cmd' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "cmd": "".to_string(),
                "reason": "fallback: could not repair command".to_string(),
            }),
            &format!("command repair failed: {}", error),
        ))
    }
}

// ============================================================================
// JSON Repair Unit
// ============================================================================

/// JSON Repair Intel Unit
///
/// Repairs malformed JSON using a dedicated profile.
pub(crate) struct JsonRepairUnit {
    profile: Profile,
}

impl JsonRepairUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }

    pub async fn repair_with_fallback(
        &self,
        client: &reqwest::Client,
        chat_url: &Url,
        original_json: &str,
        problems: &[String],
    ) -> Result<String> {
        let route_decision = RouteDecision {
            route: "DECIDE".to_string(),
            source: "json_repair".to_string(),
            distribution: vec![("DECIDE".to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: ProbabilityDecision {
                choice: "INSTRUCT".to_string(),
                source: "json_repair".to_string(),
                distribution: vec![("INSTRUCT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            workflow: ProbabilityDecision {
                choice: "WORKFLOW".to_string(),
                source: "json_repair".to_string(),
                distribution: vec![("WORKFLOW".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            mode: ProbabilityDecision {
                choice: "DECIDE".to_string(),
                source: "json_repair".to_string(),
                distribution: vec![("DECIDE".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
        };

        let context = IntelContext::new(
            original_json.to_string(),
            route_decision,
            problems.join("\n"),
            String::new(),
            Vec::new(),
            client.clone(),
        );

        let output = self.execute_with_fallback(&context).await?;
        Ok(output
            .get_str("repaired_json")
            .unwrap_or(original_json)
            .to_string())
    }
}

impl IntelUnit for JsonRepairUnit {
    fn name(&self) -> &'static str {
        "json_repair"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty JSON input"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let problems_text = if context.workspace_facts.trim().is_empty() {
            "No problems found".to_string()
        } else {
            context.workspace_facts.trim().to_string()
        };

        let req = build_intel_system_user_request(
            &self.profile,
            format!(
                "Original JSON:\n{}\n\nProblems to fix:\n{}",
                context.user_message, problems_text
            ),
        );

        let chat_url = Url::parse(&self.profile.base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base_url '{}': {}", self.profile.base_url, e))?
            .join("/v1/chat/completions")
            .map_err(|e| anyhow::anyhow!("Failed to build chat URL: {}", e))?;
        let response =
            chat_once_with_timeout(&context.client, &chat_url, &req, self.profile.timeout_s)
                .await?;
        let repaired_json = extract_response_text(&response);

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "repaired_json": repaired_json,
            }),
            0.8,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output
            .get_str("repaired_json")
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(anyhow::anyhow!("Missing repaired_json output"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "repaired_json": context.user_message,
            }),
            &format!("json repair failed: {}", error),
        ))
    }
}
