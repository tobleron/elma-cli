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
) -> Program {
    // Maestro → Orchestrator pipeline handles everything now
    match crate::orchestration_core::build_program_from_maestro(runtime, line).await {
        Ok(program) => {
            trace(
                &runtime.args,
                &format!(
                    "maestro_pipeline_generated_steps count={}",
                    program.steps.len()
                ),
            );
            program
        }
        Err(e) => {
            trace(
                &runtime.args,
                &format!("maestro_pipeline_failed error={}", e),
            );
            // Smart fallback: if request mentions a path, create a shell step
            if let Some(path) = extract_first_path_from_user_text(line) {
                trace(
                    &runtime.args,
                    &format!("maestro_fallback_shell path={path}"),
                );
                Program {
                    objective: line.to_string(),
                    steps: vec![
                        Step::Shell {
                            id: "s1".to_string(),
                            cmd: format!("ls -1 '{}'", path),
                            common: StepCommon {
                                purpose: "list directory contents".to_string(),
                                depends_on: vec![],
                                success_condition: "directory listing returned".to_string(),
                                parent_id: None,
                                depth: None,
                                unit_type: None,
                            },
                        },
                        Step::Respond {
                            id: "s2".to_string(),
                            instructions: format!("Present the directory listing for {}.", path),
                            common: StepCommon {
                                purpose: "present findings".to_string(),
                                depends_on: vec!["s1".to_string()],
                                success_condition: "user receives file list".to_string(),
                                parent_id: None,
                                depth: None,
                                unit_type: None,
                            },
                        },
                    ],
                }
            } else {
                build_direct_reply_program(line)
            }
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
