//! @efficiency-role: domain-logic
//!
//! Foundational intel units: ComplexityAssessment, EvidenceNeeds, ActionNeeds,
//! WorkflowPlanner, FormulaSelector, ScopeBuilder, PatternSuggestion.

use crate::intel_trait::*;
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
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let req = build_intel_system_user_request(
            &self.profile,
            crate::intel_narrative::build_complexity_narrative(
                &context.user_message,
                &context.route_decision,
                &context.workspace_facts,
                &context.workspace_brief,
                &context.conversation_excerpt,
            ),
        );

        let result: serde_json::Value =
            execute_traced_intel_json_for_profile(&context.client, self.name(), &self.profile, req)
                .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        let d = &output.data;
        if d.get("complexity").is_none() {
            return Err(anyhow::anyhow!("Missing 'complexity' field"));
        }
        if d.get("risk").is_none() {
            return Err(anyhow::anyhow!("Missing 'risk' field"));
        }
        match d.get("complexity").and_then(|v| v.as_str()) {
            Some("DIRECT") | Some("INVESTIGATE") | Some("MULTISTEP") | Some("OPEN_ENDED") => {}
            other => return Err(anyhow::anyhow!("Invalid complexity value: {:?}", other)),
        }
        match d.get("risk").and_then(|v| v.as_str()) {
            Some("LOW") | Some("MEDIUM") | Some("HIGH") => {}
            other => return Err(anyhow::anyhow!("Invalid risk value: {:?}", other)),
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "complexity": "INVESTIGATE", "risk": "LOW",
                "needs_evidence": false, "needs_tools": false,
                "needs_decision": false, "needs_plan": false,
                "suggested_pattern": "reply_only",
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
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result: serde_json::Value = execute_intel_json_from_user_content(
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
        Ok(IntelOutput::success(self.name(), result, 0.9))
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
            serde_json::json!({ "needs_evidence": false, "needs_tools": false }),
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
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result: serde_json::Value = execute_intel_json_from_user_content(
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
        Ok(IntelOutput::success(self.name(), result, 0.9))
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
            serde_json::json!({ "needs_decision": false, "needs_plan": false }),
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
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result: WorkflowPlannerOutput = execute_intel_json_from_user_content(
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
        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("objective").is_none() {
            return Err(anyhow::anyhow!("Missing 'objective' field"));
        }
        if output.get("complexity").is_none() {
            return Err(anyhow::anyhow!("Missing 'complexity' field"));
        }
        if output.get("risk").is_none() {
            return Err(anyhow::anyhow!("Missing 'risk' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "objective": "Complete the user's request", "complexity": "DIRECT", "risk": "LOW",
                "needs_evidence": false,
                "scope": {
                    "objective": "Complete the user's request",
                    "focus_paths": [], "include_globs": [], "exclude_globs": [],
                    "query_terms": [], "expected_artifacts": [], "reason": "fallback scope"
                },
                "preferred_formula": "reply_only", "alternatives": [], "memory_id": "",
                "reason": "fallback workflow",
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
            return Err(anyhow::anyhow!("Missing 'suggested_pattern' field"));
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
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let complexity = serde_json::to_value(&context.complexity).unwrap_or(Value::Null);
        let result: ScopePlan = execute_intel_json_from_user_content(
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
        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("objective").is_none() {
            return Err(anyhow::anyhow!("Missing 'objective' field"));
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
            return Err(anyhow::anyhow!("Empty user message"));
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
        let result: FormulaSelection = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_formula_selector_narrative(
                &context.user_message,
                &context.route_decision,
                &complexity,
                &serde_json::from_value(scope).unwrap_or_default(),
                &memory_candidates,
                &context.conversation_excerpt,
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
        if output.get("primary").is_none() {
            return Err(anyhow::anyhow!("Missing 'primary' field"));
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
