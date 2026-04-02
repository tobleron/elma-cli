//! @efficiency-role: scenario-spec
//!
//! Workflow evaluation suite for calibration.

use crate::*;

pub(crate) async fn evaluate_workflow_suite_impl(
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
    let workflow_planner_cfg = load_agent_config(&candidate_dir.join("workflow_planner.toml"))?;
    let orchestrator_cfg = load_agent_config(&candidate_dir.join("orchestrator.toml"))?;
    let status_message_cfg =
        load_agent_config(&candidate_dir.join("status_message_generator.toml"))?;
    let critic_cfg = load_agent_config(&candidate_dir.join("critic.toml"))?;
    let planner_master_cfg = load_agent_config(&candidate_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&candidate_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&candidate_dir.join("decider.toml"))?;
    let selector_cfg = load_agent_config(&candidate_dir.join("selector.toml"))?;
    let summarizer_cfg = load_agent_config(&candidate_dir.join("summarizer.toml"))?;
    let command_repair_cfg = load_agent_config(&candidate_dir.join("command_repair.toml"))?;
    let command_preflight_cfg = load_agent_config(&candidate_dir.join("command_preflight.toml"))?;
    let task_semantics_guard_cfg =
        load_agent_config(&candidate_dir.join("task_semantics_guard.toml"))?;
    let execution_sufficiency_cfg =
        load_agent_config(&candidate_dir.join("execution_sufficiency.toml"))?;
    let outcome_verifier_cfg = load_agent_config(&candidate_dir.join("outcome_verifier.toml"))?;
    let scope_builder_cfg = load_agent_config(&candidate_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&candidate_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&candidate_dir.join("artifact_classifier.toml"))?;
    let logical_reviewer_cfg = load_agent_config(&candidate_dir.join("logical_reviewer.toml"))?;
    let efficiency_reviewer_cfg =
        load_agent_config(&candidate_dir.join("efficiency_reviewer.toml"))?;
    let risk_reviewer_cfg = load_agent_config(&candidate_dir.join("risk_reviewer.toml"))?;
    let refinement_cfg = load_agent_config(&candidate_dir.join("refinement.toml"))?;
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
    let manifest = load_tuning_manifest(&args.tune_mode, true)?;
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

        let memories = load_recent_formula_memories(candidate_dir, 8).unwrap_or_default();
        let (workflow_plan, complexity, scope, formula, _) = derive_planning_prior(
            client,
            chat_url,
            &workflow_planner_cfg,
            &complexity_cfg,
            &scope_builder_cfg,
            &formula_cfg,
            &user_message,
            &decision,
            &ws,
            &ws_brief,
            &memories,
            &conversation_messages,
        )
        .await;

        let (mut program, _) = match orchestrate_program_once(
            client,
            chat_url,
            &orchestrator_cfg,
            &user_message,
            &decision,
            workflow_plan.as_ref(),
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

        // Evaluation uses guards for baseline measurement
        let _ = apply_capability_guard(&mut program, &decision, true);
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
            workflow_plan.as_ref(),
            &complexity,
            &scope,
            &formula,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        {
            // Evaluation uses guards for baseline measurement
            let _ = apply_capability_guard(&mut second_program, &decision, true);
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
            let loop_outcome = run_autonomous_loop(
                args,
                client,
                chat_url,
                &session,
                &repo,
                program,
                &decision,
                workflow_plan.as_ref(),
                &complexity,
                &scope,
                &formula,
                &ws,
                &ws_brief,
                &conversation_messages,
                &orchestrator_cfg,
                &status_message_cfg,
                &planner_cfg,
                &planner_master_cfg,
                &decider_cfg,
                &selector_cfg,
                &summarizer_cfg,
                &command_repair_cfg,
                &command_preflight_cfg,
                &task_semantics_guard_cfg,
                &evidence_compactor_cfg,
                &artifact_classifier_cfg,
                &outcome_verifier_cfg,
                &execution_sufficiency_cfg,
                &critic_cfg,
                &logical_reviewer_cfg,
                &efficiency_reviewer_cfg,
                &risk_reviewer_cfg,
                &refinement_cfg,
            )
            .await?;
            let step_results = loop_outcome.step_results;
            let step_ok = step_results.iter().all(|result| result.ok);
            if step_ok {
                execution_correct += 1;
            }

            critic_total += 1;
            if let Ok(verdict) = check_execution_sufficiency_once(
                client,
                chat_url,
                &execution_sufficiency_cfg,
                &user_message,
                &decision,
                &loop_outcome.program,
                &step_results,
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
