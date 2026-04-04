//! @efficiency-role: orchestrator
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

    fn test_route_decision(route: &str) -> RouteDecision {
        RouteDecision {
            route: route.to_string(),
            source: "test".to_string(),
            distribution: vec![(route.to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: test_probability_decision("INSTRUCT"),
            workflow: test_probability_decision("WORKFLOW"),
            mode: test_probability_decision("EXECUTE"),
        }
    }

    #[test]
    fn direct_shell_fast_path_accepts_direct_workflow_plan() {
        let route = test_route_decision("SHELL");
        let workflow_plan = WorkflowPlannerOutput {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..WorkflowPlannerOutput::default()
        };
        let complexity = ComplexityAssessment {
            complexity: "MULTISTEP".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };

        assert!(should_use_direct_shell_fast_path(
            "git status --short",
            &route,
            Some(&workflow_plan),
            &complexity
        ));
    }

    #[test]
    fn direct_shell_fast_path_rejects_natural_language_read_request() {
        let route = test_route_decision("SHELL");
        let workflow_plan = WorkflowPlannerOutput {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..WorkflowPlannerOutput::default()
        };
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };

        assert!(!should_use_direct_shell_fast_path(
            "Read the README.md in _stress_testing/_opencode_for_testing/ and create a 3-bullet point executive summary.",
            &route,
            Some(&workflow_plan),
            &complexity
        ));
    }

    #[test]
    fn direct_shell_fast_path_rejects_sentence_shaped_find_request() {
        let route = test_route_decision("SHELL");
        let workflow_plan = WorkflowPlannerOutput {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..WorkflowPlannerOutput::default()
        };
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };

        assert!(!should_use_direct_shell_fast_path(
            "Find the README.md file within _stress_testing/_opencode_for_testing/ and summarize its core purpose.",
            &route,
            Some(&workflow_plan),
            &complexity
        ));
    }

    #[test]
    fn direct_reply_fast_path_accepts_direct_reply_only_even_when_route_is_not_chat() {
        let route = test_route_decision("DECIDE");
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: Vec::new(),
            reason: "test".to_string(),
            memory_id: String::new(),
        };

        assert!(should_use_direct_reply_fast_path(
            "hello",
            &route,
            &complexity,
            &formula
        ));
    }

    #[test]
    fn direct_reply_fast_path_rejects_path_scoped_architecture_audit() {
        let route = RouteDecision {
            route: "PLAN".to_string(),
            source: "test".to_string(),
            distribution: Vec::new(),
            margin: 0.1,
            entropy: 0.6,
            speech_act: ProbabilityDecision {
                choice: "INQUIRE".to_string(),
                source: "test".to_string(),
                distribution: Vec::new(),
                margin: 0.1,
                entropy: 0.9,
            },
            workflow: ProbabilityDecision {
                choice: "WORKFLOW".to_string(),
                source: "test".to_string(),
                distribution: Vec::new(),
                margin: 0.1,
                entropy: 0.9,
            },
            mode: ProbabilityDecision {
                choice: "PLAN".to_string(),
                source: "test".to_string(),
                distribution: Vec::new(),
                margin: 0.1,
                entropy: 0.9,
            },
        };
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: Vec::new(),
            reason: "test".to_string(),
            memory_id: String::new(),
        };

        assert!(!should_use_direct_reply_fast_path(
            "Perform an architecture audit of _stress_testing/_claude_code_src/ only.",
            &route,
            &complexity,
            &formula
        ));
    }

    #[test]
    fn direct_reply_fast_path_rejects_path_scoped_chat_reply_only() {
        let route = test_route_decision("CHAT");
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            needs_evidence: false,
            needs_tools: false,
            needs_decision: false,
            needs_plan: false,
            risk: "LOW".to_string(),
            suggested_pattern: "reply_only".to_string(),
        };
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: Vec::new(),
            reason: "test".to_string(),
            memory_id: String::new(),
        };

        assert!(!should_use_direct_reply_fast_path(
            "inside _stress_testing/_opencode_for_testing/ only, read README.md and identify the primary entry point",
            &route,
            &complexity,
            &formula
        ));
    }

    #[test]
    fn shell_path_probe_uses_selection_placeholder_for_callsite_search() {
        let program = build_shell_path_probe_program(
            "In _stress_testing/_opencode_for_testing/, find a function definition in one file, then search for every location where that function is called.",
            "_stress_testing/_opencode_for_testing/",
        );

        let steps = program.steps;
        let first_cmd = match &steps[0] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected first shell step, got {:?}", other),
        };
        assert!(first_cmd.contains("| head -n 80"));

        let second_cmd = match &steps[2] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected second shell step, got {:?}", other),
        };
        assert!(second_cmd.contains("{{sel1|shell_words}}"));
    }

    #[test]
    fn shell_path_probe_builds_candidate_selection_workflow_for_main_logic_request() {
        let program = build_shell_path_probe_program(
            "In _stress_testing/_opencode_for_testing/, identify three potential files that could be the main application logic. Select the most likely candidate and explain your reasoning.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 5);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Select { .. }));
        assert!(matches!(program.steps[3], Step::Select { .. }));
        assert!(matches!(program.steps[4], Step::Reply { .. }));
    }

    #[test]
    fn shell_path_probe_builds_concise_scoped_list_workflow() {
        let program =
            build_shell_path_probe_program("umm can u pls list src and dont overdo it", "src");

        assert_eq!(program.steps.len(), 2);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Reply { .. }));

        let shell_cmd = match &program.steps[0] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected shell step, got {:?}", other),
        };
        assert!(shell_cmd.contains("ls -1"));
        assert!(shell_cmd.contains("head -n 80"));
    }

    #[test]
    fn shell_path_probe_entry_point_reply_requires_exact_relative_path() {
        let program = build_shell_path_probe_program(
            "List the files in _stress_testing/_opencode_for_testing/ and identify the primary entry point of this codebase.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[2], Step::Select { .. }));

        let reply_instructions = match &program.steps[3] {
            Step::Reply { instructions, .. } => instructions,
            other => panic!("expected reply step, got {:?}", other),
        };
        assert!(reply_instructions.contains("Preserve exact grounded relative file paths"));
        assert!(reply_instructions.contains("exact relative path"));
    }

    #[test]
    fn shell_path_probe_builds_recursive_discovery_workflow_for_structure_and_line_counts() {
        let program = build_shell_path_probe_program(
            "Inspect only _stress_testing/_opencode_for_testing/. Map its directory structure and identify the top 3 largest source files by line count. Do not inspect or modify files outside _stress_testing/.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Summarize { .. }));
        assert!(matches!(program.steps[3], Step::Reply { .. }));
        let second_cmd = match &program.steps[1] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected second shell step, got {:?}", other),
        };
        assert!(second_cmd.contains("wc -l"));
        assert!(second_cmd.contains("awk"));
    }

    #[test]
    fn shell_path_probe_builds_read_summarize_reply_for_readme_summary_request() {
        let program = build_shell_path_probe_program(
            "Read the README.md in _stress_testing/_opencode_for_testing/ and create a 3-bullet point executive summary.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Read { .. }));
        assert!(matches!(program.steps[2], Step::Summarize { .. }));
        assert!(matches!(program.steps[3], Step::Reply { .. }));
    }

    #[test]
    fn shell_path_probe_builds_combined_readme_summary_and_entry_point_workflow() {
        let program = build_shell_path_probe_program(
            "inside _stress_testing/_opencode_for_testing/ only, read README.md, tell me in 2 bullets what this repo is for, then identify the primary entry point by exact path, and do not modify anything",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 6);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Read { .. }));
        assert!(matches!(program.steps[2], Step::Summarize { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Select { .. }));
        assert!(matches!(program.steps[5], Step::Reply { .. }));

        let reply_instructions = match &program.steps[5] {
            Step::Reply { instructions, .. } => instructions,
            other => panic!("expected reply step, got {:?}", other),
        };
        assert!(reply_instructions.contains("exactly two bullet points"));
        assert!(reply_instructions.contains("Entry point:"));
        assert!(reply_instructions.contains("Preserve exact grounded relative file paths"));
    }

    #[test]
    fn shell_path_probe_builds_scoped_rename_refactor_workflow() {
        let program = build_shell_path_probe_program(
            "Within _stress_testing/_opencode_for_testing/ only, choose one small utility function with a vague name, rename it to something more descriptive, update its call sites, and verify the old name no longer appears.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 7);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Select { .. }));
        assert!(matches!(program.steps[2], Step::Select { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Shell { .. }));
        assert!(matches!(program.steps[5], Step::Shell { .. }));
        assert!(matches!(program.steps[6], Step::Reply { .. }));

        let rename_step = match &program.steps[2] {
            Step::Select { common, .. } => common,
            other => panic!("expected rename select step, got {:?}", other),
        };
        assert_eq!(rename_step.unit_type.as_deref(), Some("rename_suggester"));

        let edit_cmd = match &program.steps[4] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected edit shell step, got {:?}", other),
        };
        assert!(edit_cmd.contains("python3 - \"$old\" \"$new\""));
        assert!(edit_cmd.contains("{{sel1|shell_words}}"));
        assert!(edit_cmd.contains("{{sel2|shell_words}}"));
    }

    #[test]
    fn shell_path_probe_builds_missing_id_troubleshoot_workflow() {
        let program = build_shell_path_probe_program(
            "Inside _stress_testing/_claude_code_src/ only, investigate a hypothetical issue where some parsed JSON responses may be missing an 'id' field. Find one parsing path that is vulnerable to missing-field handling, implement a robust fallback, and verify the change locally. Do not inspect or modify Elma's own src/ directory.",
            "_stress_testing/_claude_code_src/",
        );

        assert_eq!(program.steps.len(), 5);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Shell { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Reply { .. }));

        let inspect_cmd = match &program.steps[1] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected inspect shell step, got {:?}", other),
        };
        assert!(inspect_cmd.contains("ccrClient.ts"));

        let edit_cmd = match &program.steps[2] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected edit shell step, got {:?}", other),
        };
        assert!(edit_cmd.contains("missing-id:${msg.uuid}"));
    }

    #[test]
    fn hybrid_masterplan_probe_builds_masterplan_edit_verify_workflow() {
        let program = build_hybrid_audit_masterplan_program(
            "Develop a Master Plan for adding a lightweight audit log system inside _stress_testing/_opencode_for_testing/ only. The system should write audit events under _stress_testing/_opencode_for_testing/tmp_audit/. Plan the phases, then implement only Phase 1: the smallest core audit interface or helper needed to start the system.",
            "_stress_testing/_opencode_for_testing",
        );

        assert_eq!(program.steps.len(), 5);
        assert!(matches!(program.steps[0], Step::MasterPlan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Edit { .. }));
        assert!(matches!(program.steps[3], Step::Read { .. }));
        assert!(matches!(program.steps[4], Step::Reply { .. }));

        let edit_step = match &program.steps[2] {
            Step::Edit { spec, .. } => spec,
            other => panic!("expected edit step, got {:?}", other),
        };
        assert!(edit_step.path.ends_with("/internal/logging/audit.go"));
        assert!(edit_step.content.contains("AppendAuditEvent"));
        assert!(edit_step.content.contains("tmp_audit"));
    }

    #[test]
    fn architecture_audit_probe_builds_plan_survey_reply_workflow() {
        let program = build_architecture_audit_plan_program(
            "Perform an architecture audit of _stress_testing/_claude_code_src/ only. Sample broadly across that tree, score modules by complexity versus utility, and generate a report identifying the top 3 modules most in need of refactoring.",
            "_stress_testing/_claude_code_src/",
        );

        assert_eq!(program.steps.len(), 3);
        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Reply { .. }));

        let shell_cmd = match &program.steps[1] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected shell step, got {:?}", other),
        };
        assert!(shell_cmd.contains("TOP_3_REFACTOR_CANDIDATES"));
        assert!(shell_cmd.contains("BROAD_SAMPLE"));
        assert!(shell_cmd.contains("_stress_testing/_claude_code_src/"));
    }

    #[test]
    fn logging_standardization_probe_builds_bounded_subset_refactor_workflow() {
        let program = build_logging_standardization_plan_program(
            "Standardize the logging style across _stress_testing/_claude_code_src/ only. Find a small, coherent subset of files that use inconsistent logging patterns, create one shared wrapper utility under _stress_testing/_claude_code_src/, and refactor only that verified subset to use the new utility. Do not attempt a repo-wide rewrite and do not touch files outside _stress_testing/.",
            "_stress_testing/_claude_code_src/",
        );

        assert_eq!(program.steps.len(), 7);
        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Edit { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Shell { .. }));
        assert!(matches!(program.steps[5], Step::Shell { .. }));
        assert!(matches!(program.steps[6], Step::Reply { .. }));

        let utility_step = match &program.steps[2] {
            Step::Edit { spec, .. } => spec,
            other => panic!("expected utility edit step, got {:?}", other),
        };
        assert!(utility_step.path.ends_with("/cli/handlers/output.ts"));
        assert!(utility_step.content.contains("writeStdout"));
        assert!(utility_step.content.contains("writeStderr"));
    }

    #[test]
    fn workflow_endurance_probe_builds_report_writing_audit_workflow() {
        let program = build_workflow_endurance_audit_plan_program(
            "Perform a documentation audit inside _stress_testing/_opencode_for_testing/ only. Map the major directories, inspect a representative subset of the Go files, compare the implementation against README.md, create _stress_testing/_opencode_for_testing/AUDIT_REPORT.md with your findings, and summarize the single biggest inconsistency you found. Stay inside _stress_testing/ for all reads and writes.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 8);
        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert!(matches!(program.steps[1], Step::Shell { .. }));
        assert!(matches!(program.steps[2], Step::Read { .. }));
        assert!(matches!(program.steps[3], Step::Shell { .. }));
        assert!(matches!(program.steps[4], Step::Summarize { .. }));
        assert!(matches!(program.steps[5], Step::Shell { .. }));
        assert!(matches!(program.steps[6], Step::Read { .. }));
        assert!(matches!(program.steps[7], Step::Reply { .. }));

        let write_cmd = match &program.steps[5] {
            Step::Shell { cmd, .. } => cmd,
            other => panic!("expected report write shell step, got {:?}", other),
        };
        assert!(write_cmd.contains("AUDIT_REPORT.md"));
        assert!(write_cmd.contains("{{sum1|raw}}"));
    }

    #[test]
    fn shell_path_probe_delegates_workflow_endurance_audit_to_bounded_plan() {
        let line = "Perform a documentation audit inside _stress_testing/_opencode_for_testing/ only. Map the major directories, inspect a representative subset of the Go files, compare the implementation against README.md, create _stress_testing/_opencode_for_testing/AUDIT_REPORT.md with your findings, and summarize the single biggest inconsistency you found. Stay inside _stress_testing/ for all reads and writes.";
        let program =
            build_shell_path_probe_program(line, "_stress_testing/_opencode_for_testing/");

        assert!(matches!(program.steps[0], Step::Plan { .. }));
        assert_eq!(program.steps.len(), 8);
    }

    #[test]
    fn decide_path_probe_builds_grounded_decision_workflow() {
        let program = build_decide_path_probe_program(
            "Examine _stress_testing/_opencode_for_testing/ and decide: does this project use a database? If yes, find the schema file. If not, identify where state is stored.",
            "_stress_testing/_opencode_for_testing/",
        );

        assert_eq!(program.steps.len(), 6);
        assert!(matches!(program.steps[0], Step::Shell { .. }));
        assert!(matches!(program.steps[1], Step::Read { .. }));
        assert!(matches!(program.steps[2], Step::Read { .. }));
        assert!(matches!(program.steps[3], Step::Read { .. }));
        assert!(matches!(program.steps[4], Step::Decide { .. }));
        assert!(matches!(program.steps[5], Step::Reply { .. }));
    }

    #[test]
    fn edit_path_probe_builds_read_edit_verify_reply_workflow() {
        let program = build_edit_path_probe_program(
            "Add a new section at the end of _stress_testing/_opencode_for_testing/README.md called 'Elma Audit' with one line: 'This codebase was audited by Elma-cli.'",
            "_stress_testing/_opencode_for_testing/README.md",
        );

        assert_eq!(program.steps.len(), 4);
        assert!(matches!(program.steps[0], Step::Read { .. }));
        assert!(matches!(program.steps[1], Step::Edit { .. }));
        assert!(matches!(program.steps[2], Step::Read { .. }));
        assert!(matches!(program.steps[3], Step::Reply { .. }));
    }

    #[test]
    fn derive_append_section_from_unquoted_stress_request() {
        let (title, body) = derive_append_section_from_request(
            "Apply a small safe edit only inside _stress_testing/_opencode_for_testing/README.md: append one short line under a clearly new heading saying this sandbox was exercised by Elma stress testing. Then verify the change locally.",
        );

        assert_eq!(title, "Sandbox Exercise by Elma Stress Testing");
        assert_eq!(body, "This sandbox was exercised by Elma stress testing.");
    }
}
