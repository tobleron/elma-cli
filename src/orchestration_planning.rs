//! @efficiency-role: service-orchestrator
//!
//! Planning Prior and Hierarchical Decomposition Module
//!
//! Handles planning prior derivation and hierarchical task decomposition.

use crate::app::LoadedProfiles;
use crate::decomposition::{decompose_to_subgoals, generate_masterplan, needs_decomposition};
use crate::execution_ladder::{
    assess_execution_level, assessment_needs_decomposition, ExecutionLadderAssessment,
};
use crate::intel_units::{
    AssumptionTrackerUnit, DomainDifficultyUnit, EdgeCaseEvaluatorUnit, FreshnessRequirementUnit,
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
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = WorkflowPlannerUnit::new(cfg)
        .execute_with_fallback(&ctx)
        .await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse workflow planner output: {}", e))
}

async fn trait_assess_intent_surface(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = IntentSurfaceUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

async fn trait_assess_intent_real(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = IntentRealUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

async fn trait_assess_user_expectation(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = UserExpectationUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

async fn trait_assess_domain_difficulty(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = DomainDifficultyUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

async fn trait_assess_freshness_requirement(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = FreshnessRequirementUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

async fn trait_track_assumptions(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = AssumptionTrackerUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

async fn trait_evaluate_edge_cases(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<serde_json::Value> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages);
    let output = EdgeCaseEvaluatorUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
    Ok(output.data)
}

/// Run all advanced assessment units and collect their outputs.
/// Units run sequentially but with fallbacks so failures don't block the pipeline.
async fn run_advanced_assessments(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> (
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
) {
    let domain_difficulty =
        trait_assess_domain_difficulty(client, cfg, line, route_decision, ws, ws_brief, messages)
            .await
            .ok();
    let freshness = trait_assess_freshness_requirement(
        client,
        cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .ok();
    let assumptions =
        trait_track_assumptions(client, cfg, line, route_decision, ws, ws_brief, messages)
            .await
            .ok();
    let edge_cases =
        trait_evaluate_edge_cases(client, cfg, line, route_decision, ws, ws_brief, messages)
            .await
            .ok();

    (domain_difficulty, freshness, assumptions, edge_cases)
}

async fn trait_assess_complexity(
    client: &reqwest::Client,
    cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    intent_surface: &serde_json::Value,
    intent_real: &serde_json::Value,
    user_expectation: &serde_json::Value,
) -> Result<ComplexityAssessment> {
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages)
        .with_intent_surface(intent_surface.clone())
        .with_intent_real(intent_real.clone())
        .with_user_expectation(user_expectation.clone());
    let output = ComplexityAssessmentUnit::new(cfg.clone())
        .execute_with_fallback(&ctx)
        .await?;
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
    intent_surface: &serde_json::Value,
    intent_real: &serde_json::Value,
    user_expectation: &serde_json::Value,
) -> Result<ScopePlan> {
    let mut cfg = cfg.clone();
    cfg.timeout_s = cfg.timeout_s.min(45);
    let ctx = planning_intel_context(client, line, route_decision, ws, ws_brief, messages)
        .with_complexity(complexity.clone())
        .with_intent_surface(intent_surface.clone())
        .with_intent_real(intent_real.clone())
        .with_user_expectation(user_expectation.clone());
    let output = ScopeBuilderUnit::new(cfg)
        .execute_with_fallback(&ctx)
        .await?;
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
    intent_surface: &serde_json::Value,
    intent_real: &serde_json::Value,
    user_expectation: &serde_json::Value,
) -> Result<FormulaSelection> {
    let memory_candidates = memories.iter().map(|m| serde_json::json!({
        "id": m.id, "title": m.title, "route": m.route, "complexity": m.complexity, "formula": m.formula,
        "objective": m.objective, "example_user_message": m.user_message, "program_signature": m.program_signature,
        "success_count": m.success_count, "failure_count": m.failure_count, "last_success_unix_s": m.last_success_unix_s,
        "artifact_mode_capable": m.artifact_mode_capable, "active_run_id": m.active_run_id,
    })).collect::<Vec<_>>();
    let mut cfg = cfg.clone();
    cfg.timeout_s = cfg.timeout_s.min(45);
    let ctx = planning_intel_context(client, line, route_decision, "", "", messages)
        .with_complexity(complexity.clone())
        .with_intent_surface(intent_surface.clone())
        .with_intent_real(intent_real.clone())
        .with_user_expectation(user_expectation.clone())
        .with_extra("scope", scope)?
        .with_extra("memory_candidates", memory_candidates)?;
    let output = FormulaSelectorUnit::new(cfg)
        .execute_with_fallback(&ctx)
        .await?;
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

fn build_chat_fallback(
    line: &str,
    reason: &str,
) -> (ComplexityAssessment, ScopePlan, FormulaSelection) {
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
        reason: reason.to_string(),
        memory_id: String::new(),
    };
    (complexity, scope, formula)
}

fn build_ladder_chat(reason: &str) -> ExecutionLadderAssessment {
    ExecutionLadderAssessment::new(
        ExecutionLevel::Action,
        reason.to_string(),
        false,
        false,
        false,
        false,
        "LOW".to_string(),
        "DIRECT".to_string(),
    )
}

fn persist_masterplan(masterplan: &Masterplan, session_root: &PathBuf) {
    let masterplan_dir = session_root.join("masterplans");
    if std::fs::create_dir_all(&masterplan_dir).is_err() {
        return;
    }
    let masterplan_path = masterplan_dir.join(format!(
        "plan_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));
    if let Ok(json) = serde_json::to_string_pretty(masterplan) {
        let _ = std::fs::write(&masterplan_path, json);
    }
}

fn complexity_from_ladder(route: &str, ladder: &ExecutionLadderAssessment) -> ComplexityAssessment {
    ComplexityAssessment {
        complexity: ladder.complexity.clone(),
        needs_evidence: ladder.requires_evidence,
        needs_tools: !route.eq_ignore_ascii_case("CHAT"),
        needs_decision: ladder.level == ExecutionLevel::Plan
            || route.eq_ignore_ascii_case("DECIDE"),
        needs_plan: ladder.level.requires_planning_structure(),
        risk: ladder.risk.clone(),
        suggested_pattern: fallback_formula_for_route(route, ladder.requires_evidence),
    }
}

fn alignment_for_level(level: ExecutionLevel) -> Vec<&'static str> {
    match level {
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
    }
}

fn is_empty_workflow_plan(plan: &WorkflowPlannerOutput) -> bool {
    plan.objective.trim().is_empty()
        && plan.reason.trim().is_empty()
        && plan.scope.objective.trim().is_empty()
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
    wp: &WorkflowPlannerOutput,
) -> (ComplexityAssessment, ScopePlan) {
    let complexity = ComplexityAssessment {
        complexity: if wp.complexity.trim().is_empty() {
            if route_decision.route.eq_ignore_ascii_case("CHAT") {
                "DIRECT"
            } else {
                "INVESTIGATE"
            }
            .to_string()
        } else {
            wp.complexity.trim().to_string()
        },
        needs_evidence: wp.needs_evidence,
        needs_tools: !route_decision.route.eq_ignore_ascii_case("CHAT"),
        needs_decision: route_decision.route.eq_ignore_ascii_case("DECIDE"),
        needs_plan: route_decision.route.eq_ignore_ascii_case("PLAN")
            || route_decision.route.eq_ignore_ascii_case("MASTERPLAN"),
        risk: if wp.risk.trim().is_empty() {
            "LOW".to_string()
        } else {
            wp.risk.trim().to_string()
        },
        suggested_pattern: fallback_formula_for_route(&route_decision.route, wp.needs_evidence),
    };
    let mut scope = wp.scope.clone();
    if scope.objective.trim().is_empty() {
        scope.objective = if wp.objective.trim().is_empty() {
            line.to_string()
        } else {
            wp.objective.trim().to_string()
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
        let (complexity, scope, formula) = build_chat_fallback(line, "Direct conversational turn");
        return (None, complexity, scope, formula, false);
    }

    // Assess intent early for better planning
    let intent_surface = trait_assess_intent_surface(
        client,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or(serde_json::json!({}));
    let intent_real = trait_assess_intent_real(
        client,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or(serde_json::json!({}));
    let user_expectation = trait_assess_user_expectation(
        client,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await
    .unwrap_or(serde_json::json!({}));
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
            &intent_surface,
            &intent_real,
            &user_expectation,
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
        &intent_surface,
        &intent_real,
        &user_expectation,
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
        &intent_surface,
        &intent_real,
        &user_expectation,
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
        &intent_surface,
        &intent_real,
        &user_expectation,
    )
    .await
    .unwrap_or_default();
    (None, complexity, scope, formula, true)
}

// Task 023: Hierarchical Decomposition Trigger

fn get_required_depth(complexity: &str, risk: &str) -> u8 {
    let base_depth = match complexity {
        "DIRECT" => 1,
        "INVESTIGATE" => 2,
        "MULTISTEP" => 3,
        "OPEN_ENDED" => 4,
        _ => 2,
    };

    let risk_bonus = match risk {
        "HIGH" => 1,
        _ => 0,
    };

    base_depth + risk_bonus
}

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
    let required_depth = get_required_depth(&complexity.complexity, &complexity.risk);
    if required_depth < 4 {
        return Ok(None);
    }

    let masterplan = generate_masterplan(
        client,
        chat_url,
        &profiles.orchestrator_cfg,
        user_message,
        ws,
        ws_brief,
    )
    .await?;

    persist_masterplan(&masterplan, session_root);
    Ok(Some(masterplan))
}

// Task 044: Execution Ladder Integration

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
    let uncertain = should_use_uncertain_reply_default(line, route_decision);
    if (route_decision.entropy > 0.8 || route_decision.margin < 0.15) && uncertain
        || route_decision.route.eq_ignore_ascii_case("CHAT") && uncertain
    {
        let reason = if route_decision.route.eq_ignore_ascii_case("CHAT") {
            "Direct conversational turn"
        } else {
            "Classification uncertain, using safe default"
        };
        let ladder = build_ladder_chat(reason);
        let (complexity, scope, formula) = build_chat_fallback(line, reason);
        return (None, ladder, complexity, scope, formula, false);
    }

    let features = ClassificationFeatures::from(route_decision);
    let (ladder, workflow_plan) = match assess_execution_level(
        client,
        chat_url,
        complexity_cfg,
        evidence_need_cfg,
        action_need_cfg,
        workflow_planner_cfg,
        line,
        route_decision,
        &features,
        ws,
        ws_brief,
        messages,
    )
    .await
    {
        Ok(result) => result,
        Err(ref error) => {
            trace_verbose(true, &format!("ladder_assessment_failed error={}", error));
            (
                ExecutionLadderAssessment::fallback(&format!("assessment error: {}", error)),
                WorkflowPlannerOutput::default(),
            )
        }
    };

    let complexity = complexity_from_ladder(&route_decision.route, &ladder);

    // Run advanced assessments to enrich planning context
    let (domain_difficulty, freshness, assumptions, edge_cases) = run_advanced_assessments(
        client,
        complexity_cfg,
        line,
        route_decision,
        ws,
        ws_brief,
        messages,
    )
    .await;

    let scope = trait_build_scope(
        client,
        scope_builder_cfg,
        line,
        route_decision,
        &complexity,
        ws,
        ws_brief,
        messages,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
    )
    .await
    .unwrap_or_else(|_| ScopePlan {
        objective: line.to_string(),
        ..ScopePlan::default()
    });
    let mut formula = trait_select_formula(
        client,
        formula_cfg,
        line,
        route_decision,
        &complexity,
        &scope,
        memories,
        messages,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
        &serde_json::Value::Null,
    )
    .await
    .unwrap_or_else(|_| FormulaSelection {
        primary: complexity.suggested_pattern.clone(),
        alternatives: Vec::new(),
        reason: format!("ladder level: {}", ladder.level),
        memory_id: String::new(),
    });

    // Adjust formula based on advanced assessments
    if let Some(ref domain) = domain_difficulty {
        if let Some(domain_type) = domain.get("domain_type").and_then(|v| v.as_str()) {
            if domain_type == "expert" || domain_type == "niche" {
                formula.reason = format!(
                    "{}; domain={} (expert/niche requires careful evidence)",
                    formula.reason, domain_type
                );
            }
        }
        if let Some(sensitive) = domain.get("sensitive").and_then(|v| v.as_bool()) {
            if sensitive {
                formula.reason = format!(
                    "{}; sensitive domain (conservative approach)",
                    formula.reason
                );
            }
        }
    }

    if let Some(ref freshness) = freshness {
        if let Some(freshness_needed) = freshness.get("freshness_needed").and_then(|v| v.as_str()) {
            if freshness_needed == "high" {
                formula.reason = format!(
                    "{}; high freshness needed (may require live data)",
                    formula.reason
                );
            }
        }
    }

    if let Some(ref assumptions) = assumptions {
        if let Some(needs_verification) = assumptions
            .get("needs_verification")
            .and_then(|v| v.as_bool())
        {
            if needs_verification {
                formula.reason = format!("{}; assumptions need verification", formula.reason);
            }
        }
    }

    if let Some(ref edge_cases) = edge_cases {
        if let Some(high_risk) = edge_cases.get("high_risk_count").and_then(|v| v.as_u64()) {
            if high_risk > 0 {
                formula.reason = format!(
                    "{}; {} high-risk edge cases identified",
                    formula.reason, high_risk
                );
            }
        }
    }

    let allowed = alignment_for_level(ladder.level);
    if !allowed
        .iter()
        .any(|f| formula.primary.eq_ignore_ascii_case(f))
    {
        formula.reason = format!(
            "Aligned with ladder level {:?} (was: {})",
            ladder.level, formula.primary
        );
        formula.primary = allowed[0].to_string();
    }

    let workflow_plan = if is_empty_workflow_plan(&workflow_plan) {
        None
    } else {
        Some(workflow_plan)
    };
    let fallback_used = ladder.fallback_used;
    (
        workflow_plan,
        ladder,
        complexity,
        scope,
        formula,
        fallback_used,
    )
}

/// Check if hierarchical decomposition is needed using execution ladder.
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
    if !assessment_needs_decomposition(ladder_assessment) {
        return Ok(None);
    }
    if ladder_assessment.level != ExecutionLevel::MasterPlan && !ladder_assessment.requires_phases {
        return Ok(None);
    }
    let masterplan = generate_masterplan(
        client,
        chat_url,
        &profiles.orchestrator_cfg,
        user_message,
        ws,
        ws_brief,
    )
    .await?;
    persist_masterplan(&masterplan, session_root);
    Ok(Some(masterplan))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_route_decision(route: &str, margin: f64, entropy: f64) -> RouteDecision {
        RouteDecision {
            route: route.to_string(),
            source: "test".to_string(),
            distribution: vec![(route.to_string(), 1.0)],
            margin,
            entropy,
            speech_act: ProbabilityDecision {
                choice: "INQUIRE".into(),
                source: "test".into(),
                distribution: vec![("INQUIRE".into(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            workflow: ProbabilityDecision {
                choice: "WORKFLOW".into(),
                source: "test".into(),
                distribution: vec![("WORKFLOW".into(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            mode: ProbabilityDecision {
                choice: "INSPECT".into(),
                source: "test".into(),
                distribution: vec![("INSPECT".into(), 1.0)],
                margin: 1.0,
                entropy: 0.0,
            },
            evidence_required: false,
        }
    }

    #[test]
    fn uncertain_reply_default_allows_chat_like_turns() {
        assert!(should_use_uncertain_reply_default(
            "What model are you using and what is the base url?",
            &test_route_decision("DECIDE", 0.05, 0.2)
        ));
    }

    #[test]
    fn uncertain_reply_default_rejects_path_scoped_shell_requests() {
        assert!(!should_use_uncertain_reply_default(
            "Inspect only _stress_testing/_opencode_for_testing/. Map its directory structure and identify the top 3 largest source files by line count.",
            &test_route_decision("SHELL", 0.01, 0.03)));
    }

    #[test]
    fn uncertain_reply_default_rejects_path_scoped_plan_requests() {
        assert!(!should_use_uncertain_reply_default(
            "Standardize the logging style across _stress_testing/_claude_code_src/ only. Find a small, coherent subset of files that use inconsistent logging patterns, create one shared wrapper utility under _stress_testing/_claude_code_src/, and refactor only that verified subset to use the new utility.",
            &test_route_decision("PLAN", 0.10, 0.10)));
    }
}
