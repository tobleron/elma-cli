//! @efficiency-role: domain-logic
//!
//! Classifier intel units: EvidenceMode, ArtifactClassifier, ComplexityClassifier,
//! RiskClassifier, EvidenceNeedsClassifier, ActionNeedsClassifier.

use crate::intel_trait::*;
use crate::intel_units::*;
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
        let dsl_result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, user_content)
                .await?;

        // Parse DSL result into EvidenceModeDecision
        let choice = dsl_result
            .get("choice")
            .and_then(|v| v.as_str())
            .unwrap_or("1")
            .to_string();
        let label = dsl_result
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("CHAT")
            .to_string();
        let reason = dsl_result
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("fallback: ultra concise")
            .to_string();
        let entropy = dsl_result
            .get("entropy")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);

        let mode = match choice.as_str() {
            "1" => "RAW".to_string(),
            "2" => "COMPACT".to_string(),
            "3" => "RAW_PLUS_COMPACT".to_string(),
            _ => "RAW".to_string(),
        };

        let decision = EvidenceModeDecision {
            mode,
            reason: format!("{label}: {reason}"),
        };

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&decision)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("mode").is_none() {
            return Err(anyhow::anyhow!("Missing 'mode' field"));
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
                "mode": "RAW".to_string(),
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_artifact_classifier_narrative(
                &objective, &scope, &evidence,
            ),
        )
        .await?;

        // Parse DSL result into ArtifactClassification
        let safe: Vec<String> = dsl_result
            .get("safe")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let maybe: Vec<String> = dsl_result
            .get("maybe")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let keep: Vec<String> = dsl_result
            .get("keep")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let ignore: Vec<String> = dsl_result
            .get("ignore")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let reason = dsl_result
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("fallback: no classifications")
            .to_string();

        let classification = ArtifactClassification {
            safe,
            maybe,
            keep,
            ignore,
            reason,
        };

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&classification)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("safe").is_none() {
            return Err(anyhow::anyhow!("Missing 'safe' field"));
        }
        if output.get("maybe").is_none() {
            return Err(anyhow::anyhow!("Missing 'maybe' field"));
        }
        if output.get("keep").is_none() {
            return Err(anyhow::anyhow!("Missing 'keep' field"));
        }
        if output.get("ignore").is_none() {
            return Err(anyhow::anyhow!("Missing 'ignore' field"));
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
                "safe": Vec::<String>::new(),
                "maybe": Vec::<String>::new(),
                "keep": Vec::<String>::new(),
                "ignore": Vec::<String>::new(),
                "reason": "fallback: no classifications",
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        // Parse DSL result into complexity and risk values
        let complexity = dsl_result
            .get("complexity")
            .and_then(|v| v.as_str())
            .unwrap_or("INVESTIGATE")
            .to_string();

        let risk = dsl_result
            .get("risk")
            .and_then(|v| v.as_str())
            .unwrap_or("MEDIUM")
            .to_string();

        let mut result = serde_json::Map::new();
        result.insert(
            "complexity".to_string(),
            serde_json::Value::String(complexity),
        );
        result.insert("risk".to_string(), serde_json::Value::String(risk));

        Ok(IntelOutput::success(
            self.name(),
            serde_json::Value::Object(result),
            0.9,
        ))
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
            serde_json::json!({"complexity": "INVESTIGATE".to_string()}),
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        // Parse DSL result into risk value
        let risk = dsl_result
            .get("risk")
            .and_then(|v| v.as_str())
            .unwrap_or("MEDIUM")
            .to_string();

        let mut result = serde_json::Map::new();
        result.insert("risk".to_string(), serde_json::Value::String(risk));

        Ok(IntelOutput::success(
            self.name(),
            serde_json::Value::Object(result),
            0.9,
        ))
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
            serde_json::json!({"risk": "MEDIUM".to_string()}),
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        // Parse DSL result into needs_evidence and needs_tools values
        let needs_evidence = dsl_result
            .get("needs_evidence")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let needs_tools = dsl_result
            .get("needs_tools")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut result = serde_json::Map::new();
        result.insert(
            "needs_evidence".to_string(),
            serde_json::Value::Bool(needs_evidence),
        );
        result.insert(
            "needs_tools".to_string(),
            serde_json::Value::Bool(needs_tools),
        );

        Ok(IntelOutput::success(
            self.name(),
            serde_json::Value::Object(result),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("needs_evidence").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_evidence' field"));
        }
        if output.get("needs_tools").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_tools' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "needs_evidence": false,
                "needs_tools": false,
            }),
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            serde_json::json!({
                "user_message": context.user_message,
            })
            .to_string(),
        )
        .await?;

        // Parse DSL result into needs_decision and needs_plan values
        let needs_decision = dsl_result
            .get("needs_decision")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let needs_plan = dsl_result
            .get("needs_plan")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut result = serde_json::Map::new();
        result.insert(
            "needs_decision".to_string(),
            serde_json::Value::Bool(needs_decision),
        );
        result.insert(
            "needs_plan".to_string(),
            serde_json::Value::Bool(needs_plan),
        );

        Ok(IntelOutput::success(
            self.name(),
            serde_json::Value::Object(result),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("needs_decision").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_decision' field"));
        }
        if output.get("needs_plan").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_plan' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "needs_decision": false,
                "needs_plan": false,
            }),
            &format!("action needs classification failed: {}", error),
        ))
    }
}
