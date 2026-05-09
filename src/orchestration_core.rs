//! @efficiency-role: service-orchestrator
//!
//! Core Orchestration Module
//!
//! Tool-calling pipeline: Maestro sets context → model calls tools directly → final answer.

use crate::app::AppRuntime;

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
        .state
        .messages
        .iter()
        .filter(|m| m.name.as_deref() == Some("turn_summary"))
        .map(|m| m.content.clone())
        .collect();
    let conversation = if !turn_summaries.is_empty() {
        format!("\n## Previous turns\n{}", turn_summaries.join("\n---\n"))
    } else if !runtime.state.messages.is_empty() {
        let last_msgs: Vec<String> = runtime
            .state
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

    crate::prompt_core::assemble_system_prompt(&conversation, &skill_context)
}

fn build_skill_context(runtime: &AppRuntime) -> String {
    let primary = runtime.state.execution_plan.primary_skill();
    match primary {
        SkillId::RepoExplorer => {
            if let Ok(overview) = repo_explorer::explore_repo(&runtime.workspace.repo) {
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
            let inventory = task_steward::scan_task_inventory(&runtime.workspace.repo);
            task_steward::render_inventory_summary(&inventory)
        }
        SkillId::General => "(general mode — no specialized context)".to_string(),
    }
}

/// Task 767: Structured result from the tool-calling pipeline.
/// Replaces the flat (String, usize, usize, bool) tuple so that
/// direct tool-loop metadata is available to summarizers, continuity,
/// and trace reducers.
#[derive(Debug, Clone)]
pub(crate) struct ToolCallingPipelineResult {
    pub final_answer: String,
    pub iterations: usize,
    pub tool_calls_made: usize,
    pub stopped_by_max: bool,
    pub loop_summary: crate::tool_loop::ToolLoopSummary,
    pub stop_outcome: Option<StopOutcome>,
}

/// Run the tool-calling pipeline: model plans and executes tools directly.
/// Returns structured result with direct tool-loop metadata.
pub(crate) async fn run_tool_calling_pipeline(
    runtime: &mut AppRuntime,
    line: &str,
    tui: &mut crate::ui_terminal::TerminalUI,
    context_hint: &str,
    evidence_required: bool,
    complexity: &str,
) -> Result<ToolCallingPipelineResult> {
    let system_prompt = build_tool_calling_system_prompt(runtime, line);
    trace(
        &runtime.args,
        "tool_calling: direct model planning (no Maestro)",
    );

    // Task 761: Build structured turn context instead of concatenating evidence into the user line
    let mut turn_ctx = crate::turn_context_packet::CurrentTurnContext::new(line);
    if let Some(ref prior_evidence) = runtime.state.last_evidence_summary {
        trace(
            &runtime.args,
            "tool_loop: injected cross-cycle evidence summary (via CurrentTurnContext)",
        );
        let hints = if let Some(ref stop_outcome) = runtime.state.last_stop_outcome {
            Some(format!("Previous stop reason: {}", stop_outcome.reason.as_str()))
        } else {
            None
        };
        turn_ctx.prior_relevant_evidence = Some(prior_evidence.clone());
        turn_ctx.runtime_recovery_hints = hints;
        turn_ctx.system_guidance = Some(
            "Do NOT repeat steps already completed. Continue from where you left off.".to_string(),
        );
    }
    let model_message = turn_ctx.build_model_message();

    // Tasks 763-769: Initialize work graph runner for MULTISTEP/OPEN_ENDED.
    // Derives coverage from graph sub-goal nodes, enforces node commitment,
    // and gates finalization on graph completion + coverage satisfaction.
    let mut work_graph_runner = crate::work_graph_runner::WorkGraphRunner::new(complexity, line);
    if work_graph_runner.is_graph_driven() {
        trace(
            &runtime.args,
            &format!(
                "work_graph_runner: graph_driven complexity={}",
                complexity
            ),
        );
        // Request a work graph schema from the model — shallow outline of
        // execution phases. The model knows nothing about actual workspace
        // files; it produces only generic actions (discover/read_all/answer).
        let schema_profile = &runtime.config.profiles.orchestrator_cfg;
        match crate::intel_units::request_workgraph_schema(
            &runtime.config.client,
            schema_profile,
            line,
        )
        .await
        {
            Ok(schema) if schema.has_phases() => {
                let nodes = work_graph_runner.populate_from_schema(&schema);
                work_graph_runner.seed_coverage_from_graph();
                trace(
                    &runtime.args,
                    &format!(
                        "work_graph_runner: schema populated phases={} nodes={} coverage={}",
                        schema.phases.len(),
                        nodes,
                        work_graph_runner.coverage.total()
                    ),
                );
            }
            Ok(_) => {
                trace(
                    &runtime.args,
                    "work_graph_runner: schema empty — using direct mode",
                );
            }
            Err(e) => {
                trace(
                    &runtime.args,
                    &format!("work_graph_runner: schema request failed error={}", e),
                );
                // Continue with empty graph — behaves like direct mode.
            }
        }
    }

    tui.start_status("Executing...");

    let result = run_tool_loop(
        &runtime.args,
        &runtime.config.client,
        &runtime.config.chat_url,
        &runtime.config.model_id,
        &system_prompt,
        &model_message,
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        &runtime.state.session,
        0.2, // temperature — low for reliability
        16384,
        tui,
        Some(&runtime.config.profiles.summarizer_cfg),
        context_hint,
        evidence_required,
        runtime.config.ctx_max,
        &runtime.state.goal_state,
        complexity,
        // Task 761: Pass raw user request separately for artifact extraction
        Some(line),
        // Tasks 763-769: Pass the work graph runner for graph-driven execution
        work_graph_runner,
    )
    .await?;

    tui.complete_status("Done");

    runtime.state.last_stop_outcome = result.stop_outcome.clone();
    runtime.state.last_evidence_summary = result.evidence_progress_summary.clone();

    // Task 610: Evidence ledger clearing moved to app_chat_loop.rs after
    // continuity and evidence contradiction checks so has_evidence is still
    // available for continuity scoring.

    // Strip leaked thinking/tool_call blocks before returning to the user
    let clean_answer = crate::text_utils::strip_thinking_blocks(&result.final_answer);

    Ok(ToolCallingPipelineResult {
        final_answer: clean_answer,
        iterations: result.iterations,
        tool_calls_made: result.tool_calls_made,
        stopped_by_max: result.stopped_by_max,
        loop_summary: result.loop_summary.clone(),
        stop_outcome: result.stop_outcome.clone(),
    })
}

/// Compute risk deterministically from the tool-calling result metadata.
pub(crate) fn compute_program_risk(_tool_calls_made: usize, _iterations: usize) -> ProgramRisk {
    ProgramRisk::Low
}


