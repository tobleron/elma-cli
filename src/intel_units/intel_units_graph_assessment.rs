//! @efficiency-role: domain-logic
//!
//! Graph complexity assessment intel unit — Task 389.
//! Determines whether a request needs pyramid work graph decomposition.
//! Output is a 3-field JSON object compliant with Task 378 limits.

use crate::intel_trait::*;
use crate::*;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// 3-field output for graph complexity assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GraphComplexityOutput {
    /// "simple", "moderate", or "complex"
    pub complexity: String,
    /// Whether the request needs pyramid decomposition
    pub needs_graph: bool,
    /// Why this assessment was made
    pub reason: String,
}

/// Assesses whether a user request needs pyramid work graph decomposition.
pub(crate) struct GraphComplexityUnit {
    profile: Profile,
}

impl GraphComplexityUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for GraphComplexityUnit {
    fn name(&self) -> &'static str {
        "graph_complexity"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result: GraphComplexityOutput = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            context.user_message.clone(),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.85,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get_str("complexity").is_none_or(|s| s.is_empty()) {
            return Err(anyhow!("Missing complexity field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "complexity": "simple",
                "needs_graph": false,
                "reason": format!("fallback: {}", error),
            }),
            &format!("graph complexity assessment failed: {}", error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_complexity_output_deserialization() {
        let json = serde_json::json!({
            "complexity": "complex",
            "needs_graph": true,
            "reason": "multi-step task with dependencies"
        });
        let output: GraphComplexityOutput = serde_json::from_value(json).unwrap();
        assert_eq!(output.complexity, "complex");
        assert!(output.needs_graph);
        assert!(!output.reason.is_empty());
    }

    #[test]
    fn test_graph_complexity_output_simple() {
        let json = serde_json::json!({
            "complexity": "simple",
            "needs_graph": false,
            "reason": "single factual question"
        });
        let output: GraphComplexityOutput = serde_json::from_value(json).unwrap();
        assert_eq!(output.complexity, "simple");
        assert!(!output.needs_graph);
    }

    #[test]
    fn test_graph_complexity_output_requires_all_fields() {
        // Missing reason should fail
        let json = serde_json::json!({
            "complexity": "moderate",
            "needs_graph": true
        });
        let result: Result<GraphComplexityOutput, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_graph_complexity_unit_creation() {
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
            timeout_s: 30,
            system_prompt: "Assess the complexity of this request.".to_string(),
        };
        let unit = GraphComplexityUnit::new(profile);
        assert_eq!(unit.name(), "graph_complexity");
    }

    #[test]
    fn test_graph_complexity_fallback_defaults() {
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
            timeout_s: 30,
            system_prompt: "test".to_string(),
        };
        let unit = GraphComplexityUnit::new(profile);
        let fallback = unit
            .fallback(
                &IntelContext::new(
                    String::new(),
                    crate::RouteDecision::default(),
                    String::new(),
                    String::new(),
                    Vec::new(),
                    reqwest::Client::new(),
                ),
                "test error",
            )
            .unwrap();
        assert_eq!(fallback.get_str("complexity").unwrap(), "simple");
        assert_eq!(fallback.get_bool("needs_graph").unwrap(), false);
    }
}
