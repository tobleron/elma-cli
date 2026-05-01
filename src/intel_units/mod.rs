//! @efficiency-role: domain-logic
//!
//! Intel Units
//!
//! This module contains Elma's trait-based intel units.
//! Re-exports from sub-modules for backward compatibility.

mod intel_units_action_selector;
mod intel_units_advanced;
mod intel_units_claim_mapper;
mod intel_units_classifier;
mod intel_units_core;
pub(crate) mod intel_units_dsl;
mod intel_units_evidence_quality;
mod intel_units_evidence_staleness;
mod intel_units_evidence_sufficiency;
mod intel_units_final_summary;
mod intel_units_goal_consistency;
mod intel_units_intent;
pub(crate) mod intel_units_maestro;
mod intel_units_pyramid;
mod intel_units_repair;
mod intel_units_responder;
mod intel_units_turn_summary;

// Re-export maestro types for external use
pub(crate) use intel_units_maestro::{MaestroInstruction, MaestroOutput, MaestroUnit};

// Re-export DSL output parsing utilities
pub(crate) use intel_units_dsl::{
    parse_auto_dsl, parse_claim_block_dsl, parse_critic_verdict_dsl, parse_formula_dsl,
    parse_intel_dsl_to_value, parse_list_dsl, parse_next_action_dsl, parse_pyramid_block_dsl,
    parse_record_dsl_to_value, parse_scope_dsl, parse_selection_dsl, parse_verdict_dsl,
};

// Re-export all intel units for backward compatibility
pub(crate) use intel_units_action_selector::*;
pub(crate) use intel_units_advanced::*;
pub(crate) use intel_units_claim_mapper::*;
pub(crate) use intel_units_classifier::*;
pub(crate) use intel_units_core::*;
pub(crate) use intel_units_evidence_quality::*;
pub(crate) use intel_units_evidence_staleness::*;
pub(crate) use intel_units_evidence_sufficiency::*;
pub(crate) use intel_units_final_summary::*;
pub(crate) use intel_units_goal_consistency::*;
pub(crate) use intel_units_intent::*;
pub(crate) use intel_units_pyramid::*;
pub(crate) use intel_units_repair::*;
pub(crate) use intel_units_responder::*;
pub(crate) use intel_units_turn_summary::*;

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Evidence Compactor Unit
// ============================================================================

/// Evidence Compactor Intel Unit
///
/// Compacts large evidence into a more concise form.
pub(crate) struct EvidenceCompactorUnit {
    profile: Profile,
}

impl EvidenceCompactorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceCompactorUnit {
    fn name(&self) -> &'static str {
        "evidence_compactor"
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
        let scope = context
            .extra("scope")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let cmd = context
            .extra("cmd")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let output = context
            .extra("output")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_evidence_compactor_narrative(
                &objective, &purpose, &scope, &cmd, &output,
            ),
        )
        .await?;

        let result = EvidenceCompact {
            summary: dsl_result
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            key_facts: dsl_result
                .get("key_facts")
                .and_then(|v| v.as_str())
                .map(|s| {
                    s.split(',')
                        .filter_map(|t| {
                            let t = t.trim();
                            if t.is_empty() {
                                None
                            } else {
                                Some(t.to_string())
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            noise: dsl_result
                .get("noise")
                .and_then(|v| v.as_str())
                .map(|s| {
                    s.split(',')
                        .filter_map(|t| {
                            let t = t.trim();
                            if t.is_empty() {
                                None
                            } else {
                                Some(t.to_string())
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("compacted_evidence").is_none() && output.get("summary").is_none() {
            return Err(anyhow::anyhow!(
                "Missing 'compacted_evidence' or 'summary' field"
            ));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "compacted_evidence": context.workspace_facts,
                "reason": "fallback: returned original evidence".to_string(),
            }),
            &format!("evidence compactor failed: {}", error),
        ))
    }
}

// ============================================================================
// Formatter Unit
// ============================================================================

/// Formatter Intel Unit
///
/// Cleans up and structures the final response for terminal display.
pub(crate) struct FormatterUnit {
    profile: Profile,
}

impl FormatterUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for FormatterUnit {
    fn name(&self) -> &'static str {
        "formatter"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty input text"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        // Formatter uses text-out task logic but for the "user_message" content which is the draft to format
        let result = execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            context.user_message.clone(),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({ "formatted_text": result }),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("formatted_text").is_none() {
            return Err(anyhow::anyhow!("Missing 'formatted_text' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "formatted_text": context.user_message.clone(),
                "reason": "fallback: return original text",
            }),
            &format!("formatter failed: {}", error),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_assessment_unit_creation() {
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
        let unit = ComplexityAssessmentUnit::new(profile);
        assert_eq!(unit.name(), "complexity_assessment");
    }

    #[test]
    fn test_evidence_needs_unit_creation() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = EvidenceNeedsUnit::new(profile);
        assert_eq!(unit.name(), "evidence_needs_assessment");
    }

    #[test]
    fn test_action_needs_unit_creation() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ActionNeedsUnit::new(profile);
        assert_eq!(unit.name(), "action_needs_assessment");
    }

    #[test]
    fn test_workflow_planner_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 768,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = WorkflowPlannerUnit::new(profile);
        assert_eq!(unit.name(), "workflow_planner");
    }

    #[test]
    fn test_pattern_suggestion_unit_creation() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = PatternSuggestionUnit::new(profile);
        assert_eq!(unit.name(), "pattern_suggestion");
    }

    #[test]
    fn test_scope_builder_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 768,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ScopeBuilderUnit::new(profile);
        assert_eq!(unit.name(), "scope_builder");
    }

    #[test]
    fn test_formula_selector_unit_creation() {
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
        let unit = FormulaSelectorUnit::new(profile);
        assert_eq!(unit.name(), "formula_selector");
    }

    #[test]
    fn test_selector_unit_creation() {
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
        let unit = SelectorUnit::new(profile);
        assert_eq!(unit.name(), "selector");
    }

    #[test]
    fn test_evidence_mode_unit_creation() {
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
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = EvidenceModeUnit::new(profile);
        assert_eq!(unit.name(), "evidence_mode");
    }

    #[test]
    fn test_evidence_compactor_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 1024,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = EvidenceCompactorUnit::new(profile);
        assert_eq!(unit.name(), "evidence_compactor");
    }

    #[test]
    fn test_artifact_classifier_unit_creation() {
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
        let unit = ArtifactClassifierUnit::new(profile);
        assert_eq!(unit.name(), "artifact_classifier");
    }

    #[test]
    fn test_result_presenter_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 1024,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ResultPresenterUnit::new(profile);
        assert_eq!(unit.name(), "result_presenter");
    }

    #[test]
    fn test_status_message_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 64,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = StatusMessageUnit::new(profile);
        assert_eq!(unit.name(), "status_message");
    }

    #[test]
    fn test_command_repair_unit_creation() {
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
        let unit = CommandRepairUnit::new(profile);
        assert_eq!(unit.name(), "command_repair");
    }

    // Task 012: Atomic classifier unit tests
    #[test]
    fn test_complexity_classifier_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 64,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ComplexityClassifierUnit::new(profile);
        assert_eq!(unit.name(), "complexity_classifier");
    }

    #[test]
    fn test_risk_classifier_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 64,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = RiskClassifierUnit::new(profile);
        assert_eq!(unit.name(), "risk_classifier");
    }

    #[test]
    fn test_evidence_needs_classifier_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 64,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = EvidenceNeedsClassifierUnit::new(profile);
        assert_eq!(unit.name(), "evidence_needs_classifier");
    }

    #[test]
    fn test_action_needs_classifier_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 64,
            timeout_s: 120,
            system_prompt: "test".to_string(),
        };
        let unit = ActionNeedsClassifierUnit::new(profile);
        assert_eq!(unit.name(), "action_needs_classifier");
    }
}
