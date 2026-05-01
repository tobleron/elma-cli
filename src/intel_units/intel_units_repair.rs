//! @efficiency-role: domain-logic
//!
//! Repair intel units: CommandRepair.

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
        let dsl_result = execute_intel_dsl_from_user_content(
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

        let result = CommandRepair {
            cmd: dsl_result
                .get("cmd")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            reason: dsl_result
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

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

// JSON repair (model-produced JSON) was removed by the compact DSL migration (Task 384).
