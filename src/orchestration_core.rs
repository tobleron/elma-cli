//! @efficiency-role: service-orchestrator
//!
//! Core Orchestration Module
//!
//! Tool-calling pipeline: Maestro sets context → model calls tools directly → final answer.

use crate::app::AppRuntime;
use crate::app_chat_fast_paths::build_direct_reply_program;
use crate::formulas::{select_optimal_formula, FormulaPattern, FormulaScores};
use crate::tool_loop::run_tool_loop;
use crate::tools::ToolRegistry;
use crate::*;

// ============================================================================
// Tool-Calling Orchestration (no Maestro — model plans itself)
// ============================================================================

/// Build a system prompt for tool calling without any intermediate planner.
/// The model has full context (workspace, conversation, tools) and plans directly.
///
/// The core prompt is defined in `prompt_core::TOOL_CALLING_SYSTEM_PROMPT`
/// and is protected from modification by CODEOWNERS, AGENTS.md Rule 8,
/// and build-time hash verification.
fn build_tool_calling_system_prompt(runtime: &AppRuntime, _line: &str) -> String {
    let turn_summaries: Vec<String> = runtime
        .messages
        .iter()
        .filter(|m| m.name.as_deref() == Some("turn_summary"))
        .map(|m| m.content.clone())
        .collect();
    let conversation = if !turn_summaries.is_empty() {
        format!("\n## Previous turns\n{}", turn_summaries.join("\n---\n"))
    } else if !runtime.messages.is_empty() {
        let last_msgs: Vec<String> = runtime
            .messages
            .iter()
            .rev()
            .take(6)
            .rev()
            .map(|m| {
                format!(
                    "{}: {}",
                    m.role,
                    m.content.chars().take(300).collect::<String>()
                )
            })
            .collect();
        format!("\n## Recent conversation\n{}", last_msgs.join("\n"))
    } else {
        String::new()
    };

    let skill_context = build_skill_context(runtime);

    crate::prompt_core::assemble_system_prompt(
        &conversation,
        &skill_context,
    )
}

fn build_skill_context(runtime: &AppRuntime) -> String {
    let primary = runtime.execution_plan.primary_skill();
    match primary {
        SkillId::RepoExplorer => {
            if let Ok(overview) = repo_explorer::explore_repo(&runtime.repo) {
                repo_explorer::render_repo_overview(&overview)
            } else {
                "(repo exploration unavailable)".to_string()
            }
        }
        SkillId::DocumentReader => {
            let caps = document_adapter::document_capabilities();
            let lines: Vec<String> = caps
                .iter()
                .map(|c| {
                    let note = c
                        .quality_note
                        .as_ref()
                        .map(|q| format!(" ({q})"))
                        .unwrap_or_default();
                    format!("- {} via {}{}", c.format, c.backend, note)
                })
                .collect();
            format!("Document capabilities:\n{}", lines.join("\n"))
        }
        SkillId::FileScout => {
            let exclusions: Vec<String> =
                file_scout::default_scout_exclusions().into_iter().collect();
            format!(
                "File scout exclusions: {}\nUse on-demand discovery. Stay read-only outside workspace. Disclose searched roots.",
                exclusions.join(", ")
            )
        }
        SkillId::TaskSteward => {
            let inventory = task_steward::scan_task_inventory(&runtime.repo);
            task_steward::render_inventory_summary(&inventory)
        }
        SkillId::General => "(general mode — no specialized context)".to_string(),
    }
}

/// Run the tool-calling pipeline: model plans and executes tools directly.
/// Returns (final_answer, iterations_used, tool_calls_made, stopped_by_max).
pub(crate) async fn run_tool_calling_pipeline(
    runtime: &mut AppRuntime,
    line: &str,
    tui: &mut crate::ui_terminal::TerminalUI,
    context_hint: &str,
    evidence_required: bool,
    complexity: &str,
) -> Result<(String, usize, usize, bool)> {
    let system_prompt = build_tool_calling_system_prompt(runtime, line);
    trace(
        &runtime.args,
        "tool_calling: direct model planning (no Maestro)",
    );

    // Task 590: Inject cross-cycle evidence summary if available
    let user_line: String = if let Some(ref prior_evidence) = runtime.last_evidence_summary {
        trace(&runtime.args, "tool_loop: injected cross-cycle evidence summary");
        format!(
            "{}\n\n[Previously gathered in a prior attempt]\n{}\nDo NOT repeat steps already completed. Continue from where you left off.",
            line, prior_evidence
        )
    } else {
        line.to_string()
    };

    tui.start_status("Executing...");

    let result = run_tool_loop(
        &runtime.args,
        &runtime.client,
        &runtime.chat_url,
        &runtime.model_id,
        &system_prompt,
        &user_line,
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        &runtime.session,
        0.2, // temperature — low for reliability
        16384,
        tui,
        Some(&runtime.profiles.summarizer_cfg),
        context_hint,
        evidence_required,
        runtime.ctx_max,
        &runtime.goal_state,
        complexity,
    )
    .await?;

    tui.complete_status("Done");

    runtime.last_stop_outcome = result.stop_outcome.clone();
    runtime.last_evidence_summary = result.evidence_progress_summary.clone();

    // Task 422: Clear evidence ledger at end of turn
    crate::evidence_ledger::clear_session_ledger();

    // Strip leaked thinking/tool_call blocks before returning to the user
    let clean_answer = crate::text_utils::strip_thinking_blocks(&result.final_answer);

    Ok((
        clean_answer,
        result.iterations,
        result.tool_calls_made,
        result.stopped_by_max,
    ))
}

/// Compute risk deterministically from the tool-calling result metadata.
pub(crate) fn compute_program_risk(_tool_calls_made: usize, _iterations: usize) -> ProgramRisk {
    ProgramRisk::Low
}

pub(crate) async fn orchestrate_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<(Program, String)> {
    // Get cached tool registry (avoids repeated instantiation)
    let tool_registry = crate::tool_registry::get_registry();

    // Select optimal formula based on complexity and efficiency
    let formula_selection = select_optimal_formula(
        &complexity.complexity,
        &complexity.risk,
        &route_decision.route,
        0.5, // Balanced efficiency priority (can be tuned)
    );

    let prompt = build_orchestrator_user_content(
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &formula_selection,
    );

    // Use GBNF grammar for SHELL routes to ensure valid JSON
    let use_grammar = route_decision.route.eq_ignore_ascii_case("SHELL")
        || route_decision.route.eq_ignore_ascii_case("WORKFLOW");

    orchestration_helpers::request_program_or_repair(
        client,
        chat_url,
        orchestrator_cfg,
        &prompt,
        use_grammar,
    )
    .await
}

pub(crate) async fn recover_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    failure_reason: &str,
    current_program: Option<&Program>,
    step_results: &[StepResult],
) -> Result<Program> {
    let prompt = build_recovery_user_content(
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        failure_reason,
        current_program,
        step_results,
    );
    orchestration_helpers::request_recovery_program(
        client,
        chat_url,
        orchestrator_cfg,
        &prompt,
        step_results,
    )
    .await
}

pub(crate) async fn run_critic_once(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> Result<CriticVerdict> {
    orchestration_helpers::request_critic_verdict(
        client,
        chat_url,
        critic_cfg,
        line,
        route_decision,
        program,
        step_results,
        sufficiency,
        attempt,
    )
    .await
}

pub(crate) async fn generate_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    evidence_mode_cfg: &Profile,
    expert_advisor_cfg: &Profile,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    formatter_cfg: &Profile,
    system_content: &str,
    model_id: &str,
    base_url: &str,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
    workspace_facts: &str,
    workspace_brief: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<(String, Option<u64>)> {
    let runtime_context = serde_json::json!({
        "model_id": model_id,
        "base_url": base_url,
    });
    if route_decision.route.eq_ignore_ascii_case("CHAT") && step_results.is_empty() {
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
        let evidence_mode = EvidenceModeDecision {
            mode: "COMPACT".to_string(),
            reason: "chat reply fast path".to_string(),
        };
        let response_advice = orchestration_helpers::request_response_advice_via_unit(
            client,
            expert_advisor_cfg,
            line,
            route_decision,
            &evidence_mode,
            reply_instructions,
            step_results,
            workspace_facts,
            workspace_brief,
        )
        .await
        .unwrap_or_default();
        if let Some(t) = tui.as_deref_mut() {
            let _ = t.pump_ui();
        }
        let final_text = orchestration_helpers::present_result_via_unit(
            client,
            presenter_cfg,
            line,
            route_decision,
            &runtime_context,
            &evidence_mode,
            &response_advice,
            step_results,
            reply_instructions,
            workspace_facts,
            workspace_brief,
        )
        .await
        .unwrap_or_else(|_| {
            if reply_instructions.trim().is_empty() {
                line.to_string()
            } else {
                reply_instructions.to_string()
            }
        });

        let (formatted, usage) = orchestration_helpers::maybe_format_final_text(
            client,
            chat_url,
            formatter_cfg,
            line,
            final_text,
            None,
        )
        .await;
        return Ok((formatted, usage));
    }

    let evidence_mode = orchestration_helpers::decide_evidence_mode_via_unit(
        client,
        evidence_mode_cfg,
        line,
        route_decision,
        reply_instructions,
        step_results,
        workspace_facts,
        workspace_brief,
    )
    .await
    .unwrap_or_else(|_| EvidenceModeDecision {
        mode: "COMPACT".to_string(),
        reason: "fallback".to_string(),
    });
    if let Some(t) = tui.as_deref_mut() {
        let _ = t.pump_ui();
    }
    let response_advice = orchestration_helpers::request_response_advice_via_unit(
        client,
        expert_advisor_cfg,
        line,
        route_decision,
        &evidence_mode,
        reply_instructions,
        step_results,
        workspace_facts,
        workspace_brief,
    )
    .await
    .unwrap_or_default();
    if let Some(t) = tui.as_deref_mut() {
        let _ = t.pump_ui();
    }

    let (mut final_text, mut usage_total) = if route_decision.route.eq_ignore_ascii_case("CHAT") {
        orchestration_helpers::request_chat_final_text(
            client,
            chat_url,
            elma_cfg,
            system_content,
            line,
            step_results,
            reply_instructions,
            tui,
        )
        .await?
    } else {
        let text = orchestration_helpers::present_result_via_unit(
            client,
            presenter_cfg,
            line,
            route_decision,
            &runtime_context,
            &evidence_mode,
            &response_advice,
            step_results,
            reply_instructions,
            workspace_facts,
            workspace_brief,
        )
        .await
        .unwrap_or_default();
        (text, None)
    };

    if !route_decision.route.eq_ignore_ascii_case("CHAT") && !final_text.trim().is_empty() {
        final_text = orchestration_helpers::maybe_revise_presented_result(
            client,
            chat_url,
            presenter_cfg,
            claim_checker_cfg,
            line,
            route_decision,
            &runtime_context,
            &evidence_mode,
            &response_advice,
            step_results,
            reply_instructions,
            final_text,
            workspace_facts,
            workspace_brief,
        )
        .await;
        final_text = orchestration_helpers::preserve_exact_grounded_path(
            final_text,
            step_results,
            reply_instructions,
        );
    }

    let (formatted_text, formatted_usage) = orchestration_helpers::maybe_format_final_text(
        client,
        chat_url,
        formatter_cfg,
        line,
        final_text,
        usage_total,
    )
    .await;
    usage_total = formatted_usage;
    Ok((formatted_text, usage_total))
}

pub(crate) async fn judge_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    judge_cfg: &Profile,
    scenario: &CalibrationScenario,
    user_message: &str,
    step_results: &[StepResult],
    final_text: &str,
) -> Result<CalibrationJudgeVerdict> {
    orchestration_helpers::request_judge_verdict(
        client,
        chat_url,
        judge_cfg,
        scenario,
        user_message,
        step_results,
        final_text,
    )
    .await
}
