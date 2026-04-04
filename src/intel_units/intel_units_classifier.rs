//! @efficiency-role: domain-logic
//!
//! Classifier intel units: EvidenceMode, ArtifactClassifier, ComplexityClassifier,
//! RiskClassifier, EvidenceNeedsClassifier, ActionNeedsClassifier.

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Evidence Mode Unit
// ============================================================================

/// Evidence Mode Intel Unit
///
/// Determines how to present evidence (RAW, COMPACT, RAW_PLUS_COMPACT, etc.).
pub(crate) struct EvidenceModeUnit {
    profile: Profile,
}

impl EvidenceModeUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceModeUnit {
    fn name(&self) -> &'static str {
        "evidence_mode"
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
        let user_content = context
            .extra("narrative")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                serde_json::json!({
                    "user_message": context.user_message,
                    "route": context.route_decision.route,
                })
                .to_string()
            });
        let result: EvidenceModeDecision =
            execute_intel_json_from_user_content(&context.client, &self.profile, user_content)
                .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("mode").is_none() {
            return Err(anyhow::anyhow!("Missing 'mode' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "mode": "RAW",
                "reason": "fallback: show raw output".to_string(),
            }),
            &format!("evidence mode failed: {}", error),
        ))
    }
}

// ============================================================================
// Artifact Classifier Unit
// ============================================================================

/// Artifact Classifier Intel Unit
///
/// Classifies artifacts by type and importance.
pub(crate) struct ArtifactClassifierUnit {
    profile: Profile,
}

impl ArtifactClassifierUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ArtifactClassifierUnit {
    fn name(&self) -> &'static str {
        "artifact_classifier"
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
        let scope = context
            .extra("scope")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence = context
            .extra("evidence")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: ArtifactClassification = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_artifact_classifier_narrative(
                &objective, &scope, &evidence,
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
        if output.get("safe").is_none() {
            return Err(anyhow::anyhow!("Missing 'safe' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "safe": Vec::<String>::new(),
                "maybe": Vec::<String>::new(),
                "keep": Vec::<String>::new(),
                "ignore": Vec::<String>::new(),
                "reason": "fallback: no classifications".to_string(),
            }),
            &format!("artifact classifier failed: {}", error),
        ))
    }
}

// ============================================================================
// Atomic Classification Units (Task 012)
// ============================================================================

/// Complexity Classifier Intel Unit (atomic - single output)
pub(crate) struct ComplexityClassifierUnit {
    profile: Profile,
}

impl ComplexityClassifierUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ComplexityClassifierUnit {
    fn name(&self) -> &'static str {
        "complexity_classifier"
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
        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get_str("complexity").is_none() {
            return Err(anyhow::anyhow!("Missing 'complexity' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({"complexity": "INVESTIGATE"}),
            &format!("complexity classification failed: {}", error),
        ))
    }
}

/// Risk Classifier Intel Unit (atomic - single output)
pub(crate) struct RiskClassifierUnit {
    profile: Profile,
}

impl RiskClassifierUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for RiskClassifierUnit {
    fn name(&self) -> &'static str {
        "risk_classifier"
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
        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get_str("risk").is_none() {
            return Err(anyhow::anyhow!("Missing 'risk' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({"risk": "MEDIUM"}),
            &format!("risk classification failed: {}", error),
        ))
    }
}

/// Evidence Needs Classifier Intel Unit (atomic - 2 related outputs)
pub(crate) struct EvidenceNeedsClassifierUnit {
    profile: Profile,
}

impl EvidenceNeedsClassifierUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceNeedsClassifierUnit {
    fn name(&self) -> &'static str {
        "evidence_needs_classifier"
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
        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("needs_evidence").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_evidence' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({"needs_evidence": false, "needs_tools": false}),
            &format!("evidence needs classification failed: {}", error),
        ))
    }
}

pub(crate) struct ActionNeedsClassifierUnit {
    profile: Profile,
}

impl ActionNeedsClassifierUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ActionNeedsClassifierUnit {
    fn name(&self) -> &'static str {
        "action_needs_classifier"
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
        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("needs_decision").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_decision' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({"needs_decision": false, "needs_plan": false}),
            &format!("action needs classification failed: {}", error),
        ))
    }
}
