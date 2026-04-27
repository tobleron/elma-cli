//! @efficiency-role: domain-logic
//!
//! Advanced Assessment Intel Units
//!
//! Provides specialized units for:
//! - Domain difficulty classification
//! - Freshness requirement analysis
//! - Assumption tracking
//! - Edge case evaluation

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Domain Difficulty Unit
// ============================================================================

/// Domain Difficulty Assessment Output
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct DomainDifficultyAssessment {
    pub domain_type: String,     // "common", "specialized", "expert", "niche"
    pub knowledge_level: String, // "basic", "intermediate", "advanced"
    pub sensitive: bool,
    pub expertise_required: String, // "none", "general", "specific"
    pub domain_label: String,
    pub confidence: f64,
    pub entropy: f64,
}

impl Default for DomainDifficultyAssessment {
    fn default() -> Self {
        Self {
            domain_type: "common".to_string(),
            knowledge_level: "basic".to_string(),
            sensitive: false,
            expertise_required: "none".to_string(),
            domain_label: "general".to_string(),
            confidence: 0.5,
            entropy: 0.8,
        }
    }
}

/// Domain Difficulty Intel Unit
///
/// Classifies domain expertise level and knowledge requirements.
pub(crate) struct DomainDifficultyUnit {
    profile: Profile,
}

impl DomainDifficultyUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for DomainDifficultyUnit {
    fn name(&self) -> &'static str {
        "domain_difficulty"
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
        let narrative = context
            .extra("narrative")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                crate::intel_narrative_advanced::build_domain_difficulty_narrative(
                    &context.user_message,
                    &context.route_decision,
                    &context.workspace_facts,
                    &context.workspace_brief,
                    &context.conversation_excerpt,
                )
            });

        let result: DomainDifficultyAssessment =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&result)?,
            result.confidence,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("domain_type").is_none() {
            return Err(anyhow::anyhow!("Missing 'domain_type' field"));
        }
        if output.get("knowledge_level").is_none() {
            return Err(anyhow::anyhow!("Missing 'knowledge_level' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let default = DomainDifficultyAssessment::default();
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::to_value(&default)?,
            &format!("domain difficulty assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Freshness Requirement Unit
// ============================================================================

/// Freshness Requirement Assessment Output
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct FreshnessRequirementAssessment {
    pub freshness_needed: String, // "stable", "moderate", "high"
    pub staleness_risk: String,   // "low", "medium", "high"
    pub update_frequency: String, // "rare", "occasional", "frequent"
    pub sources: Vec<String>,     // ["api", "news", "docs", "standards"]
    pub time_sensitivity: String, // "none", "low", "medium", "high"
    pub confidence: f64,
    pub entropy: f64,
}

impl Default for FreshnessRequirementAssessment {
    fn default() -> Self {
        Self {
            freshness_needed: "stable".to_string(),
            staleness_risk: "low".to_string(),
            update_frequency: "rare".to_string(),
            sources: vec![],
            time_sensitivity: "none".to_string(),
            confidence: 0.5,
            entropy: 0.8,
        }
    }
}

/// Freshness Requirement Intel Unit
///
/// Identifies information currency needs and staleness risks.
pub(crate) struct FreshnessRequirementUnit {
    profile: Profile,
}

impl FreshnessRequirementUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for FreshnessRequirementUnit {
    fn name(&self) -> &'static str {
        "freshness_requirement"
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
        let narrative = context
            .extra("narrative")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                crate::intel_narrative_advanced::build_freshness_requirement_narrative(
                    &context.user_message,
                    &context.route_decision,
                    &context.workspace_facts,
                    &context.workspace_brief,
                    &context.conversation_excerpt,
                )
            });

        let result: FreshnessRequirementAssessment =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&result)?,
            result.confidence,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("freshness_needed").is_none() {
            return Err(anyhow::anyhow!("Missing 'freshness_needed' field"));
        }
        if output.get("staleness_risk").is_none() {
            return Err(anyhow::anyhow!("Missing 'staleness_risk' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let default = FreshnessRequirementAssessment::default();
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::to_value(&default)?,
            &format!("freshness requirement assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Assumption Tracker Unit
// ============================================================================

/// Individual Assumption Record
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AssumptionRecord {
    pub text: String,
    pub risk: String, // "low", "medium", "high"
    pub dependency: String,
    pub change_impact: String, // "minor", "major", "critical"
    pub verifiable: bool,
}

/// Assumption Tracker Assessment Output
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AssumptionTrackerAssessment {
    pub assumptions: Vec<AssumptionRecord>,
    pub needs_verification: bool,
    pub assumption_count: usize,
    pub high_risk_count: usize,
    pub confidence: f64,
    pub entropy: f64,
}

impl Default for AssumptionTrackerAssessment {
    fn default() -> Self {
        Self {
            assumptions: vec![],
            needs_verification: false,
            assumption_count: 0,
            high_risk_count: 0,
            confidence: 0.5,
            entropy: 0.8,
        }
    }
}

/// Assumption Tracker Intel Unit
///
/// Tracks assumptions made, their validity, and change impacts.
pub(crate) struct AssumptionTrackerUnit {
    profile: Profile,
}

impl AssumptionTrackerUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for AssumptionTrackerUnit {
    fn name(&self) -> &'static str {
        "assumption_tracker"
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
        let narrative = context
            .extra("narrative")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                crate::intel_narrative_advanced::build_assumption_tracker_narrative(
                    &context.user_message,
                    &context.route_decision,
                    &context.workspace_facts,
                    &context.workspace_brief,
                    &context.conversation_excerpt,
                )
            });

        let result: AssumptionTrackerAssessment =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&result)?,
            result.confidence,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("assumptions").is_none() {
            return Err(anyhow::anyhow!("Missing 'assumptions' field"));
        }
        if output.get("needs_verification").is_none() {
            return Err(anyhow::anyhow!("Missing 'needs_verification' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let default = AssumptionTrackerAssessment::default();
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::to_value(&default)?,
            &format!("assumption tracker assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Edge Case Evaluator Unit
// ============================================================================

/// Individual Edge Case Record
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct EdgeCaseRecord {
    pub scenario: String,
    pub likelihood: String, // "rare", "possible", "likely"
    pub impact: String,     // "minor", "moderate", "severe"
    pub mitigation: String,
}

/// Edge Case Evaluator Assessment Output
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct EdgeCaseEvaluatorAssessment {
    pub edge_cases: Vec<EdgeCaseRecord>,
    pub failure_modes: Vec<String>,
    pub hidden_deps: Vec<String>,
    pub edge_case_count: usize,
    pub high_risk_count: usize,
    pub confidence: f64,
    pub entropy: f64,
}

impl Default for EdgeCaseEvaluatorAssessment {
    fn default() -> Self {
        Self {
            edge_cases: vec![],
            failure_modes: vec![],
            hidden_deps: vec![],
            edge_case_count: 0,
            high_risk_count: 0,
            confidence: 0.5,
            entropy: 0.8,
        }
    }
}

/// Edge Case Evaluator Intel Unit
///
/// Identifies potential failure modes, exceptions, and dependencies.
pub(crate) struct EdgeCaseEvaluatorUnit {
    profile: Profile,
}

impl EdgeCaseEvaluatorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EdgeCaseEvaluatorUnit {
    fn name(&self) -> &'static str {
        "edge_case_evaluator"
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
        let narrative = context
            .extra("narrative")
            .and_then(|value| value.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                crate::intel_narrative_advanced::build_edge_case_evaluator_narrative(
                    &context.user_message,
                    &context.route_decision,
                    &context.workspace_facts,
                    &context.workspace_brief,
                    &context.conversation_excerpt,
                )
            });

        let result: EdgeCaseEvaluatorAssessment =
            execute_intel_json_from_user_content(&context.client, &self.profile, narrative).await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&result)?,
            result.confidence,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("edge_cases").is_none() {
            return Err(anyhow::anyhow!("Missing 'edge_cases' field"));
        }
        if output.get("failure_modes").is_none() {
            return Err(anyhow::anyhow!("Missing 'failure_modes' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let default = EdgeCaseEvaluatorAssessment::default();
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::to_value(&default)?,
            &format!("edge case evaluator assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> Profile {
        Profile {
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
        }
    }

    #[test]
    fn test_domain_difficulty_unit_creation() {
        let unit = DomainDifficultyUnit::new(test_profile());
        assert_eq!(unit.name(), "domain_difficulty");
    }

    #[test]
    fn test_freshness_requirement_unit_creation() {
        let unit = FreshnessRequirementUnit::new(test_profile());
        assert_eq!(unit.name(), "freshness_requirement");
    }

    #[test]
    fn test_assumption_tracker_unit_creation() {
        let unit = AssumptionTrackerUnit::new(test_profile());
        assert_eq!(unit.name(), "assumption_tracker");
    }

    #[test]
    fn test_edge_case_evaluator_unit_creation() {
        let unit = EdgeCaseEvaluatorUnit::new(test_profile());
        assert_eq!(unit.name(), "edge_case_evaluator");
    }

    #[test]
    fn test_domain_difficulty_default() {
        let default = DomainDifficultyAssessment::default();
        assert_eq!(default.domain_type, "common");
        assert_eq!(default.knowledge_level, "basic");
        assert!(!default.sensitive);
        assert_eq!(default.expertise_required, "none");
    }

    #[test]
    fn test_freshness_requirement_default() {
        let default = FreshnessRequirementAssessment::default();
        assert_eq!(default.freshness_needed, "stable");
        assert_eq!(default.staleness_risk, "low");
        assert_eq!(default.update_frequency, "rare");
        assert_eq!(default.time_sensitivity, "none");
    }

    #[test]
    fn test_assumption_tracker_default() {
        let default = AssumptionTrackerAssessment::default();
        assert!(default.assumptions.is_empty());
        assert!(!default.needs_verification);
        assert_eq!(default.assumption_count, 0);
        assert_eq!(default.high_risk_count, 0);
    }

    #[test]
    fn test_edge_case_evaluator_default() {
        let default = EdgeCaseEvaluatorAssessment::default();
        assert!(default.edge_cases.is_empty());
        assert!(default.failure_modes.is_empty());
        assert!(default.hidden_deps.is_empty());
        assert_eq!(default.edge_case_count, 0);
        assert_eq!(default.high_risk_count, 0);
    }

    #[test]
    fn test_domain_difficulty_serialization() {
        let assessment = DomainDifficultyAssessment {
            domain_type: "expert".to_string(),
            knowledge_level: "advanced".to_string(),
            sensitive: true,
            expertise_required: "specific".to_string(),
            domain_label: "medical".to_string(),
            confidence: 0.85,
            entropy: 0.15,
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: DomainDifficultyAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.domain_type, "expert");
        assert!(deserialized.sensitive);
    }

    #[test]
    fn test_freshness_requirement_serialization() {
        let assessment = FreshnessRequirementAssessment {
            freshness_needed: "high".to_string(),
            staleness_risk: "high".to_string(),
            update_frequency: "frequent".to_string(),
            sources: vec!["api".to_string(), "news".to_string()],
            time_sensitivity: "high".to_string(),
            confidence: 0.9,
            entropy: 0.1,
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: FreshnessRequirementAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.freshness_needed, "high");
        assert_eq!(deserialized.sources.len(), 2);
    }

    #[test]
    fn test_assumption_tracker_serialization() {
        let assessment = AssumptionTrackerAssessment {
            assumptions: vec![AssumptionRecord {
                text: "User has Rust installed".to_string(),
                risk: "low".to_string(),
                dependency: "environment".to_string(),
                change_impact: "minor".to_string(),
                verifiable: true,
            }],
            needs_verification: true,
            assumption_count: 1,
            high_risk_count: 0,
            confidence: 0.75,
            entropy: 0.25,
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: AssumptionTrackerAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.assumptions.len(), 1);
        assert!(deserialized.needs_verification);
    }

    #[test]
    fn test_edge_case_evaluator_serialization() {
        let assessment = EdgeCaseEvaluatorAssessment {
            edge_cases: vec![EdgeCaseRecord {
                scenario: "Empty input".to_string(),
                likelihood: "likely".to_string(),
                impact: "minor".to_string(),
                mitigation: "Validate input before processing".to_string(),
            }],
            failure_modes: vec!["parse_error".to_string()],
            hidden_deps: vec!["locale_settings".to_string()],
            edge_case_count: 1,
            high_risk_count: 0,
            confidence: 0.8,
            entropy: 0.2,
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: EdgeCaseEvaluatorAssessment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.edge_cases.len(), 1);
        assert_eq!(deserialized.failure_modes.len(), 1);
    }
}
