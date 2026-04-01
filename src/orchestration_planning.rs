//! @efficiency-role: service-orchestrator
//!
//! Planning Prior and Hierarchical Decomposition Module
//!
//! Handles planning prior derivation and hierarchical task decomposition.

use crate::*;
use crate::app::LoadedProfiles;
use crate::decomposition::{generate_masterplan, decompose_to_subgoals, get_required_depth, needs_decomposition};

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
) -> (Option<WorkflowPlannerOutput>, ComplexityAssessment, ScopePlan, FormulaSelection, bool) {
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

    if let Ok(workflow_plan) = plan_workflow_once(
        client,
        chat_url,
        workflow_planner_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        memories,
        messages,
    )
    .await
    {
        let (complexity, scope) =
            planning_prior_from_workflow_plan(line, route_decision, &workflow_plan);
        let formula = select_formula_once(
            client,
            chat_url,
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

    let complexity = assess_complexity_once(
        client,
        chat_url,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or_default();
    let scope = build_scope_once(
        client,
        chat_url,
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
    let formula = select_formula_once(
        client,
        chat_url,
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
    ).await?;

    // Persist masterplan to session
    let masterplan_dir = session_root.join("masterplans");
    if let Err(e) = std::fs::create_dir_all(&masterplan_dir) {
        // Silently ignore errors - masterplan is optional
    } else {
        let masterplan_path = masterplan_dir.join(format!("plan_{}.json", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()));
        if let Ok(json) = serde_json::to_string_pretty(&masterplan) {
            let _ = std::fs::write(&masterplan_path, json);
        }
    }

    Ok(Some(masterplan))
}
