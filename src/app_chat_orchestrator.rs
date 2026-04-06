//! @efficiency-role: service-orchestrator
//! App Chat - Program Orchestration and Resolution

use crate::app::*;
use crate::app_chat_builders_advanced::*;
use crate::app_chat_builders_basic::*;
use crate::app_chat_fast_paths::*;
use crate::app_chat_handlers::*;
use crate::app_chat_helpers::*;
use crate::app_chat_patterns::*;
use crate::*;

pub(crate) async fn build_program(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    tui: &mut crate::ui_terminal::TerminalUI,
) -> Program {
    build_program_with_temp(
        runtime,
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        runtime.profiles.orchestrator_cfg.temperature,
        tui,
    )
    .await
}

pub(crate) async fn build_program_with_temp(
    runtime: &AppRuntime,
    line: &str,
    _route_decision: &RouteDecision,
    _workflow_plan: Option<&WorkflowPlannerOutput>,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _temperature: f64,
    tui: &mut crate::ui_terminal::TerminalUI,
) -> Program {
    // Tool-calling pipeline: model plans and executes tools directly (no Maestro)
    match crate::orchestration_core::run_tool_calling_pipeline(runtime, line, tui).await {
        Ok((answer, iterations, tool_calls, stopped_by_max)) => {
            trace(
                &runtime.args,
                &format!(
                    "tool_calling_pipeline: answer_len={} iterations={} tool_calls={} stopped={}",
                    answer.len(),
                    iterations,
                    tool_calls,
                    stopped_by_max,
                ),
            );
            // Return as a single Respond step for the execution framework
            Program {
                objective: line.to_string(),
                steps: vec![Step::Respond {
                    id: "r1".to_string(),
                    instructions: answer,
                    common: StepCommon {
                        purpose: "respond to user".to_string(),
                        depends_on: vec![],
                        success_condition: "user receives answer".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                }],
            }
        }
        Err(e) => {
            trace(
                &runtime.args,
                &format!("tool_calling_pipeline_failed error={}", e),
            );
            // Fallback: direct reply
            build_direct_reply_program(line)
        }
    }
}

pub(crate) async fn resolve_final_text(
    runtime: &AppRuntime,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    final_reply: &mut Option<String>,
) -> Result<(String, Option<u64>)> {
    let reply_instructions = final_reply.clone().unwrap_or_else(|| {
        "Respond to the user in plain terminal text. Use any step outputs as evidence.".to_string()
    });
    let (final_text, usage) = generate_final_answer_once(
        &runtime.client,
        &runtime.chat_url,
        &runtime.profiles.elma_cfg,
        &runtime.profiles.evidence_mode_cfg,
        &runtime.profiles.expert_advisor_cfg,
        &runtime.profiles.result_presenter_cfg,
        &runtime.profiles.claim_checker_cfg,
        &runtime.profiles.formatter_cfg,
        &runtime.system_content,
        &runtime.model_id,
        runtime.chat_url.as_str(),
        line,
        route_decision,
        step_results,
        &reply_instructions,
        &runtime.ws,
        &runtime.ws_brief,
    )
    .await?;

    let preserved = if line.to_ascii_lowercase().contains("entry point") {
        orchestration_helpers::preserve_exact_grounded_path(
            final_text,
            step_results,
            "State the selected exact relative path first.",
        )
    } else {
        final_text
    };

    Ok((preserved, usage))
}
