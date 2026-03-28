use crate::*;

pub(crate) async fn evaluate_response_suite_impl(
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
    let evidence_compactor_cfg =
        load_agent_config(&candidate_dir.join("evidence_compactor.toml"))?;
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
            Ok(value) => value,
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
