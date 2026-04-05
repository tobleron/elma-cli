//! @efficiency-role: domain-logic
//!
//! Responder intel units: ExpertAdvisor, ResultPresenter, StatusMessage,
//! Selector, RenameSuggester.

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Selector Unit
// ============================================================================

/// Selector Intel Unit
///
/// Selects items from evidence based on instructions.
pub(crate) struct SelectorUnit {
    profile: Profile,
}

impl SelectorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for SelectorUnit {
    fn name(&self) -> &'static str {
        "selector"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        // No specific pre-flight checks - selector is flexible
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let purpose = context
            .extra("purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let instructions = context
            .extra("instructions")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence = context
            .extra("evidence")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: SelectionOutput = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_selector_narrative(
                &context.user_message,
                &purpose,
                &instructions,
                &evidence,
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
        if output.get("items").is_none() {
            return Err(anyhow::anyhow!("Missing 'items' field"));
        }
        if output.get("reason").is_none() {
            return Err(anyhow::anyhow!("Missing 'reason' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "items": Vec::<String>::new(),
                "reason": "fallback: no items selected".to_string(),
            }),
            &format!("selector failed: {}", error),
        ))
    }
}

pub(crate) struct RenameSuggesterUnit {
    profile: Profile,
}

impl RenameSuggesterUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for RenameSuggesterUnit {
    fn name(&self) -> &'static str {
        "rename_suggester"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let purpose = context
            .extra("purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let instructions = context
            .extra("instructions")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence = context
            .extra("evidence")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: RenameSuggestion = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_rename_suggester_narrative(
                &context.user_message,
                &purpose,
                &instructions,
                &evidence,
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
        if output.get("identifier").is_none() {
            return Err(anyhow::anyhow!("Missing 'identifier' field"));
        }
        if output.get("reason").is_none() {
            return Err(anyhow::anyhow!("Missing 'reason' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "identifier": String::new(),
                "reason": "fallback: no rename suggested".to_string(),
            }),
            &format!("rename suggester failed: {}", error),
        ))
    }
}

// ============================================================================
// Result Presenter Unit
// ============================================================================

/// Result Presenter Intel Unit
///
/// Presents final results to the user in appropriate format.
pub(crate) struct ResultPresenterUnit {
    profile: Profile,
}

impl ResultPresenterUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ResultPresenterUnit {
    fn name(&self) -> &'static str {
        "result_presenter"
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
        let runtime_context = context
            .extra("runtime_context")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence_mode = context
            .extra("evidence_mode")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let response_advice = context
            .extra("response_advice")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let reply_instructions = context
            .extra("reply_instructions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("Present results clearly to the user"));
        let step_results = context
            .extra("step_results")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let result = execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_result_presenter_narrative(
                &context.user_message,
                &context.route_decision,
                &runtime_context,
                &evidence_mode,
                &response_advice,
                &reply_instructions,
                &step_results,
            ),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({ "final_text": result }),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("presentation").is_none() && output.get("final_text").is_none() {
            return Err(anyhow::anyhow!(
                "Missing 'presentation' or 'final_text' field"
            ));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "final_text": "Results presentation failed".to_string(),
                "reason": "fallback: presentation error".to_string(),
            }),
            &format!("result presenter failed: {}", error),
        ))
    }
}

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

// ============================================================================
// Expert Responder Unit
// ============================================================================

/// Expert Responder Intel Unit
///
/// Produces compact response-posture advice for the final presenter.
pub(crate) struct ExpertAdvisorUnit {
    profile: Profile,
}

impl ExpertAdvisorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ExpertAdvisorUnit {
    fn name(&self) -> &'static str {
        "expert_advisor"
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
        let evidence_mode = context
            .extra("evidence_mode")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let reply_instructions = context
            .extra("reply_instructions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("Respond clearly and use the evidence honestly."));
        let step_results = context
            .extra("step_results")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let result: ExpertAdvisorAdvice = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_expert_advisor_narrative(
                &context.user_message,
                &context.route_decision,
                &evidence_mode,
                &reply_instructions,
                &step_results,
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
        if output.get("expert_advice").is_none() {
            return Err(anyhow::anyhow!("Missing 'expert_advice' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "style": "direct",
                "focus": "answer with the key result first",
                "include_raw_output": false,
                "reason": "fallback: keep the response simple and honest",
            }),
            &format!("expert responder failed: {}", error),
        ))
    }
}

// ============================================================================
// Status Message Unit
// ============================================================================

/// Status Message Intel Unit
///
/// Generates status messages for execution steps.
pub(crate) struct StatusMessageUnit {
    profile: Profile,
}

impl StatusMessageUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for StatusMessageUnit {
    fn name(&self) -> &'static str {
        "status_message"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        // No specific pre-flight checks
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let current_action = context
            .extra("current_action")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.user_message));
        let step_type = context
            .extra("step_type")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let step_purpose = context
            .extra("step_purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_status_message_narrative(
                &current_action,
                &step_type,
                &step_purpose,
            ),
        )
        .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("status").is_none() {
            return Err(anyhow::anyhow!("Missing 'status' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "status": "Processing...",
                "reason": "fallback: default status".to_string(),
            }),
            &format!("status message failed: {}", error),
        ))
    }
}
