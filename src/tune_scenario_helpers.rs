//! @efficiency-role: scenario-spec
//!
//! Scenario Runtime Evaluation - Helper Functions

use crate::tune::TuneResources;
use crate::*;

/// Evaluate scope against scenario expectations
pub(crate) fn evaluate_scope(scope: &ScopePlan, scenario: &CalibrationScenario) -> (bool, String) {
    let expected_scope =
        !scenario.expected_scope_terms.is_empty() || !scenario.forbidden_scope_terms.is_empty();
    if !expected_scope {
        return (true, "scope not evaluated".to_string());
    }

    let scope_eval_ok = scope_contains_expected_terms(&scope, &scenario.expected_scope_terms)
        && scope_avoids_forbidden_terms(&scope, &scenario.forbidden_scope_terms);
    let scope_eval_reason = if scope_eval_ok {
        "scope matches scenario expectations".to_string()
    } else {
        format!(
            "scope mismatch: expected {:?}, forbidden {:?}, got focus_paths={:?} exclude={:?}",
            scenario.expected_scope_terms,
            scenario.forbidden_scope_terms,
            scope.focus_paths,
            scope.exclude_globs
        )
    };
    (scope_eval_ok, scope_eval_reason)
}

/// Orchestrate and evaluate program for scenario
pub(crate) async fn orchestrate_and_evaluate_program(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    user_message: &str,
    decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    conversation_messages: &[ChatMessage],
    scenario: &CalibrationScenario,
    args: &Args,
) -> Result<(Option<Program>, ProgramEvaluation)> {
    let mut program_opt: Option<Program> = None;
    let mut program_eval = ProgramEvaluation {
        parsed: false,
        parse_error: String::new(),
        shape_ok: false,
        shape_reason: "program not produced".to_string(),
        policy_ok: false,
        policy_reason: "program not produced".to_string(),
        executable_in_tune: false,
        signature: String::new(),
    };

    match orchestrate_program_once(
        client,
        chat_url,
        orchestrator_cfg,
        user_message,
        decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        conversation_messages,
    )
    .await
    {
        Ok((mut program, _)) => {
            if apply_capability_guard(&mut program, decision, true) {
                trace(
                    args,
                    &format!("tune_guard=capability_reply_only file={}", scenario.file),
                );
            }
            program_eval = evaluate_program_for_scenario(&program, scenario);
            program_opt = Some(program);
        }
        Err(error) => {
            program_eval.parse_error = error.to_string();
            program_eval.shape_reason = "program parse failed".to_string();
            program_eval.policy_reason = "program parse failed".to_string();
        }
    }

    Ok((program_opt, program_eval))
}

/// Check program consistency by running orchestration twice
pub(crate) async fn check_program_consistency(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    user_message: &str,
    decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    conversation_messages: &[ChatMessage],
    program_opt: &Option<Program>,
) -> bool {
    if let Some(program) = program_opt {
        if let Ok((mut second_program, _)) = orchestrate_program_once(
            client,
            chat_url,
            orchestrator_cfg,
            user_message,
            decision,
            workflow_plan,
            complexity,
            scope,
            formula,
            ws,
            ws_brief,
            conversation_messages,
        )
        .await
        {
            let _ = apply_capability_guard(&mut second_program, decision, true);
            return program_signature(program) == program_signature(&second_program);
        }
    }
    false
}

/// Calculate tool economy score
fn tool_economy_score(actual: usize, min: Option<usize>, max: Option<usize>) -> f64 {
    let mut score: f64 = 1.0;
    if let Some(min_steps) = min {
        if actual < min_steps {
            score -= 0.2;
        }
    }
    if let Some(max_steps) = max {
        if actual > max_steps {
            score -= 0.2;
        }
    }
    score.max(0.0)
}

/// Check if text contains expected keywords
fn text_contains_keywords(text: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    keywords.iter().any(|k| lower.contains(&k.to_lowercase()))
}

/// Check if text avoids forbidden keywords
fn text_avoids_keywords(text: &str, avoid: &[String]) -> bool {
    if avoid.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    !avoid.iter().any(|k| lower.contains(&k.to_lowercase()))
}

/// Check if text looks like markdown
fn looks_like_markdown(text: &str) -> bool {
    text.contains("```") || text.contains("**") || text.contains("##")
}

/// Execute program and evaluate all runtime metrics
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_and_evaluate_program(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    resources: &TuneResources,
    scenario: &CalibrationScenario,
    user_message: &str,
    decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    conversation_messages: &[ChatMessage],
    program: Program,
    mut actual_step_count: usize,
) -> Result<(
    bool,
    Option<bool>,
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<String>,
)> {
    let session = ensure_session_layout(&resources.tune_sessions_root)?;
    let mut loop_outcome = run_autonomous_loop(
        args,
        client,
        chat_url,
        &session,
        &resources.repo,
        program,
        decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        &resources.ws,
        &resources.ws_brief,
        conversation_messages,
        &resources.orchestrator_cfg,
        &resources.status_message_cfg,
        &resources.planner_cfg,
        &resources.planner_master_cfg,
        &resources.decider_cfg,
        &resources.selector_cfg,
        &resources.summarizer_cfg,
        &resources.command_repair_cfg,
        &resources.command_preflight_cfg,
        &resources.task_semantics_guard_cfg,
        &resources.evidence_compactor_cfg,
        &resources.artifact_classifier_cfg,
        &resources.outcome_verifier_cfg,
        &resources.execution_sufficiency_cfg,
        &resources.critic_cfg,
        &resources.logical_reviewer_cfg,
        &resources.efficiency_reviewer_cfg,
        &resources.risk_reviewer_cfg,
        &resources.refinement_cfg,
    )
    .await?;

    actual_step_count = loop_outcome.program.steps.len();
    let step_results = loop_outcome.step_results;
    let mut final_reply = loop_outcome.final_reply;
    let reasoning_clean = loop_outcome.reasoning_clean;

    let step_exec_ok = step_results.iter().all(|r| r.ok);
    let execution_ok = Some(step_exec_ok);

    let merged_program = loop_outcome.program;
    let sufficiency = check_execution_sufficiency_once(
        client,
        chat_url,
        &resources.execution_sufficiency_cfg,
        user_message,
        decision,
        &merged_program,
        &step_results,
    )
    .await
    .ok();

    let (critic_ok, critic_reason) = if let Some(ref verdict) = sufficiency {
        (
            Some(
                verdict
                    .status
                    .eq_ignore_ascii_case(if step_exec_ok { "ok" } else { "retry" }),
            ),
            Some(verdict.reason.clone()),
        )
    } else {
        (None, Some("sufficiency error".to_string()))
    };

    let shell_summaries: Vec<_> = step_results
        .iter()
        .filter(|r| r.kind == "shell")
        .map(|r| r.summary.clone())
        .collect();
    let (compaction_ok, compaction_reason) = if !shell_summaries.is_empty() {
        let compact_good = shell_summaries
            .iter()
            .all(|s| !s.trim().is_empty() && s.lines().count() <= 24);
        (
            Some(compact_good),
            Some(if compact_good {
                "shell evidence was compacted to a focused summary".to_string()
            } else {
                "shell evidence remained too noisy or empty".to_string()
            }),
        )
    } else {
        (None, None)
    };

    let (classification_ok, classification_reason) = if !scenario.expected_categories.is_empty() {
        let classification_text = step_results
            .iter()
            .map(|r| r.summary.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let classification_good =
            text_contains_keywords(&classification_text, &scenario.expected_categories);
        (
            Some(classification_good),
            Some(if classification_good {
                "artifact categories were present in the evidence summary".to_string()
            } else {
                format!(
                    "missing expected categories {:?}",
                    scenario.expected_categories
                )
            }),
        )
    } else {
        (None, None)
    };

    let reply_instructions = final_reply.take().unwrap_or_else(|| {
        "Respond to the user in plain terminal text. Use any step outputs as evidence.".to_string()
    });

    let evidence_mode = orchestration_helpers::decide_evidence_mode_via_unit(
        client,
        &resources.evidence_mode_cfg,
        user_message,
        decision,
        &reply_instructions,
        &step_results,
    )
    .await
    .unwrap_or_else(|_| EvidenceModeDecision {
        mode: "COMPACT".to_string(),
        reason: "fallback".to_string(),
    });

    let (
        response_ok,
        response_reason,
        response_plain_text,
        presentation_ok,
        presentation_reason,
        claim_check_ok,
        claim_check_reason,
    ) = match generate_final_answer_once(
        client,
        chat_url,
        &resources.elma_cfg,
        &resources.evidence_mode_cfg,
        &resources.result_presenter_cfg,
        &resources.claim_checker_cfg,
        &resources.formatter_cfg,
        &resources.system_content,
        user_message,
        decision,
        &step_results,
        &reply_instructions,
    )
    .await
    {
        Ok((final_text, _)) => {
            evaluate_final_answer(
                client,
                chat_url,
                resources,
                scenario,
                user_message,
                &evidence_mode,
                &step_results,
                &final_text,
            )
            .await
        }
        Err(error) => (
            Some(false),
            Some(format!("reply error: {error}")),
            Some(false),
            Some(false),
            Some("no final answer was produced".to_string()),
            Some(false),
            Some("claim checker skipped because reply generation failed".to_string()),
        ),
    };

    Ok((
        true,
        execution_ok,
        critic_ok,
        critic_reason,
        response_ok,
        response_reason,
        response_plain_text,
        compaction_ok,
        compaction_reason,
        classification_ok,
        classification_reason,
        claim_check_ok,
        claim_check_reason,
        presentation_ok,
        presentation_reason,
    ))
}

/// Evaluate final answer through claim checker and judge
async fn evaluate_final_answer(
    client: &reqwest::Client,
    chat_url: &Url,
    resources: &TuneResources,
    scenario: &CalibrationScenario,
    user_message: &str,
    evidence_mode: &EvidenceModeDecision,
    step_results: &[StepResult],
    final_text: &str,
) -> (
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<bool>,
    Option<String>,
    Option<bool>,
    Option<String>,
) {
    let (claim_check_ok, claim_check_reason) = match claim_check_once(
        client,
        chat_url,
        &resources.claim_checker_cfg,
        user_message,
        evidence_mode,
        step_results,
        final_text,
    )
    .await
    {
        Ok(verdict) => (
            Some(verdict.status.eq_ignore_ascii_case("ok")),
            Some(verdict.reason),
        ),
        Err(_) => (Some(false), Some("claim checker error".to_string())),
    };

    let (response_ok, response_reason, response_plain_text, presentation_ok, presentation_reason) =
        match judge_final_answer_once(
            client,
            chat_url,
            &resources.calibration_judge_cfg,
            scenario,
            user_message,
            step_results,
            final_text,
        )
        .await
        {
            Ok(verdict) => {
                let keyword_ok =
                    text_contains_keywords(final_text, &scenario.expected_answer_keywords)
                        && text_avoids_keywords(final_text, &scenario.avoid_answer_keywords);
                let ok = verdict.status.eq_ignore_ascii_case("pass")
                    && verdict.answered_request
                    && verdict.faithful_to_evidence
                    && verdict.plain_text
                    && keyword_ok;
                let present_ok = verdict.plain_text && keyword_ok;
                (
                    Some(ok),
                    Some(if keyword_ok {
                        verdict.reason
                    } else {
                        "answer keywords did not match scenario expectations".to_string()
                    }),
                    Some(verdict.plain_text),
                    Some(present_ok),
                    Some(if present_ok {
                        "final answer was concise plain text and matched expected content"
                            .to_string()
                    } else {
                        "final answer formatting or content did not match expectations".to_string()
                    }),
                )
            }
            Err(error) => (
                Some(false),
                Some(format!("judge error: {error}")),
                Some(!looks_like_markdown(final_text)),
                Some(false),
                Some("presentation judge failed".to_string()),
            ),
        };

    (
        response_ok,
        response_reason,
        response_plain_text,
        presentation_ok,
        presentation_reason,
        claim_check_ok,
        claim_check_reason,
    )
}
