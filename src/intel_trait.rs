//! @efficiency-role: domain-logic
//!
//! Intel Unit Trait Module
//!
//! Defines the standard interface for all intel (intelligence/reasoning) units.
//! Each intel unit follows a consistent pattern:
//! - pre_flight: Validate inputs before model call
//! - execute: Call model with dedicated prompt
//! - post_flight: Validate output before use
//! - fallback: Provide safe defaults on failure

use crate::*;

// ============================================================================
// Intel Context
// ============================================================================

/// Context passed to all intel units
///
/// Provides standardized input structure for reasoning units.
#[derive(Debug, Clone)]
pub(crate) struct IntelContext {
    /// Original user message
    pub user_message: String,

    /// Route decision with probability distributions
    pub route_decision: RouteDecision,

    /// Workspace facts (file tree, recent files, etc.)
    pub workspace_facts: String,

    /// Workspace brief (project summary, key files)
    pub workspace_brief: String,

    /// Conversation excerpt (last N messages)
    pub conversation_excerpt: Vec<ChatMessage>,

    /// Complexity assessment (may be set by prior unit)
    pub complexity: Option<ComplexityAssessment>,

    /// Shared HTTP client for all intel units (prevents connection pool exhaustion)
    pub client: reqwest::Client,
}

impl IntelContext {
    /// Create new context with minimal required fields
    pub fn new(
        user_message: String,
        route_decision: RouteDecision,
        workspace_facts: String,
        workspace_brief: String,
        conversation_excerpt: Vec<ChatMessage>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            user_message,
            route_decision,
            workspace_facts,
            workspace_brief,
            conversation_excerpt,
            complexity: None,
            client,
        }
    }

    /// Set complexity assessment (for units that depend on it)
    pub fn with_complexity(mut self, complexity: ComplexityAssessment) -> Self {
        self.complexity = Some(complexity);
        self
    }
}

// ============================================================================
// Intel Output
// ============================================================================

/// Generic output from intel units
///
/// Contains raw data that specialized units can wrap with typed accessors.
#[derive(Debug, Clone)]
pub(crate) struct IntelOutput {
    /// Unit name for tracing/logging
    pub unit_name: String,

    /// Raw output data (JSON value)
    pub data: serde_json::Value,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,

    /// Whether fallback was used instead of model output
    pub fallback_used: bool,

    /// Error message if fallback was used
    pub fallback_reason: Option<String>,
}

impl IntelOutput {
    /// Create successful output from model
    pub fn success(unit_name: &str, data: serde_json::Value, confidence: f64) -> Self {
        Self {
            unit_name: unit_name.to_string(),
            data,
            confidence,
            fallback_used: false,
            fallback_reason: None,
        }
    }

    /// Create fallback output when model fails
    pub fn fallback(unit_name: &str, data: serde_json::Value, reason: &str) -> Self {
        Self {
            unit_name: unit_name.to_string(),
            data,
            confidence: 0.5, // Neutral confidence for fallback
            fallback_used: true,
            fallback_reason: Some(reason.to_string()),
        }
    }

    /// Get a field from the output data
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Get a string field from the output data
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    /// Get a bool field from the output data
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| v.as_bool())
    }

    /// Get a f64 field from the output data
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.data.get(key).and_then(|v| v.as_f64())
    }

    /// Check if output contains standard fields (label or choice)
    pub fn is_standard_format(&self) -> bool {
        self.get("label").is_some() || self.get("choice").is_some()
    }
}

impl Default for IntelOutput {
    fn default() -> Self {
        Self {
            unit_name: "unknown".to_string(),
            data: serde_json::Value::Null,
            confidence: 0.5,
            fallback_used: false,
            fallback_reason: None,
        }
    }
}

// ============================================================================
// Intel Unit Trait
// ============================================================================

/// Common interface for all intel units
///
/// Each intel unit implements this trait to provide:
/// - Consistent input/output structure
/// - Pre-flight validation
/// - Post-flight verification
/// - Fallback handling
///
/// # Example
/// ```rust,ignore
/// pub struct ComplexityAssessmentUnit {
///     profile: Profile,
/// }
///
/// impl IntelUnit for ComplexityAssessmentUnit {
///     fn name(&self) -> &'static str { "complexity_assessment" }
///     fn profile(&self) -> &Profile { &self.profile }
///     
///     async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
///         // Call model with dedicated prompt
///         // Parse JSON output
///         // Return structured result
///     }
/// }
/// ```
pub(crate) trait IntelUnit: Send + Sync {
    /// Unit name for tracing/logging
    fn name(&self) -> &'static str;

    /// Profile configuration for this unit
    fn profile(&self) -> &Profile;

    /// Pre-flight validation (context, inputs)
    ///
    /// Called before execute() to validate inputs.
    /// Return Err if unit cannot proceed (missing required context, etc.)
    ///
    /// Default implementation always succeeds (units can override).
    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        Ok(())
    }

    /// Execute the intel unit (model call)
    ///
    /// Main execution logic. Should:
    /// 1. Build chat request using self.profile()
    /// 2. Call model via chat_json_with_repair_timeout()
    /// 3. Parse and validate output
    /// 4. Return IntelOutput with structured data
    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput>;

    /// Post-flight verification (output validation)
    ///
    /// Called after execute() to validate output.
    /// Return Err if output is unusable (missing required fields, invalid values, etc.)
    ///
    /// Default implementation always succeeds (units can override).
    fn post_flight(&self, _output: &IntelOutput) -> Result<()> {
        Ok(())
    }

    /// Fallback when execute() or post_flight() fails
    ///
    /// Provides safe default output when model fails.
    /// Should return conservative but usable values.
    ///
    /// Default implementation returns generic fallback (units SHOULD override).
    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        // Generic fallback - units should override with domain-specific defaults
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::Value::Null,
            &format!("generic fallback: {}", error),
        ))
    }

    /// Execute with automatic fallback handling
    ///
    /// Convenience method that:
    /// 1. Runs pre_flight()
    /// 2. Runs execute()
    /// 3. Runs post_flight()
    /// 4. On any failure, runs fallback()
    ///
    /// Returns Ok(output) in all cases (fallback ensures success).
    async fn execute_with_fallback(&self, context: &IntelContext) -> Result<IntelOutput> {
        // Pre-flight validation
        if let Err(error) = self.pre_flight(context) {
            trace_verbose(
                true, // verbose
                &format!("intel_{}_preflight_failed error={}", self.name(), error),
            );
            return self.fallback(context, &format!("pre-flight: {}", error));
        }

        // Execute model call
        match self.execute(context).await {
            Ok(output) => {
                // Post-flight verification
                match self.post_flight(&output) {
                    Ok(()) => {
                        // Success!
                        Ok(output)
                    }
                    Err(error) => {
                        trace_verbose(
                            true,
                            &format!("intel_{}_postflight_failed error={}", self.name(), error),
                        );
                        self.fallback(context, &format!("post-flight: {}", error))
                    }
                }
            }
            Err(error) => {
                trace_verbose(
                    true,
                    &format!("intel_{}_execute_failed error={}", self.name(), error),
                );
                self.fallback(context, &format!("execution: {}", error))
            }
        }
    }
}

// ============================================================================
// Specialized Output Types
// ============================================================================

/// Specialized output for complexity assessment
#[derive(Debug, Clone)]
pub(crate) struct ComplexityOutput {
    pub assessment: ComplexityAssessment,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl ComplexityOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        // Parse ComplexityAssessment from IntelOutput data
        let assessment: ComplexityAssessment = serde_json::from_value(output.data.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse complexity assessment: {}", e))?;

        Ok(Self {
            assessment,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

/// Specialized output for evidence needs assessment
#[derive(Debug, Clone)]
pub(crate) struct EvidenceNeedsOutput {
    pub needs_evidence: bool,
    pub needs_tools: bool,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl EvidenceNeedsOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let needs_evidence = output.get_bool("needs_evidence").unwrap_or(false);
        let needs_tools = output.get_bool("needs_tools").unwrap_or(false);

        Ok(Self {
            needs_evidence,
            needs_tools,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

/// Specialized output for action needs assessment
#[derive(Debug, Clone)]
pub(crate) struct ActionNeedsOutput {
    pub needs_decision: bool,
    pub needs_plan: bool,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl ActionNeedsOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let needs_decision = output.get_bool("needs_decision").unwrap_or(false);
        let needs_plan = output.get_bool("needs_plan").unwrap_or(false);

        Ok(Self {
            needs_decision,
            needs_plan,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

/// Specialized output for pattern suggestion
#[derive(Debug, Clone)]
pub(crate) struct PatternSuggestionOutput {
    pub suggested_pattern: String,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl PatternSuggestionOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let suggested_pattern = output
            .get_str("suggested_pattern")
            .unwrap_or("reply_only")
            .to_string();

        Ok(Self {
            suggested_pattern,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Trace fallback usage for metrics
pub(crate) fn trace_fallback(unit_name: &str, error: &str) {
    // Log fallback usage for debugging
    // In production, this would also update metrics
    eprintln!("[INTEL_FALLBACK] unit={} error={}", unit_name, error);
}

/// Trace verbose output (only when verbose mode enabled)
fn trace_verbose(verbose: bool, message: &str) {
    if verbose {
        eprintln!("[INTEL_VERBOSE] {}", message);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel_output_success() {
        let data = serde_json::json!({"complexity": "DIRECT", "risk": "LOW"});
        let output = IntelOutput::success("test_unit", data.clone(), 0.9);

        assert_eq!(output.unit_name, "test_unit");
        assert_eq!(output.confidence, 0.9);
        assert!(!output.fallback_used);
        assert_eq!(output.get_str("complexity"), Some("DIRECT"));
    }

    #[test]
    fn test_intel_output_fallback() {
        let data = serde_json::json!({"complexity": "INVESTIGATE", "risk": "LOW"});
        let output = IntelOutput::fallback("test_unit", data.clone(), "model timeout");

        assert_eq!(output.unit_name, "test_unit");
        assert_eq!(output.confidence, 0.5);
        assert!(output.fallback_used);
        assert_eq!(output.fallback_reason, Some("model timeout".to_string()));
    }

    #[test]
    fn test_intel_output_field_accessors() {
        let data = serde_json::json!({
            "complexity": "DIRECT",
            "risk": "LOW",
            "needs_evidence": false,
            "confidence": 0.95
        });
        let output = IntelOutput::success("test_unit", data, 0.9);

        assert_eq!(output.get_str("complexity"), Some("DIRECT"));
        assert_eq!(output.get_str("risk"), Some("LOW"));
        assert_eq!(output.get_bool("needs_evidence"), Some(false));
        assert_eq!(output.get_f64("confidence"), Some(0.95));
        assert_eq!(output.get_str("nonexistent"), None);
    }

    #[test]
    fn test_intel_context_builder() {
        let route_decision = RouteDecision {
            route: "CHAT".to_string(),
            source: "test".to_string(),
            distribution: vec![("CHAT".to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: ProbabilityDecision {
                choice: "CHAT".to_string(),
                source: "test".to_string(),
                distribution: vec![("CHAT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            workflow: ProbabilityDecision {
                choice: "CHAT".to_string(),
                source: "test".to_string(),
                distribution: vec![("CHAT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            mode: ProbabilityDecision {
                choice: "INSPECT".to_string(),
                source: "test".to_string(),
                distribution: vec![("INSPECT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
        };

        let client = reqwest::Client::new();
        let context = IntelContext::new(
            "test message".to_string(),
            route_decision.clone(),
            "workspace facts".to_string(),
            "workspace brief".to_string(),
            vec![],
            client.clone(),
        );

        assert_eq!(context.user_message, "test message");
        assert_eq!(context.route_decision.route, "CHAT");
        assert!(context.complexity.is_none());

        let context_with_complexity = context.with_complexity(ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        });

        assert!(context_with_complexity.complexity.is_some());
    }
}
