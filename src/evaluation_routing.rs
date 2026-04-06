//! @efficiency-role: scenario-spec
//!
//! Routing evaluation suite for calibration.

use crate::*;

pub(crate) async fn evaluate_routing_suite_impl(
    args: &Args,
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
    let manifest = load_tuning_manifest(&args.tune_mode, false)?;
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
        let scenario_path = calibration_scenario_path(&repo, &scenario);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage::simple("system", &String::new())];
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
