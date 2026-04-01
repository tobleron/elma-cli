//! @efficiency-role: domain-logic
//!
//! Intel Units - Migrated Implementations
//!
//! This module contains intel units that have been migrated from plain functions
//! to the IntelUnit trait for better error handling, fallback support, and testability.
//!
//! Migrated Units:
//! - ComplexityAssessmentUnit
//! - EvidenceNeedsUnit
//! - ActionNeedsUnit
//! - WorkflowPlannerUnit

use crate::*;
use crate::intel_trait::*;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "route_prior": {
                            "route": context.route_decision.route,
                            "distribution": context.route_decision.distribution.iter()
                                .map(|(route, p)| serde_json::json!({"route": route, "p": p}))
                                .collect::<Vec<_>>(),
                        },
                        "workspace_facts": context.workspace_facts,
                        "workspace_brief": context.workspace_brief,
                        "conversation": conversation_excerpt(&context.conversation_excerpt, 12),
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),  // Note: client should be passed in context or stored
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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

// Note: Compatibility wrappers are NOT provided here to avoid name conflicts.
// The original functions in src/intel.rs continue to work.
// To use the trait-based units directly, instantiate the unit struct:
//   let unit = ComplexityAssessmentUnit::new(profile);
//   let output = unit.execute_with_fallback(&context).await?;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "route": context.route_decision.route,
                        "workspace_facts": context.workspace_facts,
                        "workspace_brief": context.workspace_brief,
                        "conversation": conversation_excerpt(&context.conversation_excerpt, 12),
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "route": context.route_decision.route,
                        "workspace_facts": context.workspace_facts,
                        "workspace_brief": context.workspace_brief,
                        "conversation": conversation_excerpt(&context.conversation_excerpt, 12),
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

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
        // Workflow planner uses multiple sub-calls, so we'll keep the existing logic
        // but wrap it in the trait pattern

        // For now, return a simplified output
        // Full migration would replicate the multi-call logic from plan_workflow_once()
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "speech_act": {"choice": context.route_decision.speech_act.choice},
                        "workflow": {"choice": context.route_decision.workflow.choice},
                        "mode": {"choice": context.route_decision.mode.choice},
                        "route": context.route_decision.route,
                        "workspace_facts": context.workspace_facts,
                        "workspace_brief": context.workspace_brief,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
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
            }),
            &format!("workflow planner failed: {}", error),
        ))
    }
}

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "route": context.route_decision.route,
                        "conversation": conversation_excerpt(&context.conversation_excerpt, 12),
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "route": context.route_decision.route,
                        "speech_act": context.route_decision.speech_act.choice,
                        "complexity": context.complexity,
                        "workspace_facts": context.workspace_facts,
                        "workspace_brief": context.workspace_brief,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "speech_act": context.route_decision.speech_act.choice,
                        "route": context.route_decision.route,
                        "complexity": context.complexity,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
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
            }),
            &format!("formula selector failed: {}", error),
        ))
    }
}

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

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
        // Selector needs objective, purpose, instructions, and evidence
        // These would typically be passed in a custom context or extracted from conversation
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "objective": context.user_message,
                        "evidence": context.workspace_facts,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "route": context.route_decision.route,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "evidence": context.workspace_facts,
                        "instructions": "Compact this evidence while preserving key information",
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("compacted_evidence").is_none() && output.get("summary").is_none() {
            return Err(anyhow::anyhow!("Missing 'compacted_evidence' or 'summary' field"));
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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "artifacts": context.workspace_facts,
                        "instructions": "Classify these artifacts by type and importance",
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("classifications").is_none() {
            return Err(anyhow::anyhow!("Missing 'classifications' field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "classifications": Vec::<serde_json::Value>::new(),
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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                        "step_results": "Results from execution",
                        "instructions": "Present results clearly to the user",
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("presentation").is_none() && output.get("final_text").is_none() {
            return Err(anyhow::anyhow!("Missing 'presentation' or 'final_text' field"));
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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "current_action": context.user_message,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "failed_command": context.user_message,
                        "error": context.workspace_facts,
                        "instructions": "Repair this command to fix the error",
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

        Ok(IntelOutput::success(self.name(), result, 0.9))
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
                "repaired_command": "".to_string(),
                "reason": "fallback: could not repair command".to_string(),
            }),
            &format!("command repair failed: {}", error),
        ))
    }
}

// Note: Compatibility wrappers are NOT provided to avoid name conflicts.
// Use the unit struct directly for trait-based execution.

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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
        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": context.user_message,
                    })
                    .to_string(),
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let result: serde_json::Value = chat_json_with_repair_timeout(
            &reqwest::Client::new(),
            &Url::parse(&self.profile.base_url).unwrap(),
            &req,
            self.profile.timeout_s,
        ).await?;

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
