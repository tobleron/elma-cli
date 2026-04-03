//! @efficiency-role: domain-logic
//!
//! Intel Units
//!
//! This module contains Elma's trait-based intel units.

use crate::intel_trait::*;
use crate::*;
use serde_json::Value;

// ============================================================================
// Complexity Assessment Unit
// ============================================================================

/// Complexity Assessment Intel Unit
///
/// Assesses task complexity and risk level.
pub(crate) struct ComplexityAssessmentUnit {
    profile: Profile,
}

impl ComplexityAssessmentUnit {
    /// Create new unit with dedicated profile
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
        let data = &output.data;

        if data.get("complexity").is_none() {
            return Err(anyhow::anyhow!("Missing 'complexity' field"));
        }

        if data.get("risk").is_none() {
            return Err(anyhow::anyhow!("Missing 'risk' field"));
        }

        let complexity = data.get("complexity").and_then(|v| v.as_str());
        match complexity {
            Some("DIRECT") | Some("INVESTIGATE") | Some("MULTISTEP") | Some("OPEN_ENDED") => {}
            other => {
                return Err(anyhow::anyhow!("Invalid complexity value: {:?}", other));
            }
        }

        let risk = data.get("risk").and_then(|v| v.as_str());
        match risk {
            Some("LOW") | Some("MEDIUM") | Some("HIGH") => {}
            other => {
                return Err(anyhow::anyhow!("Invalid risk value: {:?}", other));
            }
        }

        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "complexity": "INVESTIGATE",
                "risk": "LOW",
                "needs_evidence": false,
                "needs_tools": false,
                "needs_decision": false,
                "needs_plan": false,
                "suggested_pattern": "reply_only",
            }),
            &format!("complexity assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Evidence Needs Unit
// ============================================================================

/// Evidence Needs Assessment Intel Unit
///
/// Assesses if a task requires workspace evidence and tools.
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
            serde_json::json!({
                "needs_evidence": false,
                "needs_tools": false,
            }),
            &format!("evidence needs assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Action Needs Unit
// ============================================================================

/// Action Needs Assessment Intel Unit
///
/// Assesses if a task requires decision or planning.
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
            serde_json::json!({
                "needs_decision": false,
                "needs_plan": false,
            }),
            &format!("action needs assessment failed: {}", error),
        ))
    }
}

// ============================================================================
// Workflow Planner Unit
// ============================================================================

/// Workflow Planner Intel Unit
///
/// Plans workflow scope, evidence needs, complexity, and reason.
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
                "objective": "Complete the user's request",
                "complexity": "DIRECT",
                "risk": "LOW",
                "needs_evidence": false,
                "scope": {
                    "objective": "Complete the user's request",
                    "focus_paths": [],
                    "include_globs": [],
                    "exclude_globs": [],
                    "query_terms": [],
                    "expected_artifacts": [],
                    "reason": "fallback scope"
                },
                "preferred_formula": "reply_only",
                "alternatives": [],
                "memory_id": "",
                "reason": "fallback workflow",
            }),
            &format!("workflow planner failed: {}", error),
        ))
    }
}

// ============================================================================
// Pattern Suggestion Unit
// ============================================================================

/// Pattern Suggestion Intel Unit
///
/// Suggests the best reasoning pattern (formula) for a task.
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
            serde_json::json!({
                "suggested_pattern": "reply_only",
            }),
            &format!("pattern suggestion failed: {}", error),
        ))
    }
}

// ============================================================================
// Scope Builder Unit
// ============================================================================

/// Scope Builder Intel Unit
///
/// Builds the scope plan for a task (objective, focus paths, etc.).
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
                "focus_paths": Vec::<String>::new(),
                "include_globs": Vec::<String>::new(),
                "exclude_globs": Vec::<String>::new(),
                "query_terms": Vec::<String>::new(),
                "expected_artifacts": Vec::<String>::new(),
            }),
            &format!("scope builder failed: {}", error),
        ))
    }
}

// ============================================================================
// Formula Selector Unit
// ============================================================================

/// Formula Selector Intel Unit
///
/// Selects the best reasoning formula for a task.
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
                "primary": "reply_only",
                "alternatives": Vec::<String>::new(),
                "reason": "fallback selection".to_string(),
                "memory_id": "",
            }),
            &format!("formula selector failed: {}", error),
        ))
    }
}

// ============================================================================
// Selector Unit
// ============================================================================

/// Selector Intel Unit
///
/// Selects items from evidence based on instructions.
pub(crate) struct SelectorUnit {
    profile: Profile,
}

impl SelectorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

// ============================================================================
// JSON Repair Unit
// ============================================================================

/// JSON Repair Intel Unit
///
/// Repairs malformed JSON using a dedicated profile.
pub(crate) struct JsonRepairUnit {
    profile: Profile,
}

impl JsonRepairUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }

    pub async fn repair_with_fallback(
        &self,
        client: &reqwest::Client,
        chat_url: &Url,
        original_json: &str,
        problems: &[String],
    ) -> Result<String> {
        let route_decision = RouteDecision {
            route: "DECIDE".to_string(),
            source: "json_repair".to_string(),
            distribution: vec![("DECIDE".to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: ProbabilityDecision {
                choice: "INSTRUCT".to_string(),
                source: "json_repair".to_string(),
                distribution: vec![("INSTRUCT".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            workflow: ProbabilityDecision {
                choice: "WORKFLOW".to_string(),
                source: "json_repair".to_string(),
                distribution: vec![("WORKFLOW".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            mode: ProbabilityDecision {
                choice: "DECIDE".to_string(),
                source: "json_repair".to_string(),
                distribution: vec![("DECIDE".to_string(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
        };

        let context = IntelContext::new(
            original_json.to_string(),
            route_decision,
            problems.join("\n"),
            String::new(),
            Vec::new(),
            client.clone(),
        );

        let output = self.execute_with_fallback(&context).await?;
        Ok(output
            .get_str("repaired_json")
            .unwrap_or(original_json)
            .to_string())
    }
}

impl IntelUnit for JsonRepairUnit {
    fn name(&self) -> &'static str {
        "json_repair"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty JSON input"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let problems_text = if context.workspace_facts.trim().is_empty() {
            "No problems found".to_string()
        } else {
            context.workspace_facts.trim().to_string()
        };

        let req = build_intel_system_user_request(
            &self.profile,
            format!(
                "Original JSON:\n{}\n\nProblems to fix:\n{}",
                context.user_message, problems_text
            ),
        );

        let chat_url = Url::parse(&self.profile.base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base_url '{}': {}", self.profile.base_url, e))?
            .join("/v1/chat/completions")
            .map_err(|e| anyhow::anyhow!("Failed to build chat URL: {}", e))?;
        let response =
            chat_once_with_timeout(&context.client, &chat_url, &req, self.profile.timeout_s)
                .await?;
        let repaired_json = extract_response_text(&response);

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "repaired_json": repaired_json,
            }),
            0.8,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output
            .get_str("repaired_json")
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(anyhow::anyhow!("Missing repaired_json output"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "repaired_json": context.user_message,
            }),
            &format!("json repair failed: {}", error),
        ))
    }
}

impl IntelUnit for SelectorUnit {
    fn name(&self) -> &'static str {
        "selector"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        // No specific pre-flight checks - selector is flexible
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let purpose = context
            .extra("purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let instructions = context
            .extra("instructions")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence = context
            .extra("evidence")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: SelectionOutput = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_selector_narrative(
                &context.user_message,
                &purpose,
                &instructions,
                &evidence,
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
        if output.get("items").is_none() {
            return Err(anyhow::anyhow!("Missing 'items' field"));
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
                "items": Vec::<String>::new(),
                "reason": "fallback: no items selected".to_string(),
            }),
            &format!("selector failed: {}", error),
        ))
    }
}

pub(crate) struct RenameSuggesterUnit {
    profile: Profile,
}

impl RenameSuggesterUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for RenameSuggesterUnit {
    fn name(&self) -> &'static str {
        "rename_suggester"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let purpose = context
            .extra("purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let instructions = context
            .extra("instructions")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence = context
            .extra("evidence")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: RenameSuggestion = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_rename_suggester_narrative(
                &context.user_message,
                &purpose,
                &instructions,
                &evidence,
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
        if output.get("identifier").is_none() {
            return Err(anyhow::anyhow!("Missing 'identifier' field"));
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
                "identifier": String::new(),
                "reason": "fallback: no rename suggested".to_string(),
            }),
            &format!("rename suggester failed: {}", error),
        ))
    }
}

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
        let result: EvidenceCompact = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_evidence_compactor_narrative(
                &objective, &purpose, &scope, &cmd, &output,
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
// Result Presenter Unit
// ============================================================================

/// Result Presenter Intel Unit
///
/// Presents final results to the user in appropriate format.
pub(crate) struct ResultPresenterUnit {
    profile: Profile,
}

impl ResultPresenterUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ResultPresenterUnit {
    fn name(&self) -> &'static str {
        "result_presenter"
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
        let runtime_context = context
            .extra("runtime_context")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let evidence_mode = context
            .extra("evidence_mode")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let response_advice = context
            .extra("response_advice")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let reply_instructions = context
            .extra("reply_instructions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("Present results clearly to the user"));
        let step_results = context
            .extra("step_results")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let result = execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_result_presenter_narrative(
                &context.user_message,
                &context.route_decision,
                &runtime_context,
                &evidence_mode,
                &response_advice,
                &reply_instructions,
                &step_results,
            ),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({ "final_text": result }),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("presentation").is_none() && output.get("final_text").is_none() {
            return Err(anyhow::anyhow!(
                "Missing 'presentation' or 'final_text' field"
            ));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "final_text": "Results presentation failed".to_string(),
                "reason": "fallback: presentation error".to_string(),
            }),
            &format!("result presenter failed: {}", error),
        ))
    }
}

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

// ============================================================================
// Expert Responder Unit
// ============================================================================

/// Expert Responder Intel Unit
///
/// Produces compact response-posture advice for the final presenter.
pub(crate) struct ExpertResponderUnit {
    profile: Profile,
}

impl ExpertResponderUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for ExpertResponderUnit {
    fn name(&self) -> &'static str {
        "expert_responder"
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
        let evidence_mode = context
            .extra("evidence_mode")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let reply_instructions = context
            .extra("reply_instructions")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("Respond clearly and use the evidence honestly."));
        let step_results = context
            .extra("step_results")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let result: ExpertResponderAdvice = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_expert_responder_narrative(
                &context.user_message,
                &context.route_decision,
                &evidence_mode,
                &reply_instructions,
                &step_results,
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
        if output.get("style").is_none() {
            return Err(anyhow::anyhow!("Missing 'style' field"));
        }
        if output.get("focus").is_none() {
            return Err(anyhow::anyhow!("Missing 'focus' field"));
        }
        if output.get("include_raw_output").is_none() {
            return Err(anyhow::anyhow!("Missing 'include_raw_output' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "style": "direct",
                "focus": "answer with the key result first",
                "include_raw_output": false,
                "reason": "fallback: keep the response simple and honest",
            }),
            &format!("expert responder failed: {}", error),
        ))
    }
}

// ============================================================================
// Status Message Unit
// ============================================================================

/// Status Message Intel Unit
///
/// Generates status messages for execution steps.
pub(crate) struct StatusMessageUnit {
    profile: Profile,
}

impl StatusMessageUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for StatusMessageUnit {
    fn name(&self) -> &'static str {
        "status_message"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        // No specific pre-flight checks
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let current_action = context
            .extra("current_action")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.user_message));
        let step_type = context
            .extra("step_type")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let step_purpose = context
            .extra("step_purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result: serde_json::Value = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_status_message_narrative(
                &current_action,
                &step_type,
                &step_purpose,
            ),
        )
        .await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("status").is_none() {
            return Err(anyhow::anyhow!("Missing 'status' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "status": "Processing...",
                "reason": "fallback: default status".to_string(),
            }),
            &format!("status message failed: {}", error),
        ))
    }
}

// ============================================================================
// Command Repair Unit
// ============================================================================

/// Command Repair Intel Unit
///
/// Repairs malformed or failed shell commands.
pub(crate) struct CommandRepairUnit {
    profile: Profile,
}

impl CommandRepairUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for CommandRepairUnit {
    fn name(&self) -> &'static str {
        "command_repair"
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
        let output = context
            .extra("output")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: CommandRepair = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_command_repair_narrative(
                &objective,
                &purpose,
                &context.user_message,
                &output,
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
        if output.get("repaired_command").is_none() && output.get("cmd").is_none() {
            return Err(anyhow::anyhow!("Missing 'repaired_command' or 'cmd' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "cmd": "".to_string(),
                "reason": "fallback: could not repair command".to_string(),
            }),
            &format!("command repair failed: {}", error),
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

/// Action Needs Classifier Intel Unit (atomic - 2 related outputs)
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
