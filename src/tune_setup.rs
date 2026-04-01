//! @efficiency-role: scenario-spec
//!
//! Tuning resources preparation and setup.

use crate::tune::TuneResources;
use crate::*;

pub(crate) async fn prepare_tune_resources(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
    intention_tune_cfg: &Profile,
    emit_progress: bool,
) -> Result<TuneResources> {
    if emit_progress {
        calibration_progress(args, &format!("calibrating {model_id}: router support"));
    }

    let elma_cfg = load_agent_config(&model_cfg_dir.join("_elma.config"))?;
    let json_outputter_cfg = load_agent_config(&model_cfg_dir.join("json_outputter.toml"))?;
    set_json_outputter_profile(Some(json_outputter_cfg.clone()));
    if let Ok(cfg) = load_agent_config(&model_cfg_dir.join("final_answer_extractor.toml")) {
        set_final_answer_extractor_profile(Some(cfg));
    }
    let router_cfg = load_agent_config(&model_cfg_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&model_cfg_dir.join("mode_router.toml"))?;
    let speech_act_cfg = load_agent_config(&model_cfg_dir.join("speech_act.toml"))?;
    let planner_master_cfg = load_agent_config(&model_cfg_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&model_cfg_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&model_cfg_dir.join("decider.toml"))?;
    let selector_cfg = load_agent_config(&model_cfg_dir.join("selector.toml"))?;
    let summarizer_cfg = load_agent_config(&model_cfg_dir.join("summarizer.toml"))?;
    let formatter_cfg = load_agent_config(&model_cfg_dir.join("formatter.toml"))?;
    let complexity_cfg = load_agent_config(&model_cfg_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&model_cfg_dir.join("formula_selector.toml"))?;
    let workflow_planner_cfg = load_agent_config(&model_cfg_dir.join("workflow_planner.toml"))?;
    let command_repair_cfg = load_agent_config(&model_cfg_dir.join("command_repair.toml"))?;
    let command_preflight_cfg = load_agent_config(&model_cfg_dir.join("command_preflight.toml"))?;
    let task_semantics_guard_cfg =
        load_agent_config(&model_cfg_dir.join("task_semantics_guard.toml"))?;
    let execution_sufficiency_cfg =
        load_agent_config(&model_cfg_dir.join("execution_sufficiency.toml"))?;
    let scope_builder_cfg = load_agent_config(&model_cfg_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg =
        load_agent_config(&model_cfg_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&model_cfg_dir.join("artifact_classifier.toml"))?;
    let evidence_mode_cfg = load_agent_config(&model_cfg_dir.join("evidence_mode.toml"))?;
    let outcome_verifier_cfg = load_agent_config(&model_cfg_dir.join("outcome_verifier.toml"))?;
    let memory_gate_cfg = load_agent_config(&model_cfg_dir.join("memory_gate.toml"))?;
    let result_presenter_cfg = load_agent_config(&model_cfg_dir.join("result_presenter.toml"))?;
    let claim_checker_cfg = load_agent_config(&model_cfg_dir.join("claim_checker.toml"))?;
    let orchestrator_cfg = load_agent_config(&model_cfg_dir.join("orchestrator.toml"))?;
    let critic_cfg = load_agent_config(&model_cfg_dir.join("critic.toml"))?;
    let logical_reviewer_cfg = load_agent_config(&model_cfg_dir.join("logical_reviewer.toml"))?;
    let efficiency_reviewer_cfg =
        load_agent_config(&model_cfg_dir.join("efficiency_reviewer.toml"))?;
    let risk_reviewer_cfg = load_agent_config(&model_cfg_dir.join("risk_reviewer.toml"))?;
    let refinement_cfg = load_agent_config(&model_cfg_dir.join("refinement.toml"))?;
    let calibration_judge_cfg =
        load_agent_config(&model_cfg_dir.join("calibration_judge.toml"))?;

    let n_probs = 64u32;
    let supports_logprobs = probe_router_support(client, chat_url, model_id, n_probs).await?;
    let cal = RouterCalibration {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        n_probs,
        supports_logprobs,
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
    };
    let cal_path = model_cfg_dir.join("router_calibration.toml");
    save_router_calibration(&cal_path, &cal)?;
    trace(
        args,
        &format!("tune_router_calibration_saved={}", cal_path.display()),
    );

    write_intention_mapping(
        args,
        client,
        chat_url,
        model_cfg_dir,
        intention_tune_cfg,
        emit_progress,
        model_id,
    )
    .await?;

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

    Ok(TuneResources {
        elma_cfg,
        router_cfg,
        mode_router_cfg,
        speech_act_cfg,
        planner_master_cfg,
        planner_cfg,
        decider_cfg,
        selector_cfg,
        summarizer_cfg,
        formatter_cfg,
        json_outputter_cfg,
        complexity_cfg,
        formula_cfg,
        workflow_planner_cfg,
        command_repair_cfg,
        command_preflight_cfg,
        task_semantics_guard_cfg,
        execution_sufficiency_cfg,
        scope_builder_cfg,
        evidence_compactor_cfg,
        artifact_classifier_cfg,
        evidence_mode_cfg,
        outcome_verifier_cfg,
        memory_gate_cfg,
        result_presenter_cfg,
        claim_checker_cfg,
        orchestrator_cfg,
        critic_cfg,
        logical_reviewer_cfg,
        efficiency_reviewer_cfg,
        risk_reviewer_cfg,
        refinement_cfg,
        calibration_judge_cfg,
        cal,
        supports_logprobs,
        n_probs,
        repo,
        ws,
        ws_brief,
        system_content,
        tune_sessions_root,
    })
}

async fn probe_router_support(
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    n_probs: u32,
) -> Result<bool> {
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
        grammar: None,
    };
    let cal_resp = chat_once(client, chat_url, &cal_req).await?;
    Ok(cal_resp
        .choices
        .get(0)
        .and_then(|choice| choice.logprobs.as_ref())
        .is_some())
}

async fn write_intention_mapping(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    model_cfg_dir: &PathBuf,
    intention_tune_cfg: &Profile,
    emit_progress: bool,
    model_id: &str,
) -> Result<()> {
    let scenario_paths = list_intention_scenario_paths()?;
    let scenario_count = scenario_paths.len();
    let mut lines = Vec::new();

    for (index, path) in scenario_paths.into_iter().enumerate() {
        let txt = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
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
            grammar: None,
        };
        let resp = chat_once(client, chat_url, &req).await?;
        let raw = resp
            .choices
            .get(0)
            .and_then(|choice| choice.message.content.clone())
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
    Ok(())
}
