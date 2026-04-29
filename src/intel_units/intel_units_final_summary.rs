//! @efficiency-role: domain-logic
//!
//! Final Summary Intel Unit
//!
//! Runs when summary tool is invoked to generate a concise final answer
//! using the model with accumulated evidence.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FinalSummaryOutput {
    pub summary: String,
}

pub(crate) struct FinalSummaryUnit {
    profile: Profile,
}

impl FinalSummaryUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for FinalSummaryUnit {
    fn name(&self) -> &'static str {
        "final_summary"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        let user_message = context.user_message.trim();
        if user_message.is_empty() {
            return Err(anyhow::anyhow!("No user message provided"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let user_request = &context.user_message;

        let evidence_summary = crate::evidence_ledger::get_session_ledger()
            .map(|ledger| {
                ledger
                    .entries
                    .iter()
                    .map(|e| e.summary.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        let narrative = format!(
            r#"You are generating a concise final summary for the user's request.

User request: {}

Evidence gathered:
{}

Provide a short, concise summary (1-2 sentences max) that directly answers the user's request based on the evidence above. If the task is not complete, state what is still needed."#,
            user_request,
            if evidence_summary.is_empty() {
                "(no evidence collected)".to_string()
            } else {
                evidence_summary
            }
        );

        let result =
            execute_intel_text_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({ "summary": result }),
            0.95,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("summary").is_none() {
            return Err(anyhow::anyhow!("Missing 'summary' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "summary": context.user_message.clone(),
                "reason": "fallback: returning user message as summary".to_string(),
            }),
            &format!("final summary failed: {}", error),
        ))
    }
}
