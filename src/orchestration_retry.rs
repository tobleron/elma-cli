//! @efficiency-role: service-orchestrator
//!
//! Retry Orchestration Module
//!
//! Handles retry logic with temperature escalation and meta-review synthesis.
//! Task 010: Integrated strategy chains for fallback-based retries.

use crate::app::LoadedProfiles;
use crate::*;
use std::future::Future;

async fn await_with_optional_tui<T, F>(
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result,
            _ = tokio::time::sleep(std::time::Duration::from_millis(40)) => {
                if let Some(t) = tui.as_deref_mut() {
                    let _ = t.pump_ui();
                    if let Ok(Some(queued)) = t.poll_busy_submission() {
                        t.enqueue_submission(queued);
                    }
                }
            }
        }
    }
}

/// T306: Hook for dynamic decomposition on failure.
/// Placeholder for future implementation to decompose when model struggles.
fn decompose_on_failure(_attempt: u32, _error_summary: &str) -> bool {
    // TODO: Implement decomposition logic based on struggle detection
    false
}

/// Retry orchestration with strategy chains and temperature escalation.
/// Returns the best program from all attempts, or a meta-review synthesized program.
///
/// Task 010: Now uses strategy fallback chains instead of just temperature escalation.
pub(crate) async fn orchestrate_with_retries(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    initial_program: Program,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    profiles: &LoadedProfiles,
    max_retries: u32,
    temp_step: f64,
    max_temp: f64,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<AutonomousLoopOutcome> {
    let _ = (max_retries, temp_step, max_temp);
    // Compact DSL action protocol supersedes program regeneration and JSON-based retries.
    // Execute the provided program once via the legacy step runner.
    run_autonomous_loop(
        args,
        client,
        chat_url,
        session,
        workdir,
        initial_program,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &profiles.orchestrator_cfg,
        &profiles.status_message_cfg,
        &profiles.planner_cfg,
        &profiles.planner_master_cfg,
        &profiles.decider_cfg,
        &profiles.selector_cfg,
        &profiles.summarizer_cfg,
        &profiles.command_repair_cfg,
        &profiles.command_preflight_cfg,
        &profiles.task_semantics_guard_cfg,
        &profiles.evidence_compactor_cfg,
        &profiles.artifact_classifier_cfg,
        &profiles.outcome_verifier_cfg,
        &profiles.execution_sufficiency_cfg,
        &profiles.critic_cfg,
        &profiles.logical_reviewer_cfg,
        &profiles.efficiency_reviewer_cfg,
        &profiles.risk_reviewer_cfg,
        &profiles.refinement_cfg,
        tui.as_deref_mut(),
    )
    .await
}

// Legacy retry program regeneration and meta-review synthesis were removed by the compact DSL
// action protocol migration (Task 384). Tool-calling uses the action DSL loop instead.
