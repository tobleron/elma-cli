use crate::*;

pub(crate) async fn tune_model(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
    intention_tune_cfg: &Profile,
    emit_progress: bool,
) -> Result<()> {
    set_trace_log_path(Some(model_cfg_dir.join("trace_debug.log")));
    if emit_progress {
        calibration_progress(args, &format!("calibrating {model_id}: router support"));
    }
    let elma_cfg = load_agent_config(&model_cfg_dir.join("_elma.config"))?;
    let router_cfg = load_agent_config(&model_cfg_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&model_cfg_dir.join("mode_router.toml"))?;
    let speech_act_cfg = load_agent_config(&model_cfg_dir.join("speech_act.toml"))?;
    let planner_master_cfg = load_agent_config(&model_cfg_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&model_cfg_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&model_cfg_dir.join("decider.toml"))?;
    let summarizer_cfg = load_agent_config(&model_cfg_dir.join("summarizer.toml"))?;
    let formatter_cfg = load_agent_config(&model_cfg_dir.join("formatter.toml"))?;
    let complexity_cfg = load_agent_config(&model_cfg_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&model_cfg_dir.join("formula_selector.toml"))?;
    let command_repair_cfg = load_agent_config(&model_cfg_dir.join("command_repair.toml"))?;
    let scope_builder_cfg = load_agent_config(&model_cfg_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&model_cfg_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&model_cfg_dir.join("artifact_classifier.toml"))?;
    let result_presenter_cfg = load_agent_config(&model_cfg_dir.join("result_presenter.toml"))?;
    let claim_checker_cfg = load_agent_config(&model_cfg_dir.join("claim_checker.toml"))?;
    let orchestrator_cfg = load_agent_config(&model_cfg_dir.join("orchestrator.toml"))?;
    let critic_cfg = load_agent_config(&model_cfg_dir.join("critic.toml"))?;
    let calibration_judge_cfg = load_agent_config(&model_cfg_dir.join("calibration_judge.toml"))?;

    // 1) Router calibration: check whether server returns logprobs for top_logprobs.
    // We can't perfectly guarantee inclusion in top_logprobs, but we can verify support and
    // choose an n_probs default that is "big enough".
    let routes = vec![
        "CHAT".to_string(),
        "WORKFLOW".to_string(),
        "INSPECT".to_string(),
        "EXECUTE".to_string(),
        "PLAN".to_string(),
        "MASTERPLAN".to_string(),
        "DECIDE".to_string(),
        "CAPABILITY_CHECK".to_string(),
        "INFO_REQUEST".to_string(),
        "ACTION_REQUEST".to_string(),
    ];
    let n_probs = 64u32;
    let cal_req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "Return exactly one digit: 1.\nNo other text.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            },
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: 1,
        n_probs: Some(n_probs),
        repeat_penalty: None,
        reasoning_format: None,
    };
    let cal_resp = chat_once(client, chat_url, &cal_req).await?;
    let supports_logprobs = cal_resp
        .choices
        .get(0)
        .and_then(|c| c.logprobs.as_ref())
        .is_some();

    let cal = RouterCalibration {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        n_probs,
        supports_logprobs,
        routes,
    };
    let cal_path = model_cfg_dir.join("router_calibration.toml");
    save_router_calibration(&cal_path, &cal)?;
    trace(
        args,
        &format!("tune_router_calibration_saved={}", cal_path.display()),
    );

    // 2) Build intention_mapping.txt from scenario files.
    let scenario_paths = list_intention_scenario_paths()?;
    let mut lines: Vec<String> = Vec::new();
    let scenario_count = scenario_paths.len();
    for (index, p) in scenario_paths.into_iter().enumerate() {
        let txt = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
        let Some(expected) = read_expected_line(&txt) else {
            continue;
        };
        if emit_progress {
            calibration_progress(
                args,
                &format!(
                    "calibrating {model_id}: intention tags {}/{}",
                    index + 1,
                    scenario_count
                ),
            );
        }

        let req = ChatCompletionRequest {
            model: intention_tune_cfg.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: intention_tune_cfg.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: txt,
                },
            ],
            temperature: intention_tune_cfg.temperature,
            top_p: intention_tune_cfg.top_p,
            stream: false,
            max_tokens: intention_tune_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(intention_tune_cfg.repeat_penalty),
            reasoning_format: Some(intention_tune_cfg.reasoning_format.clone()),
        };

        let resp = chat_once(client, chat_url, &req).await?;
        let raw = resp
            .choices
            .get(0)
            .and_then(|c| {
                c.message
                    .content
                    .clone()
                    .or(c.message.reasoning_content.clone())
            })
            .unwrap_or_default();
        let tags = parse_three_tags(&raw);
        lines.push(format!(
            "{}: {}, {}, {}",
            expected, tags[0], tags[1], tags[2]
        ));
    }
    let mapping_path = model_cfg_dir.join("intention_mapping.txt");
    std::fs::write(&mapping_path, lines.join("\n") + "\n")
        .with_context(|| format!("write {}", mapping_path.display()))?;
    trace(
        args,
        &format!("tune_intention_mapping_saved={}", mapping_path.display()),
    );

    // 3) Golden-corpus calibration for runtime probabilistic control.
    let manifest = load_calibration_manifest()?;
    if manifest.version != 1 {
        anyhow::bail!(
            "Unsupported calibration manifest version {}",
            manifest.version
        );
    }
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    let mut system_content = elma_cfg.system_prompt.clone();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let tune_sessions_root = sessions_root_path(&args.sessions_root)?.join("_tune");
    let mut speech_pairs = Vec::new();
    let mut workflow_pairs = Vec::new();
    let mut mode_pairs = Vec::new();
    let mut route_pairs = Vec::new();
    let mut scenario_results = Vec::new();
    let mut speech_correct = 0usize;
    let mut workflow_correct = 0usize;
    let mut mode_correct = 0usize;
    let mut mode_total = 0usize;
    let mut route_correct = 0usize;
    let mut program_parse_correct = 0usize;
    let mut program_shape_correct = 0usize;
    let mut program_policy_correct = 0usize;
    let mut program_consistency_correct = 0usize;
    let mut execution_correct = 0usize;
    let mut execution_total = 0usize;
    let mut critic_correct = 0usize;
    let mut critic_total = 0usize;
    let mut response_correct = 0usize;
    let mut response_total = 0usize;
    let mut scope_correct = 0usize;
    let mut scope_total = 0usize;
    let mut compaction_correct = 0usize;
    let mut compaction_total = 0usize;
    let mut classification_correct = 0usize;
    let mut classification_total = 0usize;
    let mut claim_check_correct = 0usize;
    let mut claim_check_total = 0usize;
    let mut presentation_correct = 0usize;
    let mut presentation_total = 0usize;
    let mut all_ok_correct = 0usize;
    let mut efficiency_scenarios = Vec::new();

    let scenario_total = manifest.scenarios.len();
    for (scenario_index, scenario) in manifest.scenarios.into_iter().enumerate() {
        if emit_progress {
            calibration_progress(
                args,
                &format!(
                    "calibrating {model_id}: runtime suite {}/{} ({})",
                    scenario_index + 1,
                    scenario_total,
                    scenario.file
                ),
            );
        }
        let scenario_path = calibration_scenario_path(&repo, &scenario);
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
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &cal,
            &user_message,
            &ws,
            &ws_brief,
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
            .map(|m| decision.mode.choice.eq_ignore_ascii_case(m));
        let route_ok = decision.route.eq_ignore_ascii_case(&scenario.route);
        let all_ok = speech_ok && workflow_ok && mode_ok.unwrap_or(true) && route_ok;

        if speech_ok {
            speech_correct += 1;
        }
        if workflow_ok {
            workflow_correct += 1;
        }
        if let Some(ok) = mode_ok {
            mode_total += 1;
            if ok {
                mode_correct += 1;
            }
        }
        if route_ok {
            route_correct += 1;
        }

        speech_pairs.push((
            scenario.speech_act.clone(),
            decision.speech_act.choice.clone(),
        ));
        workflow_pairs.push((scenario.workflow.clone(), decision.workflow.choice.clone()));
        if let Some(expected_mode) = scenario.mode.clone() {
            mode_pairs.push((expected_mode, decision.mode.choice.clone()));
        }
        route_pairs.push((scenario.route.clone(), decision.route.clone()));

        let (
            program_signature,
            actual_steps,
            program_parse_ok,
            program_parse_error,
            program_shape_ok,
            program_shape_reason,
            program_policy_ok,
            program_policy_reason,
            program_consistency_ok,
            executed_in_tune,
            execution_ok,
            critic_ok,
            critic_reason,
            response_ok,
            response_reason,
            response_plain_text,
            scope_ok,
            scope_reason,
            compaction_ok,
            compaction_reason,
            classification_ok,
            classification_reason,
            claim_check_ok,
            claim_check_reason,
            presentation_ok,
            presentation_reason,
            tool_economy,
            all_ok,
        ) = {
            let complexity = assess_complexity_once(
                client,
                chat_url,
                &complexity_cfg,
                &user_message,
                &decision,
                &ws,
                &ws_brief,
                &conversation_messages,
            )
            .await
            .unwrap_or_default();
            let scope = build_scope_once(
                client,
                chat_url,
                &scope_builder_cfg,
                &user_message,
                &decision,
                &complexity,
                &ws,
                &ws_brief,
                &conversation_messages,
            )
            .await
            .unwrap_or_default();
            let expected_scope = !scenario.expected_scope_terms.is_empty()
                || !scenario.forbidden_scope_terms.is_empty();
            let scope_eval_ok =
                scope_contains_expected_terms(&scope, &scenario.expected_scope_terms)
                    && scope_avoids_forbidden_terms(&scope, &scenario.forbidden_scope_terms);
            if expected_scope {
                scope_total += 1;
                if scope_eval_ok {
                    scope_correct += 1;
                }
            }
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
            let memories = load_recent_formula_memories(model_cfg_dir, 8).unwrap_or_default();
            let formula = select_formula_once(
                client,
                chat_url,
                &formula_cfg,
                &user_message,
                &decision,
                &complexity,
                &scope,
                &memories,
                &conversation_messages,
            )
            .await
            .unwrap_or_default();
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
                &orchestrator_cfg,
                &user_message,
                &decision,
                &complexity,
                &scope,
                &formula,
                &ws,
                &ws_brief,
                &conversation_messages,
            )
            .await
            {
                Ok((mut program, _raw)) => {
                    if apply_capability_guard(&mut program, &decision) {
                        trace(
                            args,
                            &format!("tune_guard=capability_reply_only file={}", scenario.file),
                        );
                    }
                    program_eval = evaluate_program_for_scenario(&program, &scenario);
                    program_opt = Some(program);
                }
                Err(e) => {
                    program_eval.parse_error = e.to_string();
                    program_eval.shape_reason = "program parse failed".to_string();
                    program_eval.policy_reason = "program parse failed".to_string();
                }
            }

            if program_eval.parsed {
                program_parse_correct += 1;
            }
            if program_eval.shape_ok {
                program_shape_correct += 1;
            }
            if program_eval.policy_ok {
                program_policy_correct += 1;
            }

            let mut consistency_ok = false;
            if let Some(ref program) = program_opt {
                if let Ok((mut second_program, _)) = orchestrate_program_once(
                    client,
                    chat_url,
                    &orchestrator_cfg,
                    &user_message,
                    &decision,
                    &complexity,
                    &scope,
                    &formula,
                    &ws,
                    &ws_brief,
                    &conversation_messages,
                )
                .await
                {
                    let _ = apply_capability_guard(&mut second_program, &decision);
                    consistency_ok =
                        program_signature(program) == program_signature(&second_program);
                }
            }
            if consistency_ok {
                program_consistency_correct += 1;
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
            let tool_economy = tool_economy_score(
                program_opt
                    .as_ref()
                    .map(|p| p.steps.len())
                    .unwrap_or_default(),
                scenario.minimum_step_count,
                scenario.maximum_step_count,
            );

            if let Some(program) = program_opt.clone() {
                if program_eval.parsed
                    && program_eval.shape_ok
                    && program_eval.policy_ok
                    && program_eval.executable_in_tune
                {
                    executed_in_tune = true;
                    execution_total += 1;
                    let session = ensure_session_layout(&tune_sessions_root)?;
                    let (step_results, final_reply) = execute_program(
                        args,
                        client,
                        chat_url,
                        &session,
                        &repo,
                        &program,
                        &planner_cfg,
                        &planner_master_cfg,
                        &decider_cfg,
                        &summarizer_cfg,
                        Some(&command_repair_cfg),
                        Some(&evidence_compactor_cfg),
                        Some(&artifact_classifier_cfg),
                        &scope,
                        &complexity,
                        &formula,
                        &program.objective,
                        false,
                        true,
                    )
                    .await?;
                    let step_exec_ok = step_results.iter().all(|r| r.ok);
                    execution_ok = Some(step_exec_ok);
                    if step_exec_ok {
                        execution_correct += 1;
                    }

                    let shell_summaries = step_results
                        .iter()
                        .filter(|r| r.kind == "shell")
                        .map(|r| r.summary.clone())
                        .collect::<Vec<_>>();
                    if !shell_summaries.is_empty() {
                        compaction_total += 1;
                        let compact_good = shell_summaries
                            .iter()
                            .all(|s| !s.trim().is_empty() && s.lines().count() <= 24);
                        if compact_good {
                            compaction_correct += 1;
                        }
                        compaction_ok = Some(compact_good);
                        compaction_reason = Some(if compact_good {
                            "shell evidence was compacted to a focused summary".to_string()
                        } else {
                            "shell evidence remained too noisy or empty".to_string()
                        });
                    }
                    if !scenario.expected_categories.is_empty() {
                        classification_total += 1;
                        let classification_text = step_results
                            .iter()
                            .map(|r| r.summary.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        let classification_good = text_contains_keywords(
                            &classification_text,
                            &scenario.expected_categories,
                        );
                        if classification_good {
                            classification_correct += 1;
                        }
                        classification_ok = Some(classification_good);
                        classification_reason = Some(if classification_good {
                            "artifact categories were present in the evidence summary".to_string()
                        } else {
                            format!(
                                "missing expected categories {:?}",
                                scenario.expected_categories
                            )
                        });
                    }

                    let expected_critic_ok = step_exec_ok;
                    critic_total += 1;
                    match run_critic_once(
                        client,
                        chat_url,
                        &critic_cfg,
                        &user_message,
                        &decision,
                        &program,
                        &step_results,
                        0,
                    )
                    .await
                    {
                        Ok(verdict) => {
                            let ok = verdict.status.eq_ignore_ascii_case(if expected_critic_ok {
                                "ok"
                            } else {
                                "retry"
                            });
                            if ok {
                                critic_correct += 1;
                            }
                            critic_reason = Some(verdict.reason.clone());
                            critic_ok = Some(ok);
                        }
                        Err(e) => {
                            critic_reason = Some(format!("critic error: {e}"));
                            critic_ok = Some(false);
                        }
                    }

                    let reply_instructions = final_reply.clone().unwrap_or_else(|| {
                            "Respond to the user in plain terminal text. Use any step outputs as evidence."
                                .to_string()
                        });
                    response_total += 1;
                    match generate_final_answer_once(
                        client,
                        chat_url,
                        &elma_cfg,
                        &result_presenter_cfg,
                        &claim_checker_cfg,
                        &formatter_cfg,
                        &system_content,
                        &user_message,
                        &decision,
                        &step_results,
                        &reply_instructions,
                    )
                    .await
                    {
                        Ok((final_text, _)) => {
                            claim_check_total += 1;
                            match claim_check_once(
                                client,
                                chat_url,
                                &claim_checker_cfg,
                                &user_message,
                                &step_results,
                                &final_text,
                            )
                            .await
                            {
                                Ok(verdict) => {
                                    let ok = verdict.status.eq_ignore_ascii_case("ok");
                                    if ok {
                                        claim_check_correct += 1;
                                    }
                                    claim_check_ok = Some(ok);
                                    claim_check_reason = Some(verdict.reason);
                                }
                                Err(e) => {
                                    claim_check_ok = Some(false);
                                    claim_check_reason = Some(format!("claim checker error: {e}"));
                                }
                            }
                            match judge_final_answer_once(
                                client,
                                chat_url,
                                &calibration_judge_cfg,
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
                                    if ok {
                                        response_correct += 1;
                                    }
                                    response_plain_text = Some(verdict.plain_text);
                                    response_reason = Some(if keyword_ok {
                                        verdict.reason
                                    } else {
                                        "answer keywords did not match scenario expectations"
                                            .to_string()
                                    });
                                    response_ok = Some(ok);
                                    presentation_total += 1;
                                    let present_ok = verdict.plain_text && keyword_ok;
                                    if present_ok {
                                        presentation_correct += 1;
                                    }
                                    presentation_ok = Some(present_ok);
                                    presentation_reason = Some(if present_ok {
                                        "final answer was concise plain text and matched expected content".to_string()
                                    } else {
                                        "final answer formatting or content did not match expectations".to_string()
                                    });
                                }
                                Err(e) => {
                                    response_reason = Some(format!("judge error: {e}"));
                                    response_ok = Some(false);
                                    response_plain_text = Some(!looks_like_markdown(&final_text));
                                    presentation_total += 1;
                                    presentation_ok = Some(false);
                                    presentation_reason =
                                        Some("presentation judge failed".to_string());
                                }
                            }
                        }
                        Err(e) => {
                            response_reason = Some(format!("reply error: {e}"));
                            response_ok = Some(false);
                            response_plain_text = Some(false);
                            claim_check_total += 1;
                            claim_check_ok = Some(false);
                            claim_check_reason = Some(
                                "claim checker skipped because reply generation failed".to_string(),
                            );
                            presentation_total += 1;
                            presentation_ok = Some(false);
                            presentation_reason = Some("no final answer was produced".to_string());
                        }
                    }
                }
            }

            let all_ok = speech_ok
                && workflow_ok
                && mode_ok.unwrap_or(true)
                && route_ok
                && scope_eval_ok
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
            if all_ok {
                all_ok_correct += 1;
            }

            (
                program_eval.signature,
                program_opt
                    .as_ref()
                    .map(|p| p.steps.len())
                    .unwrap_or_default(),
                program_eval.parsed,
                program_eval.parse_error,
                program_eval.shape_ok,
                program_eval.shape_reason,
                program_eval.policy_ok,
                program_eval.policy_reason,
                consistency_ok,
                executed_in_tune,
                execution_ok,
                critic_ok,
                critic_reason,
                response_ok,
                response_reason,
                response_plain_text,
                Some(scope_eval_ok),
                Some(scope_eval_reason),
                compaction_ok,
                compaction_reason,
                classification_ok,
                classification_reason,
                claim_check_ok,
                claim_check_reason,
                presentation_ok,
                presentation_reason,
                Some(tool_economy),
                all_ok,
            )
        };

        scenario_results.push(ScenarioCalibrationResult {
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
                .map(|m| probability_of(&decision.mode.distribution, m)),
            mode_ok,
            route_expected: scenario.route.clone(),
            route_predicted: decision.route.clone(),
            route_probability: probability_of(&decision.distribution, &scenario.route),
            route_ok,
            program_signature,
            program_parse_ok,
            program_parse_error,
            program_shape_ok,
            program_shape_reason,
            program_policy_ok,
            program_policy_reason,
            program_consistency_ok,
            executed_in_tune,
            execution_ok,
            critic_ok,
            critic_reason,
            response_ok,
            response_reason,
            response_plain_text,
            scope_ok,
            scope_reason,
            compaction_ok,
            compaction_reason,
            classification_ok,
            classification_reason,
            claim_check_ok,
            claim_check_reason,
            presentation_ok,
            presentation_reason,
            tool_economy_score: tool_economy,
            all_ok,
        });

        efficiency_scenarios.push(EfficiencyScenarioResult {
            suite: scenario.suite.clone(),
            file: scenario.file.clone(),
            task_success: all_ok,
            grounding_ok: response_ok,
            scope_ok,
            compaction_ok,
            classification_ok,
            claim_check_ok,
            presentation_ok,
            tool_economy_score: tool_economy.unwrap_or(0.0),
            actual_steps,
            expected_min_steps: scenario.minimum_step_count,
            expected_max_steps: scenario.maximum_step_count,
        });
    }

    let total = scenario_results.len();
    let summary = CalibrationSummary {
        total_cases: total,
        speech_act: calibration_metric(speech_correct, total),
        workflow: calibration_metric(workflow_correct, total),
        mode: calibration_metric(mode_correct, mode_total),
        route: calibration_metric(route_correct, total),
        program_parse: calibration_metric(program_parse_correct, total),
        program_shape: calibration_metric(program_shape_correct, total),
        program_policy: calibration_metric(program_policy_correct, total),
        program_consistency: calibration_metric(program_consistency_correct, total),
        execution: calibration_metric(execution_correct, execution_total),
        critic: calibration_metric(critic_correct, critic_total),
        response: calibration_metric(response_correct, response_total),
        scope: calibration_metric(scope_correct, scope_total),
        compaction: calibration_metric(compaction_correct, compaction_total),
        classification: calibration_metric(classification_correct, classification_total),
        claim_check: calibration_metric(claim_check_correct, claim_check_total),
        presentation: calibration_metric(presentation_correct, presentation_total),
        all_ok: calibration_metric(all_ok_correct, total),
        certified: total > 0
            && calibration_metric(speech_correct, total).accuracy >= 0.80
            && calibration_metric(workflow_correct, total).accuracy >= 0.85
            && calibration_metric(mode_correct, mode_total).accuracy >= 0.80
            && calibration_metric(route_correct, total).accuracy >= 0.85
            && calibration_metric(program_parse_correct, total).accuracy >= 0.95
            && calibration_metric(program_shape_correct, total).accuracy >= 0.85
            && calibration_metric(program_policy_correct, total).accuracy >= 0.95
            && calibration_metric(program_consistency_correct, total).accuracy >= 0.80
            && calibration_metric(execution_correct, execution_total).accuracy >= 0.80
            && calibration_metric(critic_correct, critic_total).accuracy >= 0.80
            && calibration_metric(response_correct, response_total).accuracy >= 0.80
            && calibration_metric(scope_correct, scope_total).accuracy >= 0.75
            && calibration_metric(compaction_correct, compaction_total).accuracy >= 0.75
            && calibration_metric(classification_correct, classification_total).accuracy >= 0.70
            && calibration_metric(claim_check_correct, claim_check_total).accuracy >= 0.75
            && calibration_metric(presentation_correct, presentation_total).accuracy >= 0.80,
        certification_rule: "speech_act>=0.80 workflow>=0.85 mode>=0.80 route>=0.85 parse>=0.95 shape>=0.85 policy>=0.95 consistency>=0.80 execution>=0.80 critic>=0.80 response>=0.80 scope>=0.75 compaction>=0.75 classification>=0.70 claim_check>=0.75 presentation>=0.80".to_string(),
    };
    let report = CalibrationReport {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        supports_logprobs,
        n_probs,
        summary,
        speech_act_confusions: build_confusions(&speech_pairs),
        workflow_confusions: build_confusions(&workflow_pairs),
        mode_confusions: build_confusions(&mode_pairs),
        route_confusions: build_confusions(&route_pairs),
        scenarios: scenario_results,
    };
    let report_path = model_cfg_dir.join("calibration_report.json");
    save_calibration_report(&report_path, &report)?;
    trace(
        args,
        &format!("tune_calibration_report_saved={}", report_path.display()),
    );

    let efficiency_total = efficiency_scenarios.len();
    let task_success_sum = efficiency_scenarios
        .iter()
        .map(|s| if s.task_success { 1.0 } else { 0.0 })
        .sum::<f64>();
    let grounding_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.grounding_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let grounding_total = efficiency_scenarios
        .iter()
        .filter(|s| s.grounding_ok.is_some())
        .count();
    let scope_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.scope_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let scope_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.scope_ok.is_some())
        .count();
    let compaction_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.compaction_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let compaction_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.compaction_ok.is_some())
        .count();
    let classification_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.classification_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let classification_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.classification_ok.is_some())
        .count();
    let claim_check_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.claim_check_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let claim_check_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.claim_check_ok.is_some())
        .count();
    let presentation_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.presentation_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let presentation_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.presentation_ok.is_some())
        .count();
    let tool_economy_sum = efficiency_scenarios
        .iter()
        .map(|s| s.tool_economy_score)
        .sum::<f64>();
    let efficiency_summary = EfficiencySummary {
        total_cases: efficiency_total,
        task_success_rate: efficiency_metric_from_score(task_success_sum, efficiency_total),
        grounding_rate: efficiency_metric_from_score(grounding_sum, grounding_total),
        scope_precision: efficiency_metric_from_score(scope_sum, scope_metric_total),
        compaction_rate: efficiency_metric_from_score(compaction_sum, compaction_metric_total),
        classification_rate: efficiency_metric_from_score(
            classification_sum,
            classification_metric_total,
        ),
        claim_check_rate: efficiency_metric_from_score(claim_check_sum, claim_check_metric_total),
        presentation_rate: efficiency_metric_from_score(
            presentation_sum,
            presentation_metric_total,
        ),
        tool_economy: efficiency_metric_from_score(tool_economy_sum, efficiency_total),
        overall_efficiency: (0.30
            * efficiency_metric_from_score(task_success_sum, efficiency_total).score)
            + (0.20 * efficiency_metric_from_score(grounding_sum, grounding_total).score)
            + (0.15 * efficiency_metric_from_score(scope_sum, scope_metric_total).score)
            + (0.05 * efficiency_metric_from_score(compaction_sum, compaction_metric_total).score)
            + (0.05
                * efficiency_metric_from_score(classification_sum, classification_metric_total)
                    .score)
            + (0.10
                * efficiency_metric_from_score(claim_check_sum, claim_check_metric_total).score)
            + (0.05
                * efficiency_metric_from_score(presentation_sum, presentation_metric_total).score)
            + (0.10 * efficiency_metric_from_score(tool_economy_sum, efficiency_total).score),
    };
    let efficiency_report = EfficiencyReport {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        summary: efficiency_summary,
        scenarios: efficiency_scenarios,
    };
    let efficiency_path = model_cfg_dir.join("efficiency_report.json");
    save_efficiency_report(&efficiency_path, &efficiency_report)?;
    trace(
        args,
        &format!("tune_efficiency_report_saved={}", efficiency_path.display()),
    );
    if emit_progress {
        calibration_progress(
            args,
            &format!(
                "calibration finished for {model_id}: score {:.3}, certified={}",
                score_calibration_report(&report),
                report.summary.certified
            ),
        );
    }

    Ok(())
}
