//! @efficiency-role: domain-logic
//!
//! Pyramid Intel Units — DecompositionUnit and NextActionSelectorUnit.
//!
//! DecompositionUnit: generates objective→goals→tasks pyramid.
//! NextActionSelectorUnit: picks the next task/action when repair stalls.

use crate::decomposition_pyramid::{DecompositionPyramid, NextAction, PyramidGoal, PyramidTask};
use crate::intel_trait::*;
use crate::intel_units::intel_units_dsl::{parse_next_action_dsl, parse_pyramid_block_dsl};
use crate::*;

// ============================================================================
// DecompositionUnit
// ============================================================================

/// Intel unit that decomposes a complex request into an objective→goals→tasks pyramid.
///
/// Runs after routing when confidence is low, evidence is needed, or the
/// execution level is Task or higher (not simple Action/CHAT).
pub(crate) struct DecompositionUnit {
    profile: Profile,
}

impl DecompositionUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for DecompositionUnit {
    fn name(&self) -> &'static str {
        "decomposition"
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
        let prior_failures = context
            .extra("prior_dsl_failures")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let narrative = crate::intel_narrative_pyramid::build_decomposition_narrative(
            &context.user_message,
            &context.route_decision,
            &context.workspace_facts,
            &context.workspace_brief,
            &context.conversation_excerpt,
            prior_failures,
        );

        let dsl_result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, narrative).await?;

        // Task 419: Parse simplified single-line OBJECTIVE DSL.
        // GOAL/TASK decomposition was removed because 3B models cannot
        // reliably produce multi-line block DSL with END terminators.
        let objective = dsl_result
            .get("text")
            .or_else(|| dsl_result.get("objective"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let risk = dsl_result
            .get("risk")
            .and_then(|v| v.as_str())
            .unwrap_or("low")
            .to_string();

        let pyramid = DecompositionPyramid {
            objective,
            risk,
            goals: vec![],
            tasks: vec![],
            next_action: None,
        };

        let pyramid_json = serde_json::json!({
            "objective": pyramid.objective,
            "risk": pyramid.risk,
            "goals": pyramid.goals.iter().map(|g| serde_json::json!({
                "text": g.text,
                "evidence_needed": g.evidence_needed,
            })).collect::<Vec<_>>(),
            "tasks": pyramid.tasks.iter().map(|t| serde_json::json!({
                "id": t.id,
                "text": t.text,
                "status": t.status,
            })).collect::<Vec<_>>(),
        });

        Ok(IntelOutput::success(self.name(), pyramid_json, 0.85))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("objective").is_none() && output.data.is_null() {
            return Err(anyhow::anyhow!("Missing objective in decomposition output"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "objective": "",
                "risk": "low",
                "goals": [],
                "tasks": [],
            }),
            &format!("decomposition failed: {}", error),
        ))
    }
}

// ============================================================================
// NextActionSelectorUnit
// ============================================================================

/// Intel unit that picks the next task/action when the tool loop repair stalls.
///
/// Receives the pyramid tasks + last error and produces a NEXT line.
pub(crate) struct NextActionSelectorUnit {
    profile: Profile,
}

impl NextActionSelectorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for NextActionSelectorUnit {
    fn name(&self) -> &'static str {
        "next_action_selector"
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
        let objective = context
            .extra("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tasks_json = context
            .extra("tasks")
            .cloned()
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        let last_error = context
            .extra("last_error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");

        let narrative = crate::intel_narrative_pyramid::build_next_action_narrative(
            &objective,
            &tasks_json,
            last_error,
        );

        let dsl_result =
            execute_intel_dsl_from_user_content(&context.client, &self.profile, narrative).await?;

        // Parse the NEXT DSL result
        let task_id = dsl_result
            .get("task_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let action = dsl_result
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("done")
            .to_string();
        let reason = dsl_result
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("next task from pyramid")
            .to_string();

        let next = NextAction {
            task_id,
            action,
            reason,
        };

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(&next).unwrap_or_default(),
            0.85,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("action").is_none() && output.data.is_null() {
            return Err(anyhow::anyhow!("Missing action in next-action output"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "task_id": 0,
                "action": "done",
                "reason": "fallback: could not select next action",
            }),
            &format!("next action selection failed: {}", error),
        ))
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decomposition_unit_creation() {
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
        let unit = DecompositionUnit::new(profile);
        assert_eq!(unit.name(), "decomposition");
    }

    #[test]
    fn test_next_action_selector_unit_creation() {
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
        let unit = NextActionSelectorUnit::new(profile);
        assert_eq!(unit.name(), "next_action_selector");
    }

    #[test]
    fn test_decomposition_pyramid_render_context() {
        let pyramid = DecompositionPyramid {
            objective: "Test the router".to_string(),
            risk: "low".to_string(),
            goals: vec![],
            tasks: vec![
                PyramidTask {
                    id: 1,
                    text: "Test routing_infer".to_string(),
                    status: "ready".to_string(),
                },
                PyramidTask {
                    id: 2,
                    text: "Test routing_calc".to_string(),
                    status: "pending".to_string(),
                },
            ],
            next_action: Some(NextAction {
                task_id: 1,
                action: "edit".to_string(),
                reason: "first task".to_string(),
            }),
        };
        let ctx = pyramid.render_context();
        assert!(ctx.contains("OBJECTIVE: Test the router"));
        assert!(ctx.contains("NEXT: task_id=1"));
        assert!(ctx.contains("PENDING TASKS:"));
        assert!(ctx.contains("task_id=1"));
        // Pending tasks are not in render_context (only ready/active)
        assert!(
            !ctx.contains("task_id=2"),
            "pending tasks should not appear in render_context"
        );
    }

    #[test]
    fn test_decomposition_pyramid_render_task_menu() {
        let pyramid = DecompositionPyramid {
            objective: "x".to_string(),
            risk: "low".to_string(),
            goals: vec![],
            tasks: vec![
                PyramidTask {
                    id: 1,
                    text: "First".to_string(),
                    status: "ready".to_string(),
                },
                PyramidTask {
                    id: 2,
                    text: "Second".to_string(),
                    status: "blocked".to_string(),
                },
            ],
            next_action: None,
        };
        let menu = pyramid.render_task_menu();
        assert!(menu.contains("AVAILABLE TASKS:"));
        assert!(menu.contains("→ task_id=1 status=ready"));
        assert!(menu.contains("· task_id=2 status=blocked"));
        assert!(menu.contains("NEXT task_id="));
    }

    #[test]
    fn test_decomposition_pyramid_default() {
        let p = DecompositionPyramid::default();
        assert_eq!(p.objective, "");
        assert_eq!(p.risk, "low");
        assert!(p.goals.is_empty());
        assert!(p.tasks.is_empty());
        assert!(p.next_action.is_none());
    }

    #[test]
    fn test_decomposition_pyramid_empty_render_context() {
        let p = DecompositionPyramid::default();
        let ctx = p.render_context();
        assert!(ctx.contains("OBJECTIVE:"));
        assert!(ctx.contains("RISK: low"));
        assert!(!ctx.contains("PENDING TASKS:"));
    }
}
