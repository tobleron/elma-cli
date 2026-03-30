//! @efficiency-role: scenario-spec
//!
//! Scenario runtime evaluation.

use crate::tune::{ScenarioRuntimeOutcome, TuneResources};
use crate::*;

pub(crate) async fn evaluate_runtime_scenario(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    resources: &TuneResources,
    scenario: CalibrationScenario,
) -> Result<ScenarioRuntimeOutcome> {
    let scenario_path = calibration_scenario_path(&resources.repo, &scenario);
    let txt = std::fs::read_to_string(&scenario_path)
        .with_context(|| format!("read {}", scenario_path.display()))?;
    let (user_message, recent_messages) = parse_scenario_dialog(&txt);
    let mut conversation_messages = vec![ChatMessage {
        role: "system".to_string(),
        content: String::new(),
    }];
    conversation_messages.extend(recent_messages.clone());

    let decision = infer_route_prior(
        client,
        chat_url,
        &resources.speech_act_cfg,
        &resources.router_cfg,
        &resources.mode_router_cfg,
        &resources.cal,
        &user_message,
        &resources.ws,
        &resources.ws_brief,
        &conversation_messages,
    )
    .await?;

    let speech_ok = decision
        .speech_act
        .choice
        .eq_ignore_ascii_case(&scenario.speech_act);
    let workflow_ok = decision
        .workflow
        .choice
        .eq_ignore_ascii_case(&scenario.workflow);
    let mode_ok = scenario
        .mode
        .as_ref()
        .map(|mode| decision.mode.choice.eq_ignore_ascii_case(mode));
    let route_ok = decision.route.eq_ignore_ascii_case(&scenario.route);

    let memories = load_recent_formula_memories(
        &resources
            .tune_sessions_root
            .parent()
            .unwrap_or(&resources.tune_sessions_root)
            .to_path_buf(),
        8,
    )
    .unwrap_or_default();
    let (workflow_plan, complexity, scope, formula, _) = derive_planning_prior(
        client,
        chat_url,
        &resources.workflow_planner_cfg,
        &resources.complexity_cfg,
        &resources.scope_builder_cfg,
        &resources.formula_cfg,
        &user_message,
        &decision,
        &resources.ws,
        &resources.ws_brief,
        &memories,
        &conversation_messages,
    )
    .await;
    let expected_scope =
        !scenario.expected_scope_terms.is_empty() || !scenario.forbidden_scope_terms.is_empty();
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
        &resources.orchestrator_cfg,
        &user_message,
        &decision,
        workflow_plan.as_ref(),
        &complexity,
        &scope,
        &formula,
        &resources.ws,
        &resources.ws_brief,
        &conversation_messages,
    )
    .await
    {
        Ok((mut program, _)) => {
            // Tuning uses guards for baseline measurement
            if apply_capability_guard(&mut program, &decision, true) {
                trace(
                    args,
                    &format!("tune_guard=capability_reply_only file={}", scenario.file),
                );
            }
            program_eval = evaluate_program_for_scenario(&program, &scenario);
            program_opt = Some(program);
        }
        Err(error) => {
            program_eval.parse_error = error.to_string();
            program_eval.shape_reason = "program parse failed".to_string();
            program_eval.policy_reason = "program parse failed".to_string();
        }
    }

    let mut consistency_ok = false;
    if let Some(ref program) = program_opt {
        if let Ok((mut second_program, _)) = orchestrate_program_once(
            client,
            chat_url,
            &resources.orchestrator_cfg,
            &user_message,
            &decision,
            workflow_plan.as_ref(),
            &complexity,
            &scope,
            &formula,
            &resources.ws,
            &resources.ws_brief,
            &conversation_messages,
        )
        .await
        {
            // Tuning uses guards for baseline measurement
            let _ = apply_capability_guard(&mut second_program, &decision, true);
            consistency_ok = program_signature(program) == program_signature(&second_program);
        }
    }

    let mut executed_in_tune = false;
    let mut execution_ok = None;
    let mut critic_ok = None;
    let mut critic_reason = None;
    let mut response_ok = None;
    let mut response_reason = None;
    let mut response_plain_text = None;
    let mut compaction_ok = None;
    let mut compaction_reason = None;
    let mut classification_ok = None;
    let mut classification_reason = None;
    let mut claim_check_ok = None;
    let mut claim_check_reason = None;
    let mut presentation_ok = None;
    let mut presentation_reason = None;
    let mut actual_step_count = program_opt
        .as_ref()
        .map(|program| program.steps.len())
        .unwrap_or_default();
    let mut tool_economy = tool_economy_score(
        actual_step_count,
        scenario.minimum_step_count,
        scenario.maximum_step_count,
    );

    if let Some(program) = program_opt.clone() {
        if program_eval.parsed
            && program_eval.shape_ok
            && program_eval.policy_ok
            && (scenario.route.eq_ignore_ascii_case("CHAT") || program_eval.executable_in_tune)
        {
            executed_in_tune = true;
            let session = ensure_session_layout(&resources.tune_sessions_root)?;
            let mut loop_outcome = run_autonomous_loop(
                args,
                client,
                chat_url,
                &session,
                &resources.repo,
                program,
                &decision,
                workflow_plan.as_ref(),
                &complexity,
                &scope,
                &formula,
                &resources.ws,
                &resources.ws_brief,
                &conversation_messages,
                &resources.orchestrator_cfg,
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
            tool_economy = tool_economy_score(
                actual_step_count,
                scenario.minimum_step_count,
                scenario.maximum_step_count,
            );
            let step_results = loop_outcome.step_results;
            let mut final_reply = loop_outcome.final_reply;
            let reasoning_clean = loop_outcome.reasoning_clean;

            let step_exec_ok = step_results.iter().all(|result| result.ok);
            execution_ok = Some(step_exec_ok);

            let merged_program = loop_outcome.program;
            let sufficiency = match check_execution_sufficiency_once(
                client,
                chat_url,
                &resources.execution_sufficiency_cfg,
                &user_message,
                &decision,
                &merged_program,
                &step_results,
            )
            .await
            {
                Ok(verdict) => Some(verdict),
                Err(error) => {
                    critic_reason = Some(format!("sufficiency error: {error}"));
                    None
                }
            };

            if let Some(sufficiency_verdict) = sufficiency.as_ref() {
                critic_reason = Some(sufficiency_verdict.reason.clone());
                critic_ok = Some(
                    sufficiency_verdict
                        .status
                        .eq_ignore_ascii_case(if step_exec_ok { "ok" } else { "retry" }),
                );
            }

            let shell_summaries = step_results
                .iter()
                .filter(|result| result.kind == "shell")
                .map(|result| result.summary.clone())
                .collect::<Vec<_>>();
            if !shell_summaries.is_empty() {
                let compact_good = shell_summaries
                    .iter()
                    .all(|summary| !summary.trim().is_empty() && summary.lines().count() <= 24);
                compaction_ok = Some(compact_good);
                compaction_reason = Some(if compact_good {
                    "shell evidence was compacted to a focused summary".to_string()
                } else {
                    "shell evidence remained too noisy or empty".to_string()
                });
            }

            if !scenario.expected_categories.is_empty() {
                let classification_text = step_results
                    .iter()
                    .map(|result| result.summary.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let classification_good =
                    text_contains_keywords(&classification_text, &scenario.expected_categories);
                classification_ok = Some(classification_good);
                classification_reason = Some(if classification_good {
                    "artifact categories were present in the evidence summary".to_string()
                } else {
                    format!("missing expected categories {:?}", scenario.expected_categories)
                });
            }

            let reply_instructions = final_reply.take().unwrap_or_else(|| {
                "Respond to the user in plain terminal text. Use any step outputs as evidence."
                    .to_string()
            });
            let evidence_mode = decide_evidence_mode_once(
                client,
                chat_url,
                &resources.evidence_mode_cfg,
                &user_message,
                &decision,
                &reply_instructions,
                &step_results,
            )
            .await
            .unwrap_or_else(|_| EvidenceModeDecision {
                mode: "COMPACT".to_string(),
                reason: "fallback".to_string(),
            });

            match generate_final_answer_once(
                client,
                chat_url,
                &resources.elma_cfg,
                &resources.evidence_mode_cfg,
                &resources.result_presenter_cfg,
                &resources.claim_checker_cfg,
                &resources.formatter_cfg,
                &resources.system_content,
                &user_message,
                &decision,
                &step_results,
                &reply_instructions,
            )
            .await
            {
                Ok((final_text, _)) => {
                    if let Ok(verdict) = claim_check_once(
                        client,
                        chat_url,
                        &resources.claim_checker_cfg,
                        &user_message,
                        &evidence_mode,
                        &step_results,
                        &final_text,
                    )
                    .await
                    {
                        claim_check_ok = Some(verdict.status.eq_ignore_ascii_case("ok"));
                        claim_check_reason = Some(verdict.reason);
                    } else {
                        claim_check_ok = Some(false);
                        claim_check_reason =
                            Some("claim checker error".to_string());
                    }

                    match judge_final_answer_once(
                        client,
                        chat_url,
                        &resources.calibration_judge_cfg,
                        &scenario,
                        &user_message,
                        &step_results,
                        &final_text,
                    )
                    .await
                    {
                        Ok(verdict) => {
                            let keyword_ok = text_contains_keywords(
                                &final_text,
                                &scenario.expected_answer_keywords,
                            ) && text_avoids_keywords(
                                &final_text,
                                &scenario.avoid_answer_keywords,
                            );
                            let ok = verdict.status.eq_ignore_ascii_case("pass")
                                && verdict.answered_request
                                && verdict.faithful_to_evidence
                                && verdict.plain_text
                                && keyword_ok;
                            response_plain_text = Some(verdict.plain_text);
                            response_reason = Some(if keyword_ok {
                                verdict.reason
                            } else {
                                "answer keywords did not match scenario expectations".to_string()
                            });
                            response_ok = Some(ok);
                            let present_ok = verdict.plain_text && keyword_ok;
                            presentation_ok = Some(present_ok);
                            presentation_reason = Some(if present_ok {
                                "final answer was concise plain text and matched expected content"
                                    .to_string()
                            } else {
                                "final answer formatting or content did not match expectations"
                                    .to_string()
                            });
                        }
                        Err(error) => {
                            response_reason = Some(format!("judge error: {error}"));
                            response_ok = Some(false);
                            response_plain_text = Some(!looks_like_markdown(&final_text));
                            presentation_ok = Some(false);
                            presentation_reason =
                                Some("presentation judge failed".to_string());
                        }
                    }
                }
                Err(error) => {
                    response_reason = Some(format!("reply error: {error}"));
                    response_ok = Some(false);
                    response_plain_text = Some(false);
                    claim_check_ok = Some(false);
                    claim_check_reason =
                        Some("claim checker skipped because reply generation failed".to_string());
                    presentation_ok = Some(false);
                    presentation_reason = Some("no final answer was produced".to_string());
                }
            }

            if !reasoning_clean {
                claim_check_reason = claim_check_reason
                    .or_else(|| Some("unclean_reasoning_fallback".to_string()));
            }
        }
    }

    let all_ok = speech_ok
        && workflow_ok
        && mode_ok.unwrap_or(true)
        && route_ok
        && (!expected_scope || scope_eval_ok)
        && program_eval.parsed
        && program_eval.shape_ok
        && program_eval.policy_ok
        && consistency_ok
        && compaction_ok.unwrap_or(true)
        && classification_ok.unwrap_or(true)
        && execution_ok.unwrap_or(true)
        && critic_ok.unwrap_or(true)
        && claim_check_ok.unwrap_or(true)
        && presentation_ok.unwrap_or(true)
        && response_ok.unwrap_or(true);

    Ok(ScenarioRuntimeOutcome {
        speech_pair: (scenario.speech_act.clone(), decision.speech_act.choice.clone()),
        workflow_pair: (scenario.workflow.clone(), decision.workflow.choice.clone()),
        mode_pair: scenario
            .mode
            .clone()
            .map(|expected_mode| (expected_mode, decision.mode.choice.clone())),
        route_pair: (scenario.route.clone(), decision.route.clone()),
        scenario_result: ScenarioCalibrationResult {
            suite: scenario.suite.clone(),
            file: scenario.file.clone(),
            notes: scenario.notes.clone(),
            speech_act_expected: scenario.speech_act.clone(),
            speech_act_predicted: decision.speech_act.choice.clone(),
            speech_act_probability: probability_of(
                &decision.speech_act.distribution,
                &scenario.speech_act,
            ),
            speech_act_ok: speech_ok,
            workflow_expected: scenario.workflow.clone(),
            workflow_predicted: decision.workflow.choice.clone(),
            workflow_probability: probability_of(
                &decision.workflow.distribution,
                &scenario.workflow,
            ),
            workflow_ok,
            mode_expected: scenario.mode.clone(),
            mode_predicted: scenario.mode.as_ref().map(|_| decision.mode.choice.clone()),
            mode_probability: scenario
                .mode
                .as_ref()
                .map(|mode| probability_of(&decision.mode.distribution, mode)),
            mode_ok,
            route_expected: scenario.route.clone(),
            route_predicted: decision.route.clone(),
            route_probability: probability_of(&decision.distribution, &scenario.route),
            route_ok,
            program_signature: program_eval.signature.clone(),
            program_parse_ok: program_eval.parsed,
            program_parse_error: program_eval.parse_error,
            program_shape_ok: program_eval.shape_ok,
            program_shape_reason: program_eval.shape_reason,
            program_policy_ok: program_eval.policy_ok,
            program_policy_reason: program_eval.policy_reason,
            program_consistency_ok: consistency_ok,
            executed_in_tune,
            execution_ok,
            critic_ok,
            critic_reason,
            response_ok,
            response_reason,
            response_plain_text,
            scope_ok: if expected_scope { Some(scope_eval_ok) } else { None },
            scope_reason: if expected_scope {
                Some(scope_eval_reason)
            } else {
                None
            },
            compaction_ok,
            compaction_reason,
            classification_ok,
            classification_reason,
            claim_check_ok,
            claim_check_reason,
            presentation_ok,
            presentation_reason,
            tool_economy_score: Some(tool_economy),
            all_ok,
        },
        efficiency_result: EfficiencyScenarioResult {
            suite: scenario.suite,
            file: scenario.file,
            task_success: all_ok,
            grounding_ok: response_ok,
            scope_ok: if expected_scope { Some(scope_eval_ok) } else { None },
            compaction_ok,
            classification_ok,
            claim_check_ok,
            presentation_ok,
            tool_economy_score: tool_economy,
            actual_steps: actual_step_count,
            expected_min_steps: scenario.minimum_step_count,
            expected_max_steps: scenario.maximum_step_count,
        },
    })
}
