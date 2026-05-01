//! @efficiency-role: domain-logic
//!
//! Intent Analysis Intel Units: IntentSurfaceUnit, IntentRealUnit, UserExpectationUnit

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Intent Surface Unit
// ============================================================================

/// Intent Surface Unit
///
/// Analyzes the surface-level intent from user message (literal request, output type, format preferences).
pub(crate) struct IntentSurfaceUnit {
    profile: Profile,
}

impl IntentSurfaceUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for IntentSurfaceUnit {
    fn name(&self) -> &'static str {
        "intent_surface"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(IntelError::EmptyUserMessage.into());
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative_intent::build_surface_intent_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
        )
        .await?;
        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("surface_intent").is_none() {
            return Err(IntelError::MissingField("surface_intent".to_string()).into());
        }
        if output.get("output_type").is_none() {
            return Err(IntelError::MissingField("output_type".to_string()).into());
        }
        if output.get("format_pref").is_none() {
            return Err(IntelError::MissingField("format_pref".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "surface_intent": "question",
                "output_type": "explanation",
                "format_pref": "paragraph",
                "choice": "1",
                "label": "QUESTION",
                "reason": "fallback: default surface intent",
                "entropy": 0.5
            }),
            &format!("intent surface failed: {}", error),
        ))
    }
}

// ============================================================================
// Intent Real Unit
// ============================================================================

/// Intent Real Unit
///
/// Infers the real underlying intent (problem, goals, decision needs, frustration).
pub(crate) struct IntentRealUnit {
    profile: Profile,
}

impl IntentRealUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for IntentRealUnit {
    fn name(&self) -> &'static str {
        "intent_real"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(IntelError::EmptyUserMessage.into());
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative_intent::build_real_intent_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
        )
        .await?;
        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("real_intent").is_none() {
            return Err(IntelError::MissingField("real_intent".to_string()).into());
        }
        if output.get("problem_type").is_none() {
            return Err(IntelError::MissingField("problem_type".to_string()).into());
        }
        if output.get("decision_needed").is_none() {
            return Err(IntelError::MissingField("decision_needed".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "real_intent": "learn",
                "problem_type": "specific",
                "decision_needed": false,
                "choice": "1",
                "label": "LEARN",
                "reason": "fallback: default real intent",
                "entropy": 0.5
            }),
            &format!("intent real failed: {}", error),
        ))
    }
}

// ============================================================================
// User Expectation Unit
// ============================================================================

/// User Expectation Unit
///
/// Determines user expectations (advice type, depth, certainty, effort level).
pub(crate) struct UserExpectationUnit {
    profile: Profile,
}

impl UserExpectationUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for UserExpectationUnit {
    fn name(&self) -> &'static str {
        "user_expectation"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(IntelError::EmptyUserMessage.into());
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative_intent::build_user_expectation_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
        )
        .await?;
        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("expectation_type").is_none() {
            return Err(IntelError::MissingField("expectation_type".to_string()).into());
        }
        if output.get("depth_level").is_none() {
            return Err(IntelError::MissingField("depth_level".to_string()).into());
        }
        if output.get("certainty_pref").is_none() {
            return Err(IntelError::MissingField("certainty_pref".to_string()).into());
        }
        if output.get("effort_level").is_none() {
            return Err(IntelError::MissingField("effort_level".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "expectation_type": "practical",
                "depth_level": "quick",
                "certainty_pref": "high",
                "effort_level": "low",
                "choice": "1",
                "label": "PRACTICAL_QUICK",
                "reason": "fallback: default expectations",
                "entropy": 0.5
            }),
            &format!("user expectation failed: {}", error),
        ))
    }
}
