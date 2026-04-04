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
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    temperature: f64,
) -> Program {
    // If the ladder already concluded this is a direct reply-only turn,
    // skip orchestrator JSON generation entirely.
    if should_use_direct_reply_fast_path(line, route_decision, complexity, formula) {
        trace(
            &runtime.args,
            &format!(
                "direct_reply_fast_path route={} formula={}",
                route_decision.route, formula.primary
            ),
        );
        return build_direct_reply_program(line);
    }

    if should_use_direct_shell_fast_path(line, route_decision, workflow_plan, complexity) {
        trace(
            &runtime.args,
            &format!(
                "direct_shell_fast_path route={} complexity={} formula={}",
                route_decision.route, complexity.complexity, formula.primary
            ),
        );
        return build_direct_shell_program(line);
    }

    if request_looks_like_workflow_endurance_audit(line) {
        if let Some(path) = extract_first_path_from_user_text(line) {
            trace(
                &runtime.args,
                &format!("workflow_endurance_authoritative_program path={path}"),
            );
            return build_workflow_endurance_audit_plan_program(line, &path);
        }
    }

    if request_looks_like_entry_point_probe(line) {
        if let Some(path) = extract_first_path_from_user_text(line) {
            trace(
                &runtime.args,
                &format!("entry_point_authoritative_program path={path}"),
            );
            return build_shell_path_probe_program(line, &path);
        }
    }

    // Create a modified orchestrator config with the escalated temperature
    let mut orchestrator_cfg = runtime.profiles.orchestrator_cfg.clone();
    orchestrator_cfg.temperature = temperature;

    match orchestrate_program_once(
        &runtime.client,
        &runtime.chat_url,
        &orchestrator_cfg,
        line,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        &runtime.ws,
        &runtime.ws_brief,
        &runtime.messages,
    )
    .await
    {
        Ok((program, _)) => program,
        Err(error) => {
            trace(
                &runtime.args,
                &format!("orchestrator_repair_parse_error={error}"),
            );

            // If it's a CHAT route, provide a robust direct reply fallback Program
            // instead of trying recovery, which might also fail if the model is being stubborn.
            if route_decision.route.eq_ignore_ascii_case("CHAT") {
                trace(&runtime.args, "chat_route_fallback_program");
                return Program {
                    objective: line.to_string(),
                    steps: vec![Step::Reply {
                        id: "r1".to_string(),
                        instructions: format!("Answer the user's message directly: {}", line),
                        common: StepCommon {
                            purpose: "direct chat response fallback".to_string(),
                            depends_on: Vec::new(),
                            success_condition: "response sent".to_string(),
                            parent_id: None,
                            depth: None,
                            unit_type: None,
                        },
                    }],
                };
            }

            if route_decision.route.eq_ignore_ascii_case("SHELL") {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    if looks_like_natural_language_edit_request(line) {
                        trace(
                            &runtime.args,
                            &format!("edit_path_probe_fallback path={path}"),
                        );
                        return build_edit_path_probe_program(line, &path);
                    }
                    trace(
                        &runtime.args,
                        &format!("shell_path_probe_fallback path={path}"),
                    );
                    return build_shell_path_probe_program(line, &path);
                }
            }

            if request_looks_like_hybrid_audit_masterplan(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("hybrid_masterplan_parse_fallback path={path}"),
                    );
                    return build_hybrid_audit_masterplan_program(line, &path);
                }
            }

            if request_looks_like_architecture_audit(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("architecture_audit_parse_fallback path={path}"),
                    );
                    return build_architecture_audit_plan_program(line, &path);
                }
            }

            if request_looks_like_logging_standardization(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("logging_standardization_parse_fallback path={path}"),
                    );
                    return build_logging_standardization_plan_program(line, &path);
                }
            }

            if request_looks_like_workflow_endurance_audit(line) {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("workflow_endurance_parse_fallback path={path}"),
                    );
                    return build_workflow_endurance_audit_plan_program(line, &path);
                }
            }

            if route_decision.route.eq_ignore_ascii_case("DECIDE") {
                if let Some(path) = extract_first_path_from_user_text(line) {
                    trace(
                        &runtime.args,
                        &format!("decide_path_probe_fallback path={path}"),
                    );
                    return build_decide_path_probe_program(line, &path);
                }
            }

            operator_trace(&runtime.args, "repairing the workflow plan");
            trace_verbose(runtime.verbose, "workflow_recovery=attempting");
            if let Ok(program) = recover_program_once(
                &runtime.client,
                &runtime.chat_url,
                &runtime.profiles.orchestrator_cfg,
                line,
                route_decision,
                workflow_plan,
                complexity,
                scope,
                formula,
                &runtime.ws,
                &runtime.ws_brief,
                &runtime.messages,
                &format!("orchestrator_parse_error: {error}"),
                None,
                &[],
            )
            .await
            {
                trace_verbose(
                    runtime.verbose,
                    "workflow_recovery=ok source=orchestrator_parse_error",
                );
                return program;
            }
            trace_verbose(
                runtime.verbose,
                "workflow_recovery=failed source=orchestrator_parse_error",
            );

            Program {
                objective: "fallback_clarification".to_string(),
                steps: vec![Step::Reply {
                    id: "r1".to_string(),
                    instructions: "Tell the user plainly that Elma could not form a safe valid workflow for this request yet. Ask one concise clarifying question or ask the user to narrow the scope. Do not invent outputs or workspace facts.".to_string(),
                    common: StepCommon {
                        purpose: "ask for clarification after workflow recovery failure".to_string(),
                        depends_on: Vec::new(),
                        success_condition: "the user receives one concise honest clarification request".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                    },
                }],
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
        &runtime.profiles.expert_responder_cfg,
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
