use crate::*;

pub(crate) async fn run() -> Result<()> {
    let args = Args::parse();

    let mode_flags = [
        args.tune,
        args.calibrate,
        args.restore_base,
        args.restore_last,
    ];
    if mode_flags.into_iter().filter(|v| *v).count() > 1 {
        anyhow::bail!("Choose only one of --tune, --calibrate, --restore-base, or --restore-last");
    }

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

    let model_id = if let Some(m) = args.model.as_ref().filter(|s| !s.trim().is_empty()) {
        m.trim().to_string()
    } else {
        fetch_first_model_id(&client, &base).await?
    };

    let model_cfg_dir = ensure_model_config_folder(&cfg_root, &base_url, &model_id)?;

    if args.restore_base {
        let baseline_dir = ensure_baseline_profile_set(&model_cfg_dir, &base_url, &model_id)?;
        activate_profile_set(
            &model_cfg_dir,
            &baseline_dir,
            &base_url,
            &model_id,
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
        return Ok(());
    }

    if args.restore_last {
        let fallback_dir = model_fallback_last_active_dir(&model_cfg_dir);
        if !fallback_dir.exists() {
            anyhow::bail!(
                "No last-active profile snapshot found for {} at {}",
                model_id,
                fallback_dir.display()
            );
        }
        activate_profile_set(
            &model_cfg_dir,
            &fallback_dir,
            &base_url,
            &model_id,
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
        return Ok(());
    }

    if args.tune || args.calibrate {
        let model_ids = if args.all_models {
            fetch_all_model_ids(&client, &base).await?
        } else {
            vec![model_id.clone()]
        };
        for mid in model_ids {
            let dir = ensure_model_config_folder(&cfg_root, &base_url, &mid)?;
            if args.calibrate {
                let tune_cfg = load_agent_config(&dir.join("intention_tune.toml"))?;
                tune_model(
                    &args, &client, &chat_url, &base_url, &dir, &mid, &tune_cfg, true,
                )
                .await?;
            } else {
                let winner =
                    optimize_model(&args, &client, &chat_url, &base_url, &dir, &mid).await?;
                eprintln!(
                    "Activated tuned profiles for {} with score {:.3} (certified: {}).",
                    mid, winner.score, winner.report.summary.certified
                );
                eprintln!("Restore last: cargo run -- --model {} --restore-last", mid);
                eprintln!("Restore base: cargo run -- --model {} --restore-base", mid);
            }
        }
        return Ok(());
    }

    let elma_cfg_path = model_cfg_dir.join("_elma.config");
    let planner_master_cfg_path = model_cfg_dir.join("planner_master.toml");
    let planner_cfg_path = model_cfg_dir.join("planner.toml");
    let decider_cfg_path = model_cfg_dir.join("decider.toml");
    let summarizer_cfg_path = model_cfg_dir.join("summarizer.toml");
    let formatter_cfg_path = model_cfg_dir.join("formatter.toml");
    let complexity_cfg_path = model_cfg_dir.join("complexity_assessor.toml");
    let formula_cfg_path = model_cfg_dir.join("formula_selector.toml");
    let command_repair_cfg_path = model_cfg_dir.join("command_repair.toml");
    let scope_builder_cfg_path = model_cfg_dir.join("scope_builder.toml");
    let evidence_compactor_cfg_path = model_cfg_dir.join("evidence_compactor.toml");
    let artifact_classifier_cfg_path = model_cfg_dir.join("artifact_classifier.toml");
    let result_presenter_cfg_path = model_cfg_dir.join("result_presenter.toml");
    let claim_checker_cfg_path = model_cfg_dir.join("claim_checker.toml");
    let orchestrator_cfg_path = model_cfg_dir.join("orchestrator.toml");
    let critic_cfg_path = model_cfg_dir.join("critic.toml");
    let router_cfg_path = model_cfg_dir.join("router.toml");
    let mode_router_cfg_path = model_cfg_dir.join("mode_router.toml");
    let speech_act_cfg_path = model_cfg_dir.join("speech_act.toml");
    let router_cal_path = model_cfg_dir.join("router_calibration.toml");

    let mut elma_cfg = load_agent_config(&elma_cfg_path)?;
    let planner_master_cfg = load_agent_config(&planner_master_cfg_path)?;
    let planner_cfg = load_agent_config(&planner_cfg_path)?;
    let decider_cfg = load_agent_config(&decider_cfg_path)?;
    let summarizer_cfg = load_agent_config(&summarizer_cfg_path)?;
    let formatter_cfg = load_agent_config(&formatter_cfg_path)?;
    let complexity_cfg = load_agent_config(&complexity_cfg_path)?;
    let formula_cfg = load_agent_config(&formula_cfg_path)?;
    let command_repair_cfg = load_agent_config(&command_repair_cfg_path)?;
    let scope_builder_cfg = load_agent_config(&scope_builder_cfg_path)?;
    let evidence_compactor_cfg = load_agent_config(&evidence_compactor_cfg_path)?;
    let artifact_classifier_cfg = load_agent_config(&artifact_classifier_cfg_path)?;
    let result_presenter_cfg = load_agent_config(&result_presenter_cfg_path)?;
    let claim_checker_cfg = load_agent_config(&claim_checker_cfg_path)?;
    let mut orchestrator_cfg = load_agent_config(&orchestrator_cfg_path)?;
    let mut critic_cfg = load_agent_config(&critic_cfg_path)?;
    let mut router_cfg = load_agent_config(&router_cfg_path)?;
    let mut mode_router_cfg = load_agent_config(&mode_router_cfg_path)?;
    let mut speech_act_cfg = load_agent_config(&speech_act_cfg_path)?;
    let router_cal = load_router_calibration(&router_cal_path)?;

    // Ensure these configs track current base/model (user can still edit files manually).
    elma_cfg.base_url = base_url.clone();
    elma_cfg.model = model_id.clone();
    save_agent_config(&elma_cfg_path, &elma_cfg)?;

    if replace_system_prompt_if_missing(
        &mut router_cfg,
        "router",
        "2 = WORKFLOW",
        default_router_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=router.system_prompt");
        save_agent_config(&router_cfg_path, &router_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut mode_router_cfg,
        "mode_router",
        "1 = INSPECT",
        default_mode_router_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=mode_router.system_prompt");
        save_agent_config(&mode_router_cfg_path, &mode_router_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut speech_act_cfg,
        "speech_act",
        "1 = CAPABILITY_CHECK",
        default_speech_act_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=speech_act.system_prompt");
        save_agent_config(&speech_act_cfg_path, &speech_act_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut orchestrator_cfg,
        "orchestrator",
        "EVIDENCE-FIRST RULES",
        default_orchestrator_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=orchestrator.system_prompt");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut critic_cfg,
        "critic",
        "there is no workspace evidence in the step results",
        default_critic_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=critic.system_prompt");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "ROUTER PRIOR RULES:\n- You will receive a probabilistic route prior over CHAT, SHELL, PLAN, MASTERPLAN, and DECIDE.\n- Treat the route prior as evidence, not a hard rule.\n- If the route prior is uncertain or the user request is genuinely ambiguous, you may output a Program with a single reply step that asks one concise clarifying question.",
    ) {
        trace(&args, "upgraded=orchestrator.router_prior");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "- A shell step is for real workspace inspection or execution only. Never use shell steps to print prose, plan lines, or explanations.\n- If the user asks for a plan, prefer a plan or masterplan step plus an optional reply step. Do not emit plan text through shell commands.",
    ) {
        trace(&args, "upgraded=orchestrator.shell_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "- If the user asks for one concrete step-by-step plan, use a plan step.\n- If the user asks for a higher-level overall plan across phases, use a masterplan step.",
    ) {
        trace(&args, "upgraded=orchestrator.plan_distinction");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "STRUCTURE RULES:\n- Every step must include purpose and success_condition.\n- Use depends_on to reference earlier step ids when a later step consumes prior results.\n- For summarize steps that summarize earlier outputs, leave text empty and set depends_on.\n- Keep programs minimal. Remove any step that does not directly advance the objective.",
    ) {
        trace(&args, "upgraded=orchestrator.structure_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "PLAN EXAMPLE:\nUser: Create a step-by-step plan to add a new config file to this Rust project.\nOutput:\n{\"objective\":\"create a concrete plan for adding a config file\",\"steps\":[{\"id\":\"p1\",\"type\":\"plan\",\"goal\":\"Add a new config file to this Rust project.\",\"purpose\":\"plan\",\"depends_on\":[],\"success_condition\":\"a concrete step-by-step plan is saved\"},{\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"Tell the user a step-by-step plan was created and summarize it briefly in plain text.\",\"purpose\":\"answer\",\"depends_on\":[\"p1\"],\"success_condition\":\"the user receives a concise plain-text summary of the saved plan\"}]}",
    ) {
        trace(&args, "upgraded=orchestrator.plan_example");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "MINIMALITY RULES:\n- For a step-by-step plan request, default to one plan step plus an optional reply step.\n- Do not inspect src/main.rs, config files, or prompt files just because examples mention them.\n- Only add shell inspection to a plan request when the plan truly depends on current workspace evidence.",
    ) {
        trace(&args, "upgraded=orchestrator.minimality_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut router_cfg,
        "router",
        "Important distinctions:\n- Greetings or general knowledge questions are usually 1.\n- Questions about the current project, files, code, or tasks that need planning or decisions are usually 2.",
    ) {
        trace(&args, "upgraded=router.examples");
        save_agent_config(&router_cfg_path, &router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut router_cfg,
        "router",
        "- Output must be exactly one digit from 1 to 2.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents whether Elma should enter workflow mode.",
    ) {
        trace(&args, "upgraded=router.workflow_rules");
        save_agent_config(&router_cfg_path, &router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut mode_router_cfg,
        "mode_router",
        "Important distinctions:\n- \"What is my current project about?\", \"read Cargo.toml and summarize it\", and \"find where fetch_ctx_max is defined\" are usually 1.\n- \"list files\", \"run tests\", and \"build the project\" are usually 2.\n- \"Create a step-by-step plan\" is 3, not 4.\n- Only choose 4 when the user truly wants an overall master plan.",
    ) {
        trace(&args, "upgraded=mode_router.examples");
        save_agent_config(&mode_router_cfg_path, &mode_router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut speech_act_cfg,
        "speech_act",
        "Important distinctions:\n- \"Are you able to list files here?\" is usually 1.\n- \"What is my current project about?\" is usually 2.\n- \"Can you list files?\" and \"Could you run the tests?\" are usually 3 in normal English, because they are indirect requests.",
    ) {
        trace(&args, "upgraded=speech_act.examples");
        save_agent_config(&speech_act_cfg_path, &speech_act_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "- If a shell step only prints prose or plan text instead of inspecting or executing something real in the workspace, choose retry.",
    ) {
        trace(&args, "upgraded=critic.shell_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "- If the user asked for a step-by-step plan and there is no plan step result, choose retry and provide a corrected Program that uses type \"plan\".\n- If the user asked for an overall or master plan and there is no masterplan step result, choose retry and provide a corrected Program that uses type \"masterplan\".",
    ) {
        trace(&args, "upgraded=critic.plan_distinction");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "EVALUATION RULES:\n- Judge whether each step's purpose and success_condition actually advanced the objective.\n- If a step has depends_on, verify the dependent outputs were meaningfully used.\n- For planning requests, reject shell steps unless they gather clearly necessary workspace evidence.\n- Prefer the simplest valid program that can satisfy the request.",
    ) {
        trace(&args, "upgraded=critic.evaluation_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "PLAN VALIDATION HINTS:\n- If any successful step_result has type \"plan\", the step-by-step plan requirement is satisfied.\n- If any successful step_result has type \"masterplan\", the master plan requirement is satisfied.\n- For a step-by-step plan request, reject unnecessary shell inspection and prefer a corrected program with only a plan step and an optional reply step.",
    ) {
        trace(&args, "upgraded=critic.plan_validation_hints");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "SPEECH-ACT RULES:\n- You will receive a probabilistic speech-act prior over CAPABILITY_CHECK, INFO_REQUEST, and ACTION_REQUEST.\n- If CAPABILITY_CHECK dominates, prefer a reply step that answers whether Elma can do it. Do not execute commands unless the user also asked for action now.\n- INFO_REQUEST may still require workspace inspection before answering.\n- ACTION_REQUEST may use shell, plan, masterplan, or decide steps as needed.",
    ) {
        trace(&args, "upgraded=orchestrator.speech_act_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "COMPLEXITY AND FORMULA PRIORS:\n- You may receive a complexity prior and a formula prior.\n- Treat them as guidance, not hard rules.\n- For cleanup, safety review, or comparison requests about the workspace, prefer inspect_decide_reply.\n- If a shell command fails because of regex, glob, quoting, or parser issues, repair it once and continue if safe.",
    ) {
        trace(&args, "upgraded=orchestrator.complexity_formula_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "CLEANUP VALIDATION:\n- If the user asked what is safe to clean up and there is no inspected workspace evidence, choose retry.\n- If a cleanup answer classifies files after a failed shell step, choose retry.\n- If a cleanup task used DECIDE without prior inspection, choose retry.",
    ) {
        trace(&args, "upgraded=critic.cleanup_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(&mut elma_cfg, "_elma", prompt_patch_elma_grounding()) {
        trace(&args, "upgraded=elma.grounding_rules");
        save_agent_config(&elma_cfg_path, &elma_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "SPEECH-ACT VALIDATION:\n- If speech_act is CAPABILITY_CHECK and the program executed shell or planning actions without explicit user intent to do so now, choose retry and replace it with a reply-only program.\n- If speech_act is ACTION_REQUEST, reject answers that only talk about capability without attempting the task when it is allowed.",
    ) {
        trace(&args, "upgraded=critic.speech_act_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }

    let ctx_max = fetch_ctx_max(&client, &base).await.unwrap_or(None);

    let sessions_root = sessions_root_path(&args.sessions_root)?;
    let session = ensure_session_layout(&sessions_root)?;
    set_trace_log_path(Some(session.root.join("trace_debug.log")));

    // Workspace intel unit: gather real facts about where we are and inject them
    // into Elma's context so she doesn't hallucinate access constraints.
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    if !ws.is_empty() {
        let p = session.root.join("workspace.txt");
        std::fs::write(&p, ws.trim().to_string() + "\n")
            .with_context(|| format!("write {}", p.display()))?;
        trace(&args, &format!("workspace_context_saved={}", p.display()));
    }
    if !ws_brief.is_empty() {
        let p = session.root.join("workspace_brief.txt");
        std::fs::write(&p, ws_brief.trim().to_string() + "\n")
            .with_context(|| format!("write {}", p.display()))?;
        trace(&args, &format!("workspace_brief_saved={}", p.display()));
    }
    trace(
        &args,
        &format!("base_url_source={base_url_source} value={base_url}"),
    );

    let mut system_content = elma_cfg.system_prompt.clone();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: system_content.clone(),
    }];

    eprintln!("Connected target: {chat_url}");
    eprintln!("Model: {model_id}");
    eprintln!("Config: {}", model_cfg_dir.display());
    eprintln!("Session: {}", session.root.display());
    eprintln!("Type /exit to quit, /reset to clear history.\n");
    // No explicit slash workflows for now; formulas should be orchestrated automatically.

    loop {
        let Some(line) = prompt_line("you> ")? else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/exit" || line == "/quit" {
            break;
        }
        if line == "/reset" {
            messages.truncate(1); // keep system
            eprintln!("(history reset)");
            continue;
        }

        // Explicit slash workflows removed; formulas are executed automatically.

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: line.to_string(),
        });

        let route_decision = infer_route_prior(
            &client,
            &chat_url,
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &router_cal,
            line,
            &ws,
            &ws_brief,
            &messages,
        )
        .await?;
        trace(
            &args,
            &format!(
                "speech_act_dist={}",
                format_route_distribution(&route_decision.speech_act.distribution)
            ),
        );
        trace(
            &args,
            &format!(
                "speech_act={} p={:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.speech_act.choice,
                route_decision
                    .speech_act
                    .distribution
                    .first()
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0),
                route_decision.speech_act.margin,
                route_decision.speech_act.entropy,
                route_decision.speech_act.source
            ),
        );
        trace(
            &args,
            &format!(
                "workflow_dist={}",
                format_route_distribution(&route_decision.workflow.distribution)
            ),
        );
        trace(
            &args,
            &format!(
                "workflow={} p={:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.workflow.choice,
                route_decision
                    .workflow
                    .distribution
                    .first()
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0),
                route_decision.workflow.margin,
                route_decision.workflow.entropy,
                route_decision.workflow.source
            ),
        );
        trace(
            &args,
            &format!(
                "mode_dist={}",
                format_route_distribution(&route_decision.mode.distribution)
            ),
        );
        trace(
            &args,
            &format!(
                "mode={} p={:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.mode.choice,
                route_decision
                    .mode
                    .distribution
                    .first()
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0),
                route_decision.mode.margin,
                route_decision.mode.entropy,
                route_decision.mode.source
            ),
        );
        trace(
            &args,
            &format!(
                "route_dist={}",
                format_route_distribution(&route_decision.distribution)
            ),
        );
        let route_p = route_decision
            .distribution
            .first()
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        trace(
            &args,
            &format!(
                "route={} p={route_p:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.route,
                route_decision.margin,
                route_decision.entropy,
                route_decision.source
            ),
        );
        let complexity = assess_complexity_once(
            &client,
            &chat_url,
            &complexity_cfg,
            line,
            &route_decision,
            &ws,
            &ws_brief,
            &messages,
        )
        .await
        .unwrap_or_default();
        trace(
            &args,
            &format!(
                "complexity={} pattern={} risk={}",
                if complexity.complexity.is_empty() {
                    "UNKNOWN"
                } else {
                    &complexity.complexity
                },
                if complexity.suggested_pattern.is_empty() {
                    "unknown"
                } else {
                    &complexity.suggested_pattern
                },
                if complexity.risk.is_empty() {
                    "UNKNOWN"
                } else {
                    &complexity.risk
                }
            ),
        );
        let scope = build_scope_once(
            &client,
            &chat_url,
            &scope_builder_cfg,
            line,
            &route_decision,
            &complexity,
            &ws,
            &ws_brief,
            &messages,
        )
        .await
        .unwrap_or_default();
        if !scope.reason.trim().is_empty() || !scope.focus_paths.is_empty() {
            operator_trace(
                &args,
                &format!(
                    "narrowing the scope{}",
                    if scope.focus_paths.is_empty() {
                        String::new()
                    } else {
                        format!(" to {}", scope.focus_paths.join(", "))
                    }
                ),
            );
        }
        trace(
            &args,
            &format!(
                "scope focus={} include={} exclude={} query={} reason={}",
                if scope.focus_paths.is_empty() {
                    "-".to_string()
                } else {
                    scope.focus_paths.join(",")
                },
                if scope.include_globs.is_empty() {
                    "-".to_string()
                } else {
                    scope.include_globs.join(",")
                },
                if scope.exclude_globs.is_empty() {
                    "-".to_string()
                } else {
                    scope.exclude_globs.join(",")
                },
                if scope.query_terms.is_empty() {
                    "-".to_string()
                } else {
                    scope.query_terms.join(",")
                },
                scope.reason
            ),
        );
        let memories = load_recent_formula_memories(&model_cfg_dir, 8).unwrap_or_default();
        let formula = select_formula_once(
            &client,
            &chat_url,
            &formula_cfg,
            line,
            &route_decision,
            &complexity,
            &scope,
            &memories,
            &messages,
        )
        .await
        .unwrap_or_default();
        trace(
            &args,
            &format!(
                "formula={} alt={} reason={}",
                if formula.primary.is_empty() {
                    "unknown"
                } else {
                    &formula.primary
                },
                if formula.alternatives.is_empty() {
                    "-".to_string()
                } else {
                    formula.alternatives.join(",")
                },
                if formula.memory_id.trim().is_empty() {
                    formula.reason.clone()
                } else {
                    format!("{} memory={}", formula.reason, formula.memory_id)
                }
            ),
        );
        operator_trace(
            &args,
            &describe_operator_intent(&route_decision, &complexity, &formula),
        );

        let mut program = match orchestrate_program_once(
            &client,
            &chat_url,
            &orchestrator_cfg,
            line,
            &route_decision,
            &complexity,
            &scope,
            &formula,
            &ws,
            &ws_brief,
            &messages,
        )
        .await
        {
            Ok((p, _raw)) => p,
            Err(e) => {
                trace(&args, &format!("orchestrator_repair_parse_error={e}"));
                Program {
                    objective: "fallback_chat".to_string(),
                    steps: vec![Step::Reply {
                        id: "r1".to_string(),
                        instructions: "Reply to the user in plain terminal text. Do not invent workspace facts you did not inspect.".to_string(),
                        common: StepCommon::default(),
                    }],
                }
            }
        };
        if apply_capability_guard(&mut program, &route_decision) {
            trace(&args, "guard=capability_reply_only");
        }

        let workdir = repo_root()?;
        let (mut step_results, mut final_reply) = execute_program(
            &args,
            &client,
            &chat_url,
            &session,
            &workdir,
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
            false,
        )
        .await?;

        // Critic repair loop (1 retry max).
        let mut replied = false;
        for attempt in 0..=1u32 {
            if replied {
                break;
            }
            let verdict: CriticVerdict = match run_critic_once(
                &client,
                &chat_url,
                &critic_cfg,
                line,
                &route_decision,
                &program,
                &step_results,
                attempt,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    trace(&args, &format!("critic_parse_error={e}"));
                    CriticVerdict {
                        status: "ok".to_string(),
                        reason: "critic_parse_error".to_string(),
                        program: None,
                    }
                }
            };
            trace(
                &args,
                &format!("critic_status={} reason={}", verdict.status, verdict.reason),
            );

            if verdict.status.eq_ignore_ascii_case("retry") {
                if let Some(p) = verdict.program {
                    program = p;
                    if apply_capability_guard(&mut program, &route_decision) {
                        trace(&args, "guard=capability_reply_only_retry");
                    }
                    let (retry_results, retry_reply) = execute_program(
                        &args,
                        &client,
                        &chat_url,
                        &session,
                        &workdir,
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
                        false,
                    )
                    .await?;
                    step_results.extend(retry_results);
                    if retry_reply.is_some() {
                        final_reply = retry_reply;
                    }
                    continue;
                }
            }

            // Produce final response via Elma using tool outputs.
            let reply_instructions = final_reply.clone().unwrap_or_else(|| {
                "Respond to the user in plain terminal text. Use any step outputs as evidence."
                    .to_string()
            });
            let (final_text, final_usage_total) = match generate_final_answer_once(
                &client,
                &chat_url,
                &elma_cfg,
                &result_presenter_cfg,
                &claim_checker_cfg,
                &formatter_cfg,
                &system_content,
                line,
                &route_decision,
                &step_results,
                &reply_instructions,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    trace(&args, &format!("reply_generation_error={e}"));
                    (
                        "I ran into a reply-generation error after executing the workflow."
                            .to_string(),
                        None,
                    )
                }
            };
            println!(
                "{}",
                if args.no_color {
                    format!("bot> {final_text}")
                } else {
                    ansi_orange(&format!("bot> {final_text}"))
                }
            );

            if let Some(ctx) = ctx_max {
                if let Some(total) = final_usage_total {
                    let pct = (total as f64 / ctx as f64) * 100.0;
                    let used_k = {
                        let k = ((total as f64) / 1000.0).round() as u64;
                        if total > 0 {
                            k.max(1)
                        } else {
                            0
                        }
                    };
                    let ctx_k = ((ctx as f64) / 1000.0).round() as u64;
                    let line = format!("ctx: {used_k}k/{ctx_k}k [{pct:.1}%]");
                    println!(
                        "{}",
                        if args.no_color {
                            line
                        } else {
                            ansi_pale_yellow(&line)
                        }
                    );
                }
            }
            println!();

            if !final_text.is_empty() {
                if step_results.iter().all(|r| r.ok)
                    && !route_decision.route.eq_ignore_ascii_case("CHAT")
                    && formula.memory_id.trim().is_empty()
                {
                    let now = now_unix_s()?;
                    let record = FormulaMemoryRecord {
                        id: format!("fm_{now}"),
                        created_unix_s: now,
                        user_message: line.to_string(),
                        route: route_decision.route.clone(),
                        complexity: complexity.complexity.clone(),
                        formula: if formula.primary.trim().is_empty() {
                            complexity.suggested_pattern.clone()
                        } else {
                            formula.primary.clone()
                        },
                        objective: program.objective.clone(),
                        title: if !scope.objective.trim().is_empty() {
                            scope.objective.clone()
                        } else {
                            line.to_string()
                        },
                        program_signature: program_signature(&program),
                    };
                    if let Ok(path) = save_formula_memory(&model_cfg_dir, &record) {
                        trace(&args, &format!("formula_memory_saved={}", path.display()));
                    }
                }
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: final_text,
                });
            }
            replied = true;
        }

        continue;
    }

    Ok(())
}
