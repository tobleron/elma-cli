use crate::*;

pub(crate) async fn evaluate_routing_suite(
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    let speech_act_cfg = load_agent_config(&candidate_dir.join("speech_act.toml"))?;
    let router_cfg = load_agent_config(&candidate_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&candidate_dir.join("mode_router.toml"))?;
    let cal = load_router_calibration(&candidate_dir.join("router_calibration.toml")).unwrap_or(
        RouterCalibration {
            version: 1,
            model: model_id.to_string(),
            base_url: String::new(),
            n_probs: 64,
            supports_logprobs: false,
            routes: vec![
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
            ],
        },
    );
    let manifest = load_calibration_manifest()?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);

    let mut speech_correct = 0usize;
    let mut workflow_correct = 0usize;
    let mut mode_correct = 0usize;
    let mut mode_total = 0usize;
    let mut route_correct = 0usize;
    let total = manifest.scenarios.len();

    for scenario in manifest.scenarios {
        let scenario_path = repo
            .join("scenarios")
            .join("intention")
            .join(&scenario.file);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages);

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

        if decision
            .speech_act
            .choice
            .eq_ignore_ascii_case(&scenario.speech_act)
        {
            speech_correct += 1;
        }
        if decision
            .workflow
            .choice
            .eq_ignore_ascii_case(&scenario.workflow)
        {
            workflow_correct += 1;
        }
        if let Some(expected_mode) = scenario.mode.as_ref() {
            mode_total += 1;
            if decision.mode.choice.eq_ignore_ascii_case(expected_mode) {
                mode_correct += 1;
            }
        }
        if decision.route.eq_ignore_ascii_case(&scenario.route) {
            route_correct += 1;
        }
    }

    let speech_acc = metric_accuracy_or_neutral(speech_correct, total);
    let workflow_acc = metric_accuracy_or_neutral(workflow_correct, total);
    let mode_acc = metric_accuracy_or_neutral(mode_correct, mode_total);
    let route_acc = metric_accuracy_or_neutral(route_correct, total);
    let score =
        (speech_acc * 0.25) + (workflow_acc * 0.25) + (mode_acc * 0.10) + (route_acc * 0.40);
    let hard_rejected = speech_acc < 0.65 || workflow_acc < 0.70 || route_acc < 0.70;
    let note = format!(
        "routing_score={score:.4}\nspeech={speech_acc:.3}\nworkflow={workflow_acc:.3}\nmode={mode_acc:.3}\nroute={route_acc:.3}\nhard_rejected={hard_rejected}\n"
    );
    Ok((score, hard_rejected, note))
}

pub(crate) async fn evaluate_workflow_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    let speech_act_cfg = load_agent_config(&candidate_dir.join("speech_act.toml"))?;
    let router_cfg = load_agent_config(&candidate_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&candidate_dir.join("mode_router.toml"))?;
    let complexity_cfg = load_agent_config(&candidate_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&candidate_dir.join("formula_selector.toml"))?;
    let orchestrator_cfg = load_agent_config(&candidate_dir.join("orchestrator.toml"))?;
    let critic_cfg = load_agent_config(&candidate_dir.join("critic.toml"))?;
    let planner_master_cfg = load_agent_config(&candidate_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&candidate_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&candidate_dir.join("decider.toml"))?;
    let summarizer_cfg = load_agent_config(&candidate_dir.join("summarizer.toml"))?;
    let command_repair_cfg = load_agent_config(&candidate_dir.join("command_repair.toml"))?;
    let scope_builder_cfg = load_agent_config(&candidate_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&candidate_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&candidate_dir.join("artifact_classifier.toml"))?;
    let cal = load_router_calibration(&candidate_dir.join("router_calibration.toml")).unwrap_or(
        RouterCalibration {
            version: 1,
            model: model_id.to_string(),
            base_url: String::new(),
            n_probs: 64,
            supports_logprobs: false,
            routes: vec![],
        },
    );
    let manifest = load_calibration_manifest()?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    let tune_sessions_root = sessions_root_path(&args.sessions_root)?.join("_tune_search");

    let scenarios: Vec<CalibrationScenario> = manifest
        .scenarios
        .into_iter()
        .filter(is_workflow_calibration_scenario)
        .collect();

    let mut route_correct = 0usize;
    let mut parse_correct = 0usize;
    let mut shape_correct = 0usize;
    let mut policy_correct = 0usize;
    let mut consistency_correct = 0usize;
    let mut execution_correct = 0usize;
    let mut execution_total = 0usize;
    let mut critic_correct = 0usize;
    let mut critic_total = 0usize;

    for scenario in &scenarios {
        let scenario_path = calibration_scenario_path(&repo, scenario);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages);

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
        if decision.route.eq_ignore_ascii_case(&scenario.route) {
            route_correct += 1;
        }

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
        let memories = load_recent_formula_memories(candidate_dir, 8).unwrap_or_default();
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

        let (mut program, _) = match orchestrate_program_once(
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
            Ok(v) => v,
            Err(_) => continue,
        };

        let _ = apply_capability_guard(&mut program, &decision);
        let program_eval = evaluate_program_for_scenario(&program, scenario);
        if program_eval.parsed {
            parse_correct += 1;
        }
        if program_eval.shape_ok {
            shape_correct += 1;
        }
        if program_eval.policy_ok {
            policy_correct += 1;
        }

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
            if program_signature(&program) == program_signature(&second_program) {
                consistency_correct += 1;
            }
        }

        if program_eval.parsed
            && program_eval.shape_ok
            && program_eval.policy_ok
            && program_eval.executable_in_tune
        {
            execution_total += 1;
            let session = ensure_session_layout(&tune_sessions_root)?;
            let (step_results, _) = execute_program(
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
            let step_ok = step_results.iter().all(|r| r.ok);
            if step_ok {
                execution_correct += 1;
            }

            critic_total += 1;
            if let Ok(verdict) = run_critic_once(
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
                let expected = if step_ok { "ok" } else { "retry" };
                if verdict.status.eq_ignore_ascii_case(expected) {
                    critic_correct += 1;
                }
            }
        }
    }

    let total = scenarios.len();
    let route_acc = metric_accuracy_or_neutral(route_correct, total);
    let parse_acc = metric_accuracy_or_neutral(parse_correct, total);
    let shape_acc = metric_accuracy_or_neutral(shape_correct, total);
    let policy_acc = metric_accuracy_or_neutral(policy_correct, total);
    let consistency_acc = metric_accuracy_or_neutral(consistency_correct, total);
    let execution_acc = metric_accuracy_or_neutral(execution_correct, execution_total);
    let critic_acc = metric_accuracy_or_neutral(critic_correct, critic_total);
    let score = (route_acc * 0.10)
        + (parse_acc * 0.20)
        + (shape_acc * 0.25)
        + (policy_acc * 0.20)
        + (consistency_acc * 0.15)
        + (execution_acc * 0.05)
        + (critic_acc * 0.05);
    let hard_rejected = parse_acc < 0.90 || policy_acc < 0.95 || shape_acc < 0.70;
    let note = format!(
        "workflow_score={score:.4}\nroute={route_acc:.3}\nparse={parse_acc:.3}\nshape={shape_acc:.3}\npolicy={policy_acc:.3}\nconsistency={consistency_acc:.3}\nexecution={execution_acc:.3}\ncritic={critic_acc:.3}\nhard_rejected={hard_rejected}\n"
    );
    Ok((score, hard_rejected, note))
}

pub(crate) async fn evaluate_response_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    let elma_cfg = load_agent_config(&candidate_dir.join("_elma.config"))?;
    let result_presenter_cfg = load_agent_config(&candidate_dir.join("result_presenter.toml"))?;
    let claim_checker_cfg = load_agent_config(&candidate_dir.join("claim_checker.toml"))?;
    let formatter_cfg = load_agent_config(&candidate_dir.join("formatter.toml"))?;
    let calibration_judge_cfg = load_agent_config(&candidate_dir.join("calibration_judge.toml"))?;
    let speech_act_cfg = load_agent_config(&candidate_dir.join("speech_act.toml"))?;
    let router_cfg = load_agent_config(&candidate_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&candidate_dir.join("mode_router.toml"))?;
    let complexity_cfg = load_agent_config(&candidate_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&candidate_dir.join("formula_selector.toml"))?;
    let orchestrator_cfg = load_agent_config(&candidate_dir.join("orchestrator.toml"))?;
    let planner_master_cfg = load_agent_config(&candidate_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&candidate_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&candidate_dir.join("decider.toml"))?;
    let summarizer_cfg = load_agent_config(&candidate_dir.join("summarizer.toml"))?;
    let command_repair_cfg = load_agent_config(&candidate_dir.join("command_repair.toml"))?;
    let scope_builder_cfg = load_agent_config(&candidate_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&candidate_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&candidate_dir.join("artifact_classifier.toml"))?;
    let cal = load_router_calibration(&candidate_dir.join("router_calibration.toml")).unwrap_or(
        RouterCalibration {
            version: 1,
            model: model_id.to_string(),
            base_url: String::new(),
            n_probs: 64,
            supports_logprobs: false,
            routes: vec![],
        },
    );
    let manifest = load_calibration_manifest()?;
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
    let tune_sessions_root = sessions_root_path(&args.sessions_root)?.join("_tune_search");
    let scenarios: Vec<CalibrationScenario> = manifest
        .scenarios
        .into_iter()
        .filter(is_response_calibration_scenario)
        .collect();

    let mut response_correct = 0usize;
    let mut response_total = 0usize;
    let mut route_correct = 0usize;
    let mut route_total = 0usize;
    let mut plain_text_correct = 0usize;
    let mut plain_text_total = 0usize;

    for scenario in &scenarios {
        let scenario_path = calibration_scenario_path(&repo, scenario);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages);

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
        route_total += 1;
        if decision.route.eq_ignore_ascii_case(&scenario.route) {
            route_correct += 1;
        }

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
        let memories = load_recent_formula_memories(candidate_dir, 8).unwrap_or_default();
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
        let (mut program, _) = match orchestrate_program_once(
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
            Ok(v) => v,
            Err(_) => continue,
        };
        let _ = apply_capability_guard(&mut program, &decision);
        let program_eval = evaluate_program_for_scenario(&program, scenario);
        if !(program_eval.parsed
            && program_eval.shape_ok
            && program_eval.policy_ok
            && (scenario.route.eq_ignore_ascii_case("CHAT") || program_eval.executable_in_tune))
        {
            continue;
        }

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
        let reply_instructions = final_reply.unwrap_or_else(|| {
            "Respond to the user in plain terminal text. Use any step outputs as evidence."
                .to_string()
        });
        response_total += 1;
        if let Ok((final_text, _)) = generate_final_answer_once(
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
            plain_text_total += 1;
            if !looks_like_markdown(&final_text) {
                plain_text_correct += 1;
            }
            if let Ok(verdict) = judge_final_answer_once(
                client,
                chat_url,
                &calibration_judge_cfg,
                scenario,
                &user_message,
                &step_results,
                &final_text,
            )
            .await
            {
                if verdict.status.eq_ignore_ascii_case("pass")
                    && verdict.answered_request
                    && verdict.faithful_to_evidence
                    && verdict.plain_text
                {
                    response_correct += 1;
                }
            }
        }
    }

    let route_acc = metric_accuracy_or_neutral(route_correct, route_total);
    let response_acc = metric_accuracy_or_neutral(response_correct, response_total);
    let plain_text_acc = metric_accuracy_or_neutral(plain_text_correct, plain_text_total);
    let score = (route_acc * 0.15) + (response_acc * 0.70) + (plain_text_acc * 0.15);
    let hard_rejected = response_total == 0 || response_acc < 0.60 || plain_text_acc < 0.80;
    let note = format!(
        "response_score={score:.4}\nroute={route_acc:.3}\nresponse={response_acc:.3}\nplain_text={plain_text_acc:.3}\ncovered={response_total}\nhard_rejected={hard_rejected}\n"
    );
    Ok((score, hard_rejected, note))
}

pub(crate) async fn evaluate_candidate_dir(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    candidate_dir: &PathBuf,
    model_id: &str,
    emit_progress: bool,
) -> Result<CandidateScore> {
    sync_profile_dir_base_url_and_model(candidate_dir, base_url, model_id)?;
    let tune_cfg = load_agent_config(&candidate_dir.join("intention_tune.toml"))?;
    tune_model(
        args,
        client,
        chat_url,
        base_url,
        candidate_dir,
        model_id,
        &tune_cfg,
        emit_progress,
    )
    .await?;
    let report = load_calibration_report(&candidate_dir.join("calibration_report.json"))?;
    let efficiency_report =
        load_efficiency_report(&candidate_dir.join("efficiency_report.json")).ok();
    let efficiency_score = efficiency_report
        .as_ref()
        .map(score_efficiency_report)
        .unwrap_or(0.0);
    let score = (0.75 * score_calibration_report(&report)) + (0.25 * efficiency_score);
    let hard_rejected = hard_rejects_calibration_report(&report);
    Ok(CandidateScore {
        name: candidate_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "candidate".to_string()),
        dir: candidate_dir.clone(),
        report,
        score,
        hard_rejected,
    })
}

pub(crate) fn make_candidate_dir(run_root: &Path, name: &str) -> Result<PathBuf> {
    let safe = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let dir = run_root.join("candidates").join(safe);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    Ok(dir)
}

pub(crate) fn select_top_beam(candidates: Vec<CandidateScore>, beam_width: usize) -> Vec<CandidateScore> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut out = Vec::new();
    for candidate in sorted {
        if candidate.hard_rejected {
            continue;
        }
        out.push(candidate);
        if out.len() >= beam_width {
            break;
        }
    }
    out
}

pub(crate) fn select_top_search_beam(
    candidates: Vec<SearchCandidate>,
    beam_width: usize,
) -> Vec<SearchCandidate> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut out = Vec::new();
    for candidate in sorted {
        if candidate.hard_rejected {
            continue;
        }
        out.push(candidate);
        if out.len() >= beam_width {
            break;
        }
    }
    out
}
