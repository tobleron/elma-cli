//! Routing configuration and feature flags for confidence-based decisions

use serde::{Deserialize, Serialize};

/// Configuration for routing behavior, enabling migration from hardcoded to confidence-based decisions
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RoutingConfig {
    /// Enable confidence-based routing decisions (vs hardcoded heuristics)
    #[serde(default = "default_false")]
    pub confidence_based_routing: bool,

    /// Minimum confidence margin required for direct shell execution
    #[serde(default = "default_min_confidence_margin")]
    pub min_confidence_margin: f64,

    /// Maximum entropy allowed for direct shell execution
    #[serde(default = "default_max_entropy")]
    pub max_entropy: f64,

    /// Weight given to route decision confidence in final assessment (0.0-1.0)
    #[serde(default = "default_route_confidence_weight")]
    pub route_confidence_weight: f64,

    /// Weight given to workflow complexity assessment in final assessment (0.0-1.0)
    #[serde(default = "default_workflow_weight")]
    pub workflow_weight: f64,

    /// Thresholds for short-circuit chat route detection
    #[serde(default = "default_speech_chat_entropy_threshold")]
    pub speech_chat_entropy_threshold: f64,
    #[serde(default = "default_speech_chat_margin_threshold")]
    pub speech_chat_margin_threshold: f64,
    #[serde(default = "default_workflow_chat_entropy_threshold")]
    pub workflow_chat_entropy_threshold: f64,
    #[serde(default = "default_workflow_chat_margin_threshold")]
    pub workflow_chat_margin_threshold: f64,

    /// Thresholds for speech chat boost
    #[serde(default = "default_workflow_chat_boost_margin_threshold")]
    pub workflow_chat_boost_margin_threshold: f64,
    #[serde(default = "default_workflow_chat_boost_entropy_threshold")]
    pub workflow_chat_boost_entropy_threshold: f64,

    /// Thresholds for workflow confident detection
    #[serde(default = "default_workflow_confident_entropy_threshold")]
    pub workflow_confident_entropy_threshold: f64,
    #[serde(default = "default_workflow_confident_margin_threshold")]
    pub workflow_confident_margin_threshold: f64,

    /// Thresholds for speech confident chat detection
    #[serde(default = "default_speech_confident_chat_entropy_threshold")]
    pub speech_confident_chat_entropy_threshold: f64,
    #[serde(default = "default_speech_confident_chat_margin_threshold")]
    pub speech_confident_chat_margin_threshold: f64,

    /// Thresholds for speech confident instruct detection
    #[serde(default = "default_speech_confident_instruct_entropy_threshold")]
    pub speech_confident_instruct_entropy_threshold: f64,
    #[serde(default = "default_speech_confident_instruct_margin_threshold")]
    pub speech_confident_instruct_margin_threshold: f64,

    /// Thresholds for fallback to chat detection
    #[serde(default = "default_fallback_chat_margin_threshold")]
    pub fallback_chat_margin_threshold: f64,
    #[serde(default = "default_fallback_chat_entropy_threshold")]
    pub fallback_chat_entropy_threshold: f64,
    #[serde(default = "default_hard_fallback_margin_threshold")]
    pub hard_fallback_margin_threshold: f64,
}

fn default_false() -> bool {
    false
}

fn default_min_confidence_margin() -> f64 {
    0.3
}

fn default_max_entropy() -> f64 {
    0.5
}

fn default_route_confidence_weight() -> f64 {
    0.6
}

fn default_workflow_weight() -> f64 {
    0.4
}

// Speech chat route thresholds (matching original values)
fn default_speech_chat_entropy_threshold() -> f64 {
    0.20
}
fn default_speech_chat_margin_threshold() -> f64 {
    0.70
}
fn default_workflow_chat_entropy_threshold() -> f64 {
    0.20
}
fn default_workflow_chat_margin_threshold() -> f64 {
    0.70
}

// Speech chat boost thresholds (matching original values)
fn default_workflow_chat_boost_margin_threshold() -> f64 {
    0.15
}
fn default_workflow_chat_boost_entropy_threshold() -> f64 {
    0.50
}

// Workflow confident thresholds (matching original values)
fn default_workflow_confident_entropy_threshold() -> f64 {
    0.60
}
fn default_workflow_confident_margin_threshold() -> f64 {
    0.30
}

// Speech confident chat thresholds (matching original values)
fn default_speech_confident_chat_entropy_threshold() -> f64 {
    0.40
}
fn default_speech_confident_chat_margin_threshold() -> f64 {
    0.50
}

// Speech confident instruct thresholds (matching original values)
fn default_speech_confident_instruct_entropy_threshold() -> f64 {
    0.60
}
fn default_speech_confident_instruct_margin_threshold() -> f64 {
    0.30
}

// Fallback to chat thresholds (matching original values)
fn default_fallback_chat_margin_threshold() -> f64 {
    0.20
}
fn default_fallback_chat_entropy_threshold() -> f64 {
    0.60
}
fn default_hard_fallback_margin_threshold() -> f64 {
    0.12
}

impl RoutingConfig {
    /// Returns true if confidence-based routing is enabled
    pub fn is_confidence_based_routing_enabled(&self) -> bool {
        self.confidence_based_routing
    }

    /// Returns whether a route decision meets confidence thresholds for direct execution
    pub fn meets_confidence_thresholds(
        &self,
        route_decision: &crate::types_core::RouteDecision,
    ) -> bool {
        route_decision.margin >= self.min_confidence_margin
            && route_decision.entropy <= self.max_entropy
    }

    /// Calculates a combined confidence score considering multiple factors
    pub fn calculate_combined_confidence(
        &self,
        route_decision: &crate::types_core::RouteDecision,
        workflow_plan: Option<&crate::types_api::WorkflowPlannerOutput>,
        complexity: &crate::types_api::ComplexityAssessment,
    ) -> f64 {
        // Base score from route decision confidence (margin - entropy)
        let route_confidence = (route_decision.margin - route_decision.entropy).max(0.0);

        // Workflow compatibility score
        let workflow_score = self.calculate_workflow_compatibility(workflow_plan, complexity);

        // Weighted combination
        (route_confidence * self.route_confidence_weight) + (workflow_score * self.workflow_weight)
    }

    /// Calculates workflow compatibility score (0.0-1.0)
    fn calculate_workflow_compatibility(
        &self,
        workflow_plan: Option<&crate::types_api::WorkflowPlannerOutput>,
        complexity: &crate::types_api::ComplexityAssessment,
    ) -> f64 {
        let mut score = 0.0;
        let mut factors = 0;

        // Check complexity assessment
        if complexity.complexity.eq_ignore_ascii_case("DIRECT") {
            score += 0.25;
        }
        factors += 1;

        if complexity.risk.eq_ignore_ascii_case("LOW") {
            score += 0.25;
        }
        factors += 1;

        if !complexity.needs_plan {
            score += 0.25;
        }
        factors += 1;

        if !complexity.needs_decision {
            score += 0.25;
        }
        factors += 1;

        // Check workflow plan if available
        if let Some(plan) = workflow_plan {
            if plan.complexity.trim().is_empty() || plan.complexity.eq_ignore_ascii_case("DIRECT") {
                score += 0.25;
            }
            factors += 1;

            if plan.risk.trim().is_empty() || plan.risk.eq_ignore_ascii_case("LOW") {
                score += 0.25;
            }
            factors += 1;
        }

        // Return average score, or 0.5 if no factors (neutral)
        if factors > 0 {
            score / factors as f64
        } else {
            0.5
        }
    }
}
