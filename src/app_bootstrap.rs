use crate::app::{AppRuntime, LoadedProfiles};
use crate::*;

pub(crate) async fn bootstrap_app() -> Result<Option<AppRuntime>> {
    let args = Args::parse();
    set_reasoning_display(args.show_thinking && args.debug_trace, args.no_color);
    validate_mode_flags(&args)?;

    let cfg_root = config_root_path(&args.config_root)?;
    let (base_url, base_url_source) =
        resolve_base_url(&cfg_root, args.base_url.as_deref(), args.model.as_deref());

    let base = Url::parse(&base_url).context("Invalid --base-url")?;
    let chat_url = base
        .join("/v1/chat/completions")
        .context("Failed to build /v1/chat/completions URL")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let model_id = if let Some(model) = args.model.as_ref().filter(|s| !s.trim().is_empty()) {
        model.trim().to_string()
    } else {
        fetch_first_model_id(&client, &base).await?
    };
    let model_cfg_dir = ensure_model_config_folder(&cfg_root, &base_url, &model_id)?;

    if handle_special_modes(
        &args,
        &client,
        &base,
        &chat_url,
        &base_url,
        &model_id,
        &model_cfg_dir,
        &cfg_root,
    )
    .await?
    {
        return Ok(None);
    }

    let mut profiles = load_profiles(&model_cfg_dir)?;
    sync_and_upgrade_profiles(&args, &model_cfg_dir, &base_url, &model_id, &mut profiles)?;

    let ctx_max = fetch_ctx_max(&client, &base).await.unwrap_or(None);
    let session = prepare_session(&args)?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    persist_workspace_intel(&args, &session, &ws, &ws_brief)?;
    trace(
        &args,
        &format!("base_url_source={base_url_source} value={base_url}"),
    );

    let system_content = build_system_content(&profiles.elma_cfg.system_prompt, &ws, &ws_brief);
    let messages = vec![ChatMessage {
        role: "system".to_string(),
        content: system_content.clone(),
    }];
    emit_startup_banner(&args, &chat_url, &model_id, &model_cfg_dir, &session);

    Ok(Some(AppRuntime {
        args,
        client,
        chat_url,
        model_id,
        model_cfg_dir,
        ctx_max,
        session,
        repo,
        ws,
        ws_brief,
        system_content,
        messages,
        profiles,
    }))
}

fn validate_mode_flags(args: &Args) -> Result<()> {
    let mode_flags = [
        args.tune,
        args.calibrate,
        args.restore_base,
        args.restore_last,
    ];
    if mode_flags.into_iter().filter(|v| *v).count() > 1 {
        anyhow::bail!("Choose only one of --tune, --calibrate, --restore-base, or --restore-last");
    }
    Ok(())
}

async fn handle_special_modes(
    args: &Args,
    client: &reqwest::Client,
    base: &Url,
    chat_url: &Url,
    base_url: &str,
    model_id: &str,
    model_cfg_dir: &PathBuf,
    cfg_root: &PathBuf,
) -> Result<bool> {
    if args.restore_base {
        let baseline_dir = ensure_baseline_profile_set(model_cfg_dir, base_url, model_id)?;
        activate_profile_set(
            model_cfg_dir,
            &baseline_dir,
            base_url,
            model_id,
            "baseline",
            None,
            0.0,
            false,
        )?;
        eprintln!(
            "Restored baseline profiles for {} from {}",
            model_id,
            baseline_dir.display()
        );
        return Ok(true);
    }

    if args.restore_last {
        let fallback_dir = model_fallback_last_active_dir(model_cfg_dir);
        if !fallback_dir.exists() {
            anyhow::bail!(
                "No last-active profile snapshot found for {} at {}",
                model_id,
                fallback_dir.display()
            );
        }
        activate_profile_set(
            model_cfg_dir,
            &fallback_dir,
            base_url,
            model_id,
            "fallback_last_active",
            None,
            0.0,
            false,
        )?;
        eprintln!(
            "Restored last active profiles for {} from {}",
            model_id,
            fallback_dir.display()
        );
        return Ok(true);
    }

    if !(args.tune || args.calibrate) {
        return Ok(false);
    }

    let model_ids = if args.all_models {
        fetch_all_model_ids(client, base).await?
    } else {
        vec![model_id.to_string()]
    };
    for mid in model_ids {
        let dir = ensure_model_config_folder(cfg_root, base_url, &mid)?;
        if args.calibrate {
            let tune_cfg = load_agent_config(&dir.join("intention_tune.toml"))?;
            tune_model(args, client, chat_url, base_url, &dir, &mid, &tune_cfg, true).await?;
        } else {
            let winner = optimize_model(args, client, chat_url, base_url, &dir, &mid).await?;
            eprintln!(
                "Activated tuned profiles for {} with score {:.3} (certified: {}).",
                mid, winner.score, winner.report.summary.certified
            );
            eprintln!("Restore last: cargo run -- --model {} --restore-last", mid);
            eprintln!("Restore base: cargo run -- --model {} --restore-base", mid);
        }
    }
    Ok(true)
}

fn load_profiles(model_cfg_dir: &PathBuf) -> Result<LoadedProfiles> {
    Ok(LoadedProfiles {
        elma_cfg: load_agent_config(&model_cfg_dir.join("_elma.config"))?,
        planner_master_cfg: load_agent_config(&model_cfg_dir.join("planner_master.toml"))?,
        planner_cfg: load_agent_config(&model_cfg_dir.join("planner.toml"))?,
        decider_cfg: load_agent_config(&model_cfg_dir.join("decider.toml"))?,
        selector_cfg: load_agent_config(&model_cfg_dir.join("selector.toml"))?,
        summarizer_cfg: load_agent_config(&model_cfg_dir.join("summarizer.toml"))?,
        formatter_cfg: load_agent_config(&model_cfg_dir.join("formatter.toml"))?,
        complexity_cfg: load_agent_config(&model_cfg_dir.join("complexity_assessor.toml"))?,
        formula_cfg: load_agent_config(&model_cfg_dir.join("formula_selector.toml"))?,
        workflow_planner_cfg: load_agent_config(&model_cfg_dir.join("workflow_planner.toml"))?,
        evidence_mode_cfg: load_agent_config(&model_cfg_dir.join("evidence_mode.toml"))?,
        command_repair_cfg: load_agent_config(&model_cfg_dir.join("command_repair.toml"))?,
        task_semantics_guard_cfg: load_agent_config(
            &model_cfg_dir.join("task_semantics_guard.toml"),
        )?,
        execution_sufficiency_cfg: load_agent_config(
            &model_cfg_dir.join("execution_sufficiency.toml"),
        )?,
        outcome_verifier_cfg: load_agent_config(&model_cfg_dir.join("outcome_verifier.toml"))?,
        memory_gate_cfg: load_agent_config(&model_cfg_dir.join("memory_gate.toml"))?,
        command_preflight_cfg: load_agent_config(&model_cfg_dir.join("command_preflight.toml"))?,
        scope_builder_cfg: load_agent_config(&model_cfg_dir.join("scope_builder.toml"))?,
        evidence_compactor_cfg: load_agent_config(&model_cfg_dir.join("evidence_compactor.toml"))?,
        artifact_classifier_cfg: load_agent_config(&model_cfg_dir.join("artifact_classifier.toml"))?,
        result_presenter_cfg: load_agent_config(&model_cfg_dir.join("result_presenter.toml"))?,
        claim_checker_cfg: load_agent_config(&model_cfg_dir.join("claim_checker.toml"))?,
        orchestrator_cfg: load_agent_config(&model_cfg_dir.join("orchestrator.toml"))?,
        critic_cfg: load_agent_config(&model_cfg_dir.join("critic.toml"))?,
        logical_reviewer_cfg: load_agent_config(&model_cfg_dir.join("logical_reviewer.toml"))?,
        efficiency_reviewer_cfg: load_agent_config(
            &model_cfg_dir.join("efficiency_reviewer.toml"),
        )?,
        risk_reviewer_cfg: load_agent_config(&model_cfg_dir.join("risk_reviewer.toml"))?,
        router_cfg: load_agent_config(&model_cfg_dir.join("router.toml"))?,
        mode_router_cfg: load_agent_config(&model_cfg_dir.join("mode_router.toml"))?,
        speech_act_cfg: load_agent_config(&model_cfg_dir.join("speech_act.toml"))?,
        router_cal: load_router_calibration(&model_cfg_dir.join("router_calibration.toml"))?,
    })
}

fn sync_and_upgrade_profiles(
    args: &Args,
    model_cfg_dir: &PathBuf,
    base_url: &str,
    model_id: &str,
    profiles: &mut LoadedProfiles,
) -> Result<()> {
    let elma_cfg_path = model_cfg_dir.join("_elma.config");
    profiles.elma_cfg.base_url = base_url.to_string();
    profiles.elma_cfg.model = model_id.to_string();
    save_agent_config(&elma_cfg_path, &profiles.elma_cfg)?;

    let router_cfg_path = model_cfg_dir.join("router.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.router_cfg,
        "router",
        "2 = WORKFLOW",
        default_router_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=router.system_prompt");
        save_agent_config(&router_cfg_path, &profiles.router_cfg)?;
    }

    let mode_router_cfg_path = model_cfg_dir.join("mode_router.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.mode_router_cfg,
        "mode_router",
        "1 = INSPECT",
        default_mode_router_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=mode_router.system_prompt");
        save_agent_config(&mode_router_cfg_path, &profiles.mode_router_cfg)?;
    }

    let speech_act_cfg_path = model_cfg_dir.join("speech_act.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.speech_act_cfg,
        "speech_act",
        "1 = CAPABILITY_CHECK",
        default_speech_act_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=speech_act.system_prompt");
        save_agent_config(&speech_act_cfg_path, &profiles.speech_act_cfg)?;
    }

    let orchestrator_cfg_path = model_cfg_dir.join("orchestrator.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "EVIDENCE-FIRST RULES",
        default_orchestrator_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=orchestrator.system_prompt");
        save_agent_config(&orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }

    let critic_cfg_path = model_cfg_dir.join("critic.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.critic_cfg,
        "critic",
        "there is no workspace evidence in the step results",
        default_critic_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=critic.system_prompt");
        save_agent_config(&critic_cfg_path, &profiles.critic_cfg)?;
    }

    apply_prompt_upgrades(
        args,
        &elma_cfg_path,
        &router_cfg_path,
        &mode_router_cfg_path,
        &speech_act_cfg_path,
        &orchestrator_cfg_path,
        &critic_cfg_path,
        profiles,
    )
}

fn apply_prompt_upgrades(
    args: &Args,
    elma_cfg_path: &PathBuf,
    router_cfg_path: &PathBuf,
    mode_router_cfg_path: &PathBuf,
    speech_act_cfg_path: &PathBuf,
    orchestrator_cfg_path: &PathBuf,
    critic_cfg_path: &PathBuf,
    profiles: &mut LoadedProfiles,
) -> Result<()> {
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "ROUTER PRIOR RULES:\n- You will receive a probabilistic route prior over CHAT, SHELL, PLAN, MASTERPLAN, and DECIDE.\n- Treat the route prior as evidence, not a hard rule.\n- If the route prior is uncertain or the user request is genuinely ambiguous, you may output a Program with a single reply step that asks one concise clarifying question.",
    ) {
        trace(args, "upgraded=orchestrator.router_prior");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "- A shell step is for real workspace inspection or execution only. Never use shell steps to print prose, plan lines, or explanations.\n- If the user asks for a plan, prefer a plan or masterplan step plus an optional reply step. Do not emit plan text through shell commands.",
    ) {
        trace(args, "upgraded=orchestrator.shell_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "- If the user asks for one concrete step-by-step plan, use a plan step.\n- If the user asks for a higher-level overall plan across phases, use a masterplan step.",
    ) {
        trace(args, "upgraded=orchestrator.plan_distinction");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "STRUCTURE RULES:\n- Every step must include purpose and success_condition.\n- Use depends_on to reference earlier step ids when a later step consumes prior results.\n- For summarize steps that summarize earlier outputs, leave text empty and set depends_on.\n- Keep programs minimal. Remove any step that does not directly advance the objective.",
    ) {
        trace(args, "upgraded=orchestrator.structure_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "PLAN EXAMPLE:\nUser: Create a step-by-step plan to add a new config file to this Rust project.\nOutput:\n{\"objective\":\"create a concrete plan for adding a config file\",\"steps\":[{\"id\":\"p1\",\"type\":\"plan\",\"goal\":\"Add a new config file to this Rust project.\",\"purpose\":\"plan\",\"depends_on\":[],\"success_condition\":\"a concrete step-by-step plan is saved\"},{\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"Tell the user a step-by-step plan was created and summarize it briefly in plain text.\",\"purpose\":\"answer\",\"depends_on\":[\"p1\"],\"success_condition\":\"the user receives a concise plain-text summary of the saved plan\"}]}",
    ) {
        trace(args, "upgraded=orchestrator.plan_example");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "MINIMALITY RULES:\n- For a step-by-step plan request, default to one plan step plus an optional reply step.\n- Do not inspect src/main.rs, config files, or prompt files just because examples mention them.\n- Only add shell inspection to a plan request when the plan truly depends on current workspace evidence.",
    ) {
        trace(args, "upgraded=orchestrator.minimality_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.router_cfg,
        "router",
        "Important distinctions:\n- Greetings or general knowledge questions are usually 1.\n- Questions about the current project, files, code, or tasks that need planning or decisions are usually 2.",
    ) {
        trace(args, "upgraded=router.examples");
        save_agent_config(router_cfg_path, &profiles.router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.router_cfg,
        "router",
        "- Output must be exactly one digit from 1 to 2.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents whether Elma should enter workflow mode.",
    ) {
        trace(args, "upgraded=router.workflow_rules");
        save_agent_config(router_cfg_path, &profiles.router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.mode_router_cfg,
        "mode_router",
        "Important distinctions:\n- \"What is my current project about?\", \"read Cargo.toml and summarize it\", and \"find where fetch_ctx_max is defined\" are usually 1.\n- \"list files\", \"run tests\", and \"build the project\" are usually 2.\n- \"Create a step-by-step plan\" is 3, not 4.\n- Only choose 4 when the user truly wants an overall master plan.",
    ) {
        trace(args, "upgraded=mode_router.examples");
        save_agent_config(mode_router_cfg_path, &profiles.mode_router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.speech_act_cfg,
        "speech_act",
        "Important distinctions:\n- \"Are you able to list files here?\" is usually 1.\n- \"What is my current project about?\" is usually 2.\n- \"Can you list files?\" and \"Could you run the tests?\" are usually 3 in normal English, because they are indirect requests.",
    ) {
        trace(args, "upgraded=speech_act.examples");
        save_agent_config(speech_act_cfg_path, &profiles.speech_act_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "- If a shell step only prints prose or plan text instead of inspecting or executing something real in the workspace, choose retry.",
    ) {
        trace(args, "upgraded=critic.shell_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "- If the user asked for a step-by-step plan and there is no plan step result, choose retry and provide a corrected Program that uses type \"plan\".\n- If the user asked for an overall or master plan and there is no masterplan step result, choose retry and provide a corrected Program that uses type \"masterplan\".",
    ) {
        trace(args, "upgraded=critic.plan_distinction");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "EVALUATION RULES:\n- Judge whether each step's purpose and success_condition actually advanced the objective.\n- If a step has depends_on, verify the dependent outputs were meaningfully used.\n- For planning requests, reject shell steps unless they gather clearly necessary workspace evidence.\n- Prefer the simplest valid program that can satisfy the request.",
    ) {
        trace(args, "upgraded=critic.evaluation_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "PLAN VALIDATION HINTS:\n- If any successful step_result has type \"plan\", the step-by-step plan requirement is satisfied.\n- If any successful step_result has type \"masterplan\", the master plan requirement is satisfied.\n- For a step-by-step plan request, reject unnecessary shell inspection and prefer a corrected program with only a plan step and an optional reply step.",
    ) {
        trace(args, "upgraded=critic.plan_validation_hints");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "SPEECH-ACT RULES:\n- You will receive a probabilistic speech-act prior over CAPABILITY_CHECK, INFO_REQUEST, and ACTION_REQUEST.\n- If CAPABILITY_CHECK dominates, prefer a reply step that answers whether Elma can do it. Do not execute commands unless the user also asked for action now.\n- INFO_REQUEST may still require workspace inspection before answering.\n- ACTION_REQUEST may use shell, plan, masterplan, or decide steps as needed.",
    ) {
        trace(args, "upgraded=orchestrator.speech_act_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "COMPLEXITY AND FORMULA PRIORS:\n- You may receive a complexity prior and a formula prior.\n- Treat them as guidance, not hard rules.\n- For cleanup, safety review, or comparison requests about the workspace, prefer inspect_decide_reply.\n- If a shell command fails because of regex, glob, quoting, or parser issues, repair it once and continue if safe.",
    ) {
        trace(args, "upgraded=orchestrator.complexity_formula_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "CLEANUP VALIDATION:\n- If the user asked what is safe to clean up and there is no inspected workspace evidence, choose retry.\n- If a cleanup answer classifies files after a failed shell step, choose retry.\n- If a cleanup task used DECIDE without prior inspection, choose retry.",
    ) {
        trace(args, "upgraded=critic.cleanup_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.elma_cfg,
        "_elma",
        prompt_patch_elma_grounding(),
    ) {
        trace(args, "upgraded=elma.grounding_rules");
        save_agent_config(elma_cfg_path, &profiles.elma_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "SPEECH-ACT VALIDATION:\n- If speech_act is CAPABILITY_CHECK and the program executed shell or planning actions without explicit user intent to do so now, choose retry and replace it with a reply-only program.\n- If speech_act is ACTION_REQUEST, reject answers that only talk about capability without attempting the task when it is allowed.",
    ) {
        trace(args, "upgraded=critic.speech_act_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    Ok(())
}

fn prepare_session(args: &Args) -> Result<SessionPaths> {
    let sessions_root = sessions_root_path(&args.sessions_root)?;
    let session = ensure_session_layout(&sessions_root)?;
    set_trace_log_path(Some(session.root.join("trace_debug.log")));
    Ok(session)
}

fn persist_workspace_intel(
    args: &Args,
    session: &SessionPaths,
    ws: &str,
    ws_brief: &str,
) -> Result<()> {
    if !ws.is_empty() {
        let path = session.root.join("workspace.txt");
        std::fs::write(&path, ws.trim().to_string() + "\n")
            .with_context(|| format!("write {}", path.display()))?;
        trace(args, &format!("workspace_context_saved={}", path.display()));
    }
    if !ws_brief.is_empty() {
        let path = session.root.join("workspace_brief.txt");
        std::fs::write(&path, ws_brief.trim().to_string() + "\n")
            .with_context(|| format!("write {}", path.display()))?;
        trace(args, &format!("workspace_brief_saved={}", path.display()));
    }
    Ok(())
}

fn build_system_content(base_prompt: &str, ws: &str, ws_brief: &str) -> String {
    let mut system_content = base_prompt.to_string();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    system_content
}

fn emit_startup_banner(
    args: &Args,
    chat_url: &Url,
    model_id: &str,
    model_cfg_dir: &Path,
    session: &SessionPaths,
) {
    let target = chat_url
        .host_str()
        .map(|host| {
            let port = chat_url.port().map(|p| format!(":{p}")).unwrap_or_default();
            format!("{}://{host}{port}", chat_url.scheme())
        })
        .unwrap_or_else(|| chat_url.to_string());
    let session_name = session
        .root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| session.root.display().to_string());

    if args.no_color {
        eprintln!("Elma");
        eprintln!("  target   {target}");
        eprintln!("  model    {model_id}");
        eprintln!("  config   {}", model_cfg_dir.display());
        eprintln!("  session  {session_name}");
        eprintln!("  commands /exit  /reset  /snapshot  /rollback <id>\n");
        return;
    }

    eprintln!("{}", ansi_orange("Elma"));
    eprintln!("{} {target}", ansi_grey("  target  "));
    eprintln!("{} {model_id}", ansi_grey("  model   "));
    eprintln!("{} {}", ansi_grey("  config  "), model_cfg_dir.display());
    eprintln!("{} {session_name}", ansi_grey("  session "));
    eprintln!(
        "{} /exit  /reset  /snapshot  /rollback <id>\n",
        ansi_grey("  commands")
    );
}
