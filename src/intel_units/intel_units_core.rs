//! @efficiency-role: domain-logic
//!
//! Foundational intel units: ComplexityAssessment, EvidenceNeeds, ActionNeeds,
//! WorkflowPlanner, FormulaSelector, ScopeBuilder, PatternSuggestion.

use crate::intel_trait::*;
use crate::intel_units::*;
use crate::*;
use serde_json::Value;

// --- ComplexityAssessment ---

pub(crate) struct ComplexityAssessmentUnit {
    profile: Profile,
}
impl ComplexityAssessmentUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ComplexityAssessmentUnit {
    fn name(&self) -> &'static str {
        "complexity_assessment"
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_complexity_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
                &context
                    .intent_surface
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
                &context
                    .intent_real
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
                &context
                    .user_expectation
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
            ),
        )
        .await?;

        // Parse DSL result into assessment fields
        let complexity = dsl_result
            .get("complexity")
            .and_then(|v| v.as_str())
            .unwrap_or("INVESTIGATE")
            .to_string();

        let risk = dsl_result
            .get("risk")
            .and_then(|v| v.as_str())
            .unwrap_or("LOW")
            .to_string();

        let needs_evidence = dsl_result
            .get("needs_evidence")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let needs_tools = dsl_result
            .get("needs_tools")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let needs_decision = dsl_result
            .get("needs_decision")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let needs_plan = dsl_result
            .get("needs_plan")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let suggested_pattern = dsl_result
            .get("suggested_pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("reply_only")
            .to_string();

        let mut result = serde_json::Map::new();
        result.insert(
            "complexity".to_string(),
            serde_json::Value::String(complexity),
        );
        result.insert("risk".to_string(), serde_json::Value::String(risk));
        result.insert(
            "needs_evidence".to_string(),
            serde_json::Value::Bool(needs_evidence),
        );
        result.insert(
            "needs_tools".to_string(),
            serde_json::Value::Bool(needs_tools),
        );
        result.insert(
            "needs_decision".to_string(),
            serde_json::Value::Bool(needs_decision),
        );
        result.insert(
            "needs_plan".to_string(),
            serde_json::Value::Bool(needs_plan),
        );
        result.insert(
            "suggested_pattern".to_string(),
            serde_json::Value::String(suggested_pattern),
        );

        Ok(IntelOutput::success(
            self.name(),
            serde_json::Value::Object(result),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        let d = &output.data;
        if d.get("complexity").is_none() {
            return Err(IntelError::MissingField("complexity".to_string()).into());
        }
        if d.get("risk").is_none() {
            return Err(IntelError::MissingField("risk".to_string()).into());
        }
        if d.get("needs_evidence").is_none() {
            return Err(IntelError::MissingField("needs_evidence".to_string()).into());
        }
        if d.get("needs_tools").is_none() {
            return Err(IntelError::MissingField("needs_tools".to_string()).into());
        }
        if d.get("needs_decision").is_none() {
            return Err(IntelError::MissingField("needs_decision".to_string()).into());
        }
        if d.get("needs_plan").is_none() {
            return Err(IntelError::MissingField("needs_plan".to_string()).into());
        }
        if d.get("suggested_pattern").is_none() {
            return Err(IntelError::MissingField("suggested_pattern".to_string()).into());
        }
        match d.get("complexity").and_then(|v| v.as_str()) {
            Some("DIRECT") | Some("INVESTIGATE") | Some("MULTISTEP") | Some("OPEN_ENDED") => {}
            other => {
                return Err(IntelError::InvalidValue(
                    "complexity".to_string(),
                    format!("{:?}", other),
                )
                .into())
            }
        }
        match d.get("risk").and_then(|v| v.as_str()) {
            Some("LOW") | Some("MEDIUM") | Some("HIGH") => {}
            other => {
                return Err(
                    IntelError::InvalidValue("risk".to_string(), format!("{:?}", other)).into(),
                )
            }
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "complexity": "INVESTIGATE".to_string(),
                "risk": "LOW".to_string(),
                "needs_evidence": false,
                "needs_tools": false,
                "needs_decision": false,
                "needs_plan": false,
                "suggested_pattern": "reply_only".to_string(),
            }),
            &format!("complexity assessment failed: {}", error),
        ))
    }
}

// --- EvidenceNeeds ---

pub(crate) struct EvidenceNeedsUnit {
    profile: Profile,
}
impl EvidenceNeedsUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceNeedsUnit {
    fn name(&self) -> &'static str {
        "evidence_needs_assessment"
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_evidence_needs_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
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
            return Err(IntelError::MissingField("needs_evidence".to_string()).into());
        }
        if output.get("needs_tools").is_none() {
            return Err(IntelError::MissingField("needs_tools".to_string()).into());
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
            &format!("evidence needs assessment failed: {}", error),
        ))
    }
}

// --- ActionNeeds ---

pub(crate) struct ActionNeedsUnit {
    profile: Profile,
}
impl ActionNeedsUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ActionNeedsUnit {
    fn name(&self) -> &'static str {
        "action_needs_assessment"
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_action_needs_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
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
            return Err(IntelError::MissingField("needs_decision".to_string()).into());
        }
        if output.get("needs_plan").is_none() {
            return Err(IntelError::MissingField("needs_plan".to_string()).into());
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
            &format!("action needs assessment failed: {}", error),
        ))
    }
}

// --- WorkflowPlanner ---

pub(crate) struct WorkflowPlannerUnit {
    profile: Profile,
}
impl WorkflowPlannerUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for WorkflowPlannerUnit {
    fn name(&self) -> &'static str {
        "workflow_planner"
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
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_workflow_planner_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
        )
        .await?;

        let result: WorkflowPlannerOutput = serde_json::from_value(dsl_result)?;
        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("objective").is_none() {
            return Err(IntelError::MissingField("objective".to_string()).into());
        }
        if output.get("complexity").is_none() {
            return Err(IntelError::MissingField("complexity".to_string()).into());
        }
        if output.get("risk").is_none() {
            return Err(IntelError::MissingField("risk".to_string()).into());
        }
        if output.get("needs_evidence").is_none() {
            return Err(IntelError::MissingField("needs_evidence".to_string()).into());
        }
        if output.get("scope").is_none() {
            return Err(IntelError::MissingField("scope".to_string()).into());
        }
        if output.get("preferred_formula").is_none() {
            return Err(IntelError::MissingField("preferred_formula".to_string()).into());
        }
        if output.get("memory_id").is_none() {
            return Err(IntelError::MissingField("memory_id".to_string()).into());
        }
        if output.get("reason").is_none() {
            return Err(IntelError::MissingField("reason".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "objective": "Complete the user's request".to_string(),
                "complexity": "INVESTIGATE".to_string(),
                "risk": "LOW".to_string(),
                "needs_evidence": false,
                "scope": {
                    "objective": "Complete the user's request".to_string(),
                    "focus_paths": Vec::<String>::new(),
                    "include_globs": Vec::<String>::new(),
                    "exclude_globs": Vec::<String>::new(),
                    "query_terms": Vec::<String>::new(),
                    "expected_artifacts": Vec::<String>::new(),
                    "reason": "fallback scope".to_string()
                },
                "preferred_formula": "reply_only".to_string(),
                "alternatives": Vec::<String>::new(),
                "memory_id": "".to_string(),
                "reason": "fallback workflow".to_string(),
            }),
            &format!("workflow planner failed: {}", error),
        ))
    }
}

// --- PatternSuggestion ---

pub(crate) struct PatternSuggestionUnit {
    profile: Profile,
}
impl PatternSuggestionUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for PatternSuggestionUnit {
    fn name(&self) -> &'static str {
        "pattern_suggestion"
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
            serde_json::json!({
                "user_message": context.user_message,
                "route": context.route_decision.route,
                "conversation": conversation_excerpt(&context.conversation_excerpt, 12),
            })
            .to_string(),
        )
        .await?;
        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("suggested_pattern").is_none() {
            return Err(IntelError::MissingField("suggested_pattern".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({ "suggested_pattern": "reply_only" }),
            &format!("pattern suggestion failed: {}", error),
        ))
    }
}

// --- ScopeBuilder ---

pub(crate) struct ScopeBuilderUnit {
    profile: Profile,
}
impl ScopeBuilderUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ScopeBuilderUnit {
    fn name(&self) -> &'static str {
        "scope_builder"
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
        let complexity = serde_json::to_value(&context.complexity).unwrap_or(Value::Null);
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_scope_builder_narrative(
                &context.user_message,
                &context.route_decision,
                &complexity,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
        )
        .await?;
        let result: ScopePlan = serde_json::from_value(dsl_result)?;
        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("objective").is_none() {
            return Err(IntelError::MissingField("objective".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "objective": "Complete the user's request",
                "focus_paths": Vec::<String>::new(), "include_globs": Vec::<String>::new(),
                "exclude_globs": Vec::<String>::new(), "query_terms": Vec::<String>::new(),
                "expected_artifacts": Vec::<String>::new(),
            }),
            &format!("scope builder failed: {}", error),
        ))
    }
}

// --- FormulaSelector ---

pub(crate) struct FormulaSelectorUnit {
    profile: Profile,
}
impl FormulaSelectorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for FormulaSelectorUnit {
    fn name(&self) -> &'static str {
        "formula_selector"
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
        let scope = context
            .extra("scope")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(ScopePlan::default()));
        let memory_candidates = context
            .extra("memory_candidates")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let complexity = serde_json::to_value(&context.complexity).unwrap_or(Value::Null);
        let dsl_result = execute_intel_dsl_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_formula_selector_narrative(
                &context.user_message,
                &context.route_decision,
                &complexity,
                &serde_json::from_value(scope).unwrap_or_default(),
                &memory_candidates,
                &context.conversation_excerpt,
                &context
                    .intent_surface
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
                &context
                    .intent_real
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
                &context
                    .user_expectation
                    .clone()
                    .unwrap_or(serde_json::Value::Null),
            ),
        )
        .await?;
        let result: FormulaSelection = serde_json::from_value(dsl_result)?;
        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("primary").is_none() {
            return Err(IntelError::MissingField("primary".to_string()).into());
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "primary": "reply_only", "alternatives": Vec::<String>::new(),
                "reason": "fallback selection".to_string(), "memory_id": "",
            }),
            &format!("formula selector failed: {}", error),
        ))
    }
}
