//! @efficiency-role: service-orchestrator
//!
//! Planning Prior and Hierarchical Decomposition Module
//!
//! Handles planning prior derivation and hierarchical task decomposition.
//!
//! Task 044: Integrated execution ladder for minimum-sufficient orchestration.

use crate::app::LoadedProfiles;
use crate::decomposition::{decompose_to_subgoals, generate_masterplan, needs_decomposition};
use crate::execution_ladder::{
    assess_execution_level, assessment_needs_decomposition, ExecutionLadderAssessment,
};
use crate::*;

fn planning_intel_context(
    client: &reqwest::Client,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> IntelContext {
    IntelContext::new(
        line.to_string(),
        route_decision.clone(),
        ws.to_string(),
        ws_brief.to_string(),
        messages.to_vec(),
        client.clone(),
    )
}

async fn trait_plan_workflow(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<WorkflowPlannerOutput> {
    let mut cfg = cfg.clone();
    cfg.timeout_s = cfg.timeout_s.min(45);
    let unit = WorkflowPlannerUnit::new(cfg);
    let context = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse workflow planner output: {}", e))
}

async fn trait_assess_complexity(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<ComplexityAssessment> {
    let unit = ComplexityAssessmentUnit::new(cfg.clone());
    let context = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = unit.execute_with_fallback(&context).await?;
    Ok(ComplexityOutput::from_intel_output(&output)?.assessment)
}

async fn trait_build_scope(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<ScopePlan> {
    let mut cfg = cfg.clone();
    cfg.timeout_s = cfg.timeout_s.min(45);
    let unit = ScopeBuilderUnit::new(cfg);
    let context = planning_intel_context(client, line, route_decision, ws, ws_brief, messages)
        .with_complexity(complexity.clone());
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse scope plan: {}", e))
}

async fn trait_select_formula(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> Result<FormulaSelection> {
    let memory_candidates = memories
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id, "title": m.title, "route": m.route, "complexity": m.complexity,
                "formula": m.formula, "objective": m.objective, "example_user_message": m.user_message,
                "program_signature": m.program_signature, "success_count": m.success_count,
                "failure_count": m.failure_count, "last_success_unix_s": m.last_success_unix_s,
                "artifact_mode_capable": m.artifact_mode_capable, "active_run_id": m.active_run_id,
            })
        })
        .collect::<Vec<_>>();
    let mut cfg = cfg.clone();
    cfg.timeout_s = cfg.timeout_s.min(45);
    let unit = FormulaSelectorUnit::new(cfg);
    let context = planning_intel_context(client, line, route_decision, "", "", messages)
        .with_complexity(complexity.clone())
        .with_extra("scope", scope)?
        .with_extra("memory_candidates", memory_candidates)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse formula selection: {}", e))
}

fn fallback_formula_for_route(route: &str, needs_evidence: bool) -> String {
    if route.eq_ignore_ascii_case("CHAT") {
        "reply_only".to_string()
    } else if route.eq_ignore_ascii_case("PLAN") {
        "plan_reply".to_string()
    } else if route.eq_ignore_ascii_case("MASTERPLAN") {
        "masterplan_reply".to_string()
    } else if route.eq_ignore_ascii_case("DECIDE") {
        if needs_evidence {
            "inspect_decide_reply".to_string()
        } else {
            "reply_only".to_string()
        }
    } else if needs_evidence {
        "inspect_reply".to_string()
    } else {
        "execute_reply".to_string()
    }
}

fn should_use_uncertain_reply_default(line: &str, route_decision: &RouteDecision) -> bool {
    if route_decision.route.eq_ignore_ascii_case("CHAT") {
        return true;
    }

    // Do not under-execute path-scoped workspace requests into reply-only.
    // If the user anchored the request to explicit repo targets, we should
    // prefer grounded evidence gathering over a free answer.
    extract_first_path_from_user_text(line).is_none()
}

fn planning_prior_from_workflow_plan(
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: &WorkflowPlannerOutput,
) -> (ComplexityAssessment, ScopePlan) {
    let complexity = ComplexityAssessment {
        complexity: if workflow_plan.complexity.trim().is_empty() {
            if route_decision.route.eq_ignore_ascii_case("CHAT") {
                "DIRECT".to_string()
            } else {
                "INVESTIGATE".to_string()
            }
        } else {
            workflow_plan.complexity.trim().to_string()
        },
        needs_evidence: workflow_plan.needs_evidence,
        needs_tools: !route_decision.route.eq_ignore_ascii_case("CHAT"),
        needs_decision: route_decision.route.eq_ignore_ascii_case("DECIDE"),
        needs_plan: route_decision.route.eq_ignore_ascii_case("PLAN")
            || route_decision.route.eq_ignore_ascii_case("MASTERPLAN"),
        risk: if workflow_plan.risk.trim().is_empty() {
            "LOW".to_string()
        } else {
            workflow_plan.risk.trim().to_string()
        },
        suggested_pattern: fallback_formula_for_route(
            &route_decision.route,
            workflow_plan.needs_evidence,
        ),
    };

    let mut scope = workflow_plan.scope.clone();
    if scope.objective.trim().is_empty() {
        scope.objective = if workflow_plan.objective.trim().is_empty() {
            line.to_string()
        } else {
            workflow_plan.objective.trim().to_string()
        };
    }

    (complexity, scope)
}

pub(crate) async fn derive_planning_prior(
    client: &reqwest::Client,
    chat_url: &Url,
    workflow_planner_cfg: &Profile,
    complexity_cfg: &Profile,
    scope_builder_cfg: &Profile,
    formula_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> (
    Option<WorkflowPlannerOutput>,
    ComplexityAssessment,
    ScopePlan,
    FormulaSelection,
    bool,
) {
    if route_decision.route.eq_ignore_ascii_case("CHAT") {
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let scope = ScopePlan {
            objective: line.to_string(),
            ..ScopePlan::default()
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec!["capability_reply".to_string()],
            reason: "Direct conversational turn".to_string(),
            memory_id: String::new(),
        };
        return (None, complexity, scope, formula, false);
    }

    if let Ok(workflow_plan) = trait_plan_workflow(
        client,
        workflow_planner_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    {
        let (complexity, scope) =
            planning_prior_from_workflow_plan(line, route_decision, &workflow_plan);
        let formula = trait_select_formula(
            client,
            formula_cfg,
            line,
            route_decision,
            &complexity,
            &scope,
            memories,
            messages,
        )
        .await
        .unwrap_or_else(|_| FormulaSelection {
            primary: complexity.suggested_pattern.clone(),
            alternatives: Vec::new(),
            reason: workflow_plan.reason.clone(),
            memory_id: String::new(),
        });
        return (Some(workflow_plan), complexity, scope, formula, false);
    }

    let complexity = trait_assess_complexity(
        client,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or_default();
    let scope = trait_build_scope(
        client,
        scope_builder_cfg,
        line,
        route_decision,
        &complexity,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or_default();
    let formula = trait_select_formula(
        client,
        formula_cfg,
        line,
        route_decision,
        &complexity,
        &scope,
        memories,
        messages,
    )
    .await
    .unwrap_or_default();
    (None, complexity, scope, formula, true)
}

// ============================================================================
// Task 023: Hierarchical Decomposition Trigger
// ============================================================================

/// Check if hierarchical decomposition is needed and trigger it
///
/// Returns:
/// - Some(Masterplan) if decomposition was triggered (OPEN_ENDED or HIGH risk)
/// - None if direct execution should proceed
pub async fn try_hierarchical_decomposition(
    client: &reqwest::Client,
    chat_url: &Url,
    profiles: &LoadedProfiles,
    session_root: &PathBuf,
    user_message: &str,
    complexity: &ComplexityAssessment,
    ws: &str,
    ws_brief: &str,
    _messages: &[ChatMessage],
) -> Result<Option<Masterplan>> {
    // Check if decomposition is needed
    let required_depth = get_required_depth(&complexity.complexity, &complexity.risk);

    if required_depth < 4 {
        // No decomposition needed for simple tasks - use direct execution
        return Ok(None);
    }

    // Decomposition required - generate masterplan (Goal level)
    let masterplan = generate_masterplan(
        client,
        chat_url,
        &profiles.orchestrator_cfg,
        user_message,
        ws,
        ws_brief,
    )
    .await?;

    // Persist masterplan to session
    let masterplan_dir = session_root.join("masterplans");
    if let Err(e) = std::fs::create_dir_all(&masterplan_dir) {
        // Silently ignore errors - masterplan is optional
    } else {
        let masterplan_path = masterplan_dir.join(format!(
            "plan_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        if let Ok(json) = serde_json::to_string_pretty(&masterplan) {
            let _ = std::fs::write(&masterplan_path, json);
        }
    }

    Ok(Some(masterplan))
}

// ============================================================================
// Task 044: Execution Ladder Integration
// ============================================================================

/// Derive planning prior using execution ladder assessment
///
/// This is the Task 044 replacement for derive_planning_prior().
/// Uses the execution ladder to determine minimum sufficient operational level.
///
/// Returns:
/// - Execution ladder assessment
/// - Complexity assessment (for backward compatibility)
/// - Scope plan
/// - Formula selection
/// - Fallback flag (true if assessment used fallback)
pub async fn derive_planning_prior_with_ladder(
    client: &reqwest::Client,
    chat_url: &Url,
    workflow_planner_cfg: &Profile,
    complexity_cfg: &Profile,
    evidence_need_cfg: &Profile,
    action_need_cfg: &Profile,
    scope_builder_cfg: &Profile,
    formula_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> (
    Option<WorkflowPlannerOutput>,
    ExecutionLadderAssessment,
    ComplexityAssessment,
    ScopePlan,
    FormulaSelection,
    bool,
) {
    // Task 014: Check classification confidence
    // If model is uncertain (high entropy or low margin), default to safe CHAT route
    // This prevents over-orchestration on ambiguous inputs WITHOUT hardcoded rules
    // Principle: When uncertain, safer to under-execute than over-execute
    if (route_decision.entropy > 0.8 || route_decision.margin < 0.15)
        && should_use_uncertain_reply_default(line, route_decision)
    {
        // Override to CHAT for uncertain classifications
        // Safer to under-execute than over-execute
        let ladder = ExecutionLadderAssessment::new(
            ExecutionLevel::Action,
            "Classification uncertain, using safe default".to_string(),
            false,
            false,
            false,
            false,
            "LOW".to_string(),
            "DIRECT".to_string(),
        );
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let scope = ScopePlan {
            objective: line.to_string(),
            ..ScopePlan::default()
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec!["capability_reply".to_string()],
            reason: "Classification uncertain, using safe default".to_string(),
            memory_id: String::new(),
        };
        return (None, ladder, complexity, scope, formula, false);
    }

    // Handle CHAT route specially (no ladder needed)
    if route_decision.route.eq_ignore_ascii_case("CHAT") {
        let ladder = ExecutionLadderAssessment::new(
            ExecutionLevel::Action,
            "Direct conversational turn".to_string(),
            false,
            false,
            false,
            false,
            "LOW".to_string(),
            "DIRECT".to_string(),
        );
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let scope = ScopePlan {
            objective: line.to_string(),
            ..ScopePlan::default()
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec!["capability_reply".to_string()],
            reason: "Direct conversational turn".to_string(),
            memory_id: String::new(),
        };
        return (None, ladder, complexity, scope, formula, false);
    }

    // Create classification features from route decision (Task 007: soft guidance)
    let features = ClassificationFeatures::from(route_decision);

    // Run execution ladder assessment
    let ladder_result = assess_execution_level(
        client,
        chat_url,
        complexity_cfg,
        evidence_need_cfg,
        action_need_cfg,
        workflow_planner_cfg,
        line,
        route_decision,
        &features, // Task 007: Pass full feature vector for better escalation
        ws,
        ws_brief,
        messages,
    )
    .await;

    let (ladder, workflow_plan) = match ladder_result {
        Ok(result) => result,
        Err(error) => {
            trace_verbose(true, &format!("ladder_assessment_failed error={}", error));
            (
                ExecutionLadderAssessment::fallback(&format!("assessment error: {}", error)),
                WorkflowPlannerOutput::default(),
            )
        }
    };

    // Convert ladder assessment to complexity assessment (backward compatibility)
    let complexity = ComplexityAssessment {
        complexity: ladder.complexity.clone(),
        needs_evidence: ladder.requires_evidence,
        needs_tools: !route_decision.route.eq_ignore_ascii_case("CHAT"),
        needs_decision: ladder.level == ExecutionLevel::Plan
            || route_decision.route.eq_ignore_ascii_case("DECIDE"),
        needs_plan: ladder.level.requires_planning_structure(),
        risk: ladder.risk.clone(),
        suggested_pattern: fallback_formula_for_route(
            &route_decision.route,
            ladder.requires_evidence,
        ),
    };

    // Build scope (use ladder's objective if available, otherwise build from scratch)
    let scope = trait_build_scope(
        client,
        scope_builder_cfg,
        line,
        route_decision,
        &complexity,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or_else(|_| ScopePlan {
        objective: line.to_string(),
        ..ScopePlan::default()
    });

    // Select formula based on ladder assessment
    let mut formula = trait_select_formula(
        client,
        formula_cfg,
        line,
        route_decision,
        &complexity,
        &scope,
        memories,
        messages,
    )
    .await
    .unwrap_or_else(|_| FormulaSelection {
        primary: complexity.suggested_pattern.clone(),
        alternatives: Vec::new(),
        reason: format!("ladder level: {}", ladder.level),
        memory_id: String::new(),
    });

    // Task 014: Enforce formula-level alignment (ladder determines allowed formulas)
    // This ensures formula complexity matches the minimum sufficient level
    let allowed_formulas = match ladder.level {
        ExecutionLevel::Action => vec!["reply_only", "execute_reply"],
        ExecutionLevel::Task => vec![
            "execute_reply",
            "inspect_reply",
            "inspect_summarize_reply",
            "inspect_decide_reply",
            "inspect_edit_verify_reply",
        ],
        ExecutionLevel::Plan => vec!["plan_reply"],
        ExecutionLevel::MasterPlan => vec!["masterplan_reply"],
    };

    if !allowed_formulas
        .iter()
        .any(|f| formula.primary.eq_ignore_ascii_case(f))
    {
        // Formula doesn't match ladder level - override to appropriate formula
        formula.reason = format!(
            "Aligned with ladder level {:?} (was: {})",
            ladder.level, formula.primary
        );
        formula.primary = allowed_formulas[0].to_string();
    }

    let fallback_used = ladder.fallback_used;

    let workflow_plan = if workflow_plan.objective.trim().is_empty()
        && workflow_plan.reason.trim().is_empty()
        && workflow_plan.scope.objective.trim().is_empty()
    {
        None
    } else {
        Some(workflow_plan)
    };

    (
        workflow_plan,
        ladder,
        complexity,
        scope,
        formula,
        fallback_used,
    )
}

/// Check if hierarchical decomposition is needed using execution ladder
///
/// Task 044 update: Uses ladder assessment instead of old depth gating.
pub async fn try_hierarchical_decomposition_with_ladder(
    client: &reqwest::Client,
    chat_url: &Url,
    profiles: &LoadedProfiles,
    session_root: &PathBuf,
    user_message: &str,
    ladder_assessment: &ExecutionLadderAssessment,
    ws: &str,
    ws_brief: &str,
    _messages: &[ChatMessage],
) -> Result<Option<Masterplan>> {
    // Check if decomposition is needed using ladder assessment
    if !assessment_needs_decomposition(ladder_assessment) {
        // No decomposition needed for Action/Task levels
        return Ok(None);
    }

    // Decomposition required for Plan/MasterPlan levels
    // For MasterPlan, generate full strategic decomposition
    if ladder_assessment.level == ExecutionLevel::MasterPlan || ladder_assessment.requires_phases {
        let masterplan = generate_masterplan(
            client,
            chat_url,
            &profiles.orchestrator_cfg,
            user_message,
            ws,
            ws_brief,
        )
        .await?;

        // Persist masterplan to session
        let masterplan_dir = session_root.join("masterplans");
        if let Err(e) = std::fs::create_dir_all(&masterplan_dir) {
            // Silently ignore errors - masterplan is optional
        } else {
            let masterplan_path = masterplan_dir.join(format!(
                "plan_{}.json",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
            if let Ok(json) = serde_json::to_string_pretty(&masterplan) {
                let _ = std::fs::write(&masterplan_path, json);
            }
        }

        Ok(Some(masterplan))
    } else {
        // Plan level but not MasterPlan - may not need full hierarchy
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_probability_decision(choice: &str) -> ProbabilityDecision {
        ProbabilityDecision {
            choice: choice.to_string(),
            source: "test".to_string(),
            distribution: vec![(choice.to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
        }
    }

    fn test_route_decision(route: &str, margin: f64, entropy: f64) -> RouteDecision {
        RouteDecision {
            route: route.to_string(),
            source: "test".to_string(),
            distribution: vec![(route.to_string(), 1.0)],
            margin,
            entropy,
            speech_act: test_probability_decision("INQUIRE"),
            workflow: test_probability_decision("WORKFLOW"),
            mode: test_probability_decision("INSPECT"),
        }
    }

    #[test]
    fn uncertain_reply_default_allows_chat_like_turns() {
        let route = test_route_decision("DECIDE", 0.05, 0.2);
        assert!(should_use_uncertain_reply_default(
            "What model are you using and what is the base url?",
            &route
        ));
    }

    #[test]
    fn uncertain_reply_default_rejects_path_scoped_shell_requests() {
        let route = test_route_decision("SHELL", 0.01, 0.03);
        assert!(!should_use_uncertain_reply_default(
            "Inspect only _stress_testing/_opencode_for_testing/. Map its directory structure and identify the top 3 largest source files by line count.",
            &route
        ));
    }

    #[test]
    fn uncertain_reply_default_rejects_path_scoped_plan_requests() {
        let route = test_route_decision("PLAN", 0.10, 0.10);
        assert!(!should_use_uncertain_reply_default(
            "Standardize the logging style across _stress_testing/_claude_code_src/ only. Find a small, coherent subset of files that use inconsistent logging patterns, create one shared wrapper utility under _stress_testing/_claude_code_src/, and refactor only that verified subset to use the new utility.",
            &route
        ));
    }
}
