//! @efficiency-role: orchestrator
//!
//! App Bootstrap - Core Bootstrap Function

use crate::app::{AppRuntime, LoadedProfiles};
use crate::app_bootstrap_modes::*;
use crate::app_bootstrap_profiles::*;
use crate::dirs::ElmaPaths;
use crate::ui_state::{
    set_final_answer_extractor_profile, set_json_outputter_profile, set_model_behavior_profile,
    set_reasoning_display, set_trace_log_path,
};
use crate::ui_theme::*;
use crate::ui_trace::trace;
use crate::*;

pub(crate) async fn bootstrap_app(args: Args) -> Result<Option<AppRuntime>> {
    set_reasoning_display(args.show_thinking && args.debug_trace, args.no_color);
    if args.show_thinking {
        crate::set_show_reasoning(true);
    }
    validate_mode_flags(&args)?;

    if let Some(paths) = ElmaPaths::new() {
        paths.ensure_dirs()?;
    }

    let cfg_root = config_root_path(&args.config_root)?;
    let llm_runtime_cfg = load_or_create_runtime_llm_config(&cfg_root)?;
    set_runtime_llm_config(llm_runtime_cfg.clone());
    let (base_url, base_url_source) =
        resolve_base_url(&cfg_root, args.base_url.as_deref(), args.model.as_deref())?;

    // Persist to elma.toml (primary config) and global.toml (legacy)
    if base_url_source == "cli_or_env" {
        let elma_path = elma_config_path()?;
        let elma_cfg = ElmaProjectConfig {
            base_url: base_url.clone(),
            model: args.model.clone().unwrap_or_default(),
        };
        let s = toml::to_string_pretty(&elma_cfg).context("Failed to serialize elma.toml")?;
        std::fs::write(&elma_path, s.as_bytes()).context("Failed to write elma.toml")?;
    }

    save_global_config(
        &global_config_path(&cfg_root),
        &GlobalConfig {
            version: 1,
            base_url: base_url.clone(),
        },
    )?;

    let base = Url::parse(&base_url).context("Invalid --base-url")?;
    let chat_url = base
        .join("/v1/chat/completions")
        .context("Failed to build /v1/chat/completions URL")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(llm_runtime_cfg.http_timeout_s))
        .build()
        .context("Failed to build HTTP client")?;

    let model_id = if let Some(model) = args.model.as_ref().filter(|s| !s.trim().is_empty()) {
        model.trim().to_string()
    } else {
        fetch_first_model_id(&client, &base).await?
    };
    let model_cfg_dir = ensure_model_config_folder(&cfg_root, &base_url, &model_id)?;

    // Set config root for grammar injection
    crate::ui_chat::set_config_root(cfg_root.clone());

    // Ensure default configs exist (create if missing)
    // ensure_default_configs(&model_cfg_dir, &base_url, &model_id)?;  // Deprecated

    let behavior =
        ensure_model_behavior_profile(&client, &chat_url, &base_url, &model_cfg_dir, &model_id)
            .await?;
    set_model_behavior_profile(Some(behavior.clone()));

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

    let is_tuned = !should_auto_tune_on_startup(&args, &model_cfg_dir);

    if !is_tuned {
        // Need to tune
        let tune_mode = if args.tune_mode == "quick" {
            "quick (5 scenarios)"
        } else {
            "full"
        };
        if args.no_color {
            eprintln!("First-time model setup");
            eprintln!("  model    {}", model_id);
            eprintln!("  config   {}", model_cfg_dir.display());
            eprintln!("  action   {} tuning before chat startup", tune_mode);
        } else {
            eprintln!("{}", warn_yellow("First-time model setup"));
            eprintln!("{} {}", meta_comment("  model   "), model_id);
            eprintln!("{} {}", meta_comment("  config  "), model_cfg_dir.display());
            eprintln!(
                "{} {} {}",
                meta_comment("  action  "),
                tune_mode,
                "tuning before chat startup"
            );
        }

        let mut auto_tune_args = args.clone();
        auto_tune_args.tune = true;
        match optimize_model(
            &auto_tune_args,
            &client,
            &chat_url,
            &base_url,
            &model_cfg_dir,
            &model_id,
        )
        .await
        {
            Ok(winner) => {
                eprintln!(
                    "  tuned    score {:.3}  certified {}",
                    winner.score, winner.report.summary.certified
                );
            }
            Err(error) => {
                eprintln!("  tuned    failed");
                eprintln!("  reason   {error:#}");
                eprintln!("  action   continuing with baseline profiles");
            }
        }
    }

    let mut profiles = load_profiles(&model_cfg_dir)?;
    sync_and_upgrade_profiles(&args, &model_cfg_dir, &base_url, &model_id, &mut profiles)?;
    set_json_outputter_profile(Some(profiles.json_outputter_cfg.clone()));
    set_final_answer_extractor_profile(Some(profiles.final_answer_extractor_cfg.clone()));

    // Task 046: Check if intel unit prompts have changed since last tuning
    // TEMPORARILY DISABLED for stress testing - causes issues
    /*
    if is_tuned {
        let current_hashes = crate::tune::compute_all_prompt_hashes(&profiles);
        match crate::tune::check_prompt_changes(&model_cfg_dir, &current_hashes) {
            Ok((changed, units)) => {
                if changed {
                    // Prompts changed, need to re-tune
                    if args.no_color {
                        eprintln!("Prompt changes detected");
                        eprintln!("  changed units: {}", units.join(", "));
                        eprintln!("  action: auto-tuning to update profiles");
                    } else {
                        eprintln!("{}", warn_yellow("Prompt changes detected"));
                        eprintln!("{} {}", meta_comment("  changed units:"), units.join(", "));
                        eprintln!("{} auto-tuning to update profiles", meta_comment("  action: "));
                    }

                    // Trigger auto-tune
                    let mut auto_tune_args = args.clone();
                    auto_tune_args.tune = true;
                    match optimize_model(
                        &auto_tune_args,
                        &client,
                        &chat_url,
                        &base_url,
                        &model_cfg_dir,
                        &model_id,
                    )
                    .await {
                        Ok(winner) => {
                            eprintln!(
                                "  tuned    score {:.3}  certified {}",
                                winner.score, winner.report.summary.certified
                            );
                            // Reload profiles with new tuning
                            profiles = load_profiles(&model_cfg_dir)?;
                            sync_and_upgrade_profiles(&args, &model_cfg_dir, &base_url, &model_id, &mut profiles)?;
                        }
                        Err(error) => {
                            eprintln!("  tuned    failed");
                            eprintln!("  reason   {error:#}");
                            eprintln!("  action   continuing with existing profiles");
                        }
                    }
                }
            }
            Err(error) => {
                trace(&args, &format!("prompt_change_check_failed error={}", error));
            }
        }
    }
    */

    let ctx_max = fetch_ctx_max(&client, &base).await.unwrap_or(None);
    let session = prepare_session(&args)?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    let guidance = load_project_guidance(&repo);
    persist_workspace_intel(&args, &session, &ws, &ws_brief)?;
    persist_guidance_snapshot(&args, &session, &guidance)?;
    trace(
        &args,
        &format!("base_url_source={base_url_source} value={base_url}"),
    );
    trace(
        &args,
        &format!(
            "model_behavior preferred_reasoning={} auto_separated={} auto_truncated={} finalizer={} none_clean={} json_auto={} json_none={}",
            behavior.preferred_reasoning_format,
            behavior.auto_reasoning_separated,
            behavior.auto_truncated_before_final,
            behavior.needs_text_finalizer,
            behavior.none_final_clean,
            behavior.json_clean_with_auto,
            behavior.json_clean_with_none
        ),
    );

    let system_content = build_system_content(
        &profiles.elma_cfg.system_prompt,
        &ws,
        &ws_brief,
        &guidance,
        &model_id,
        chat_url.as_str(),
    );
    let messages = vec![ChatMessage::simple("system", &system_content.clone())];

    let goal_state = load_goal_state(&session.root).unwrap_or_default();
    let active_runtime_task = load_latest_runtime_task(&session.root);
    if goal_state.has_active_goal() {
        trace(
            &args,
            &format!(
                "loaded_goal_state objective={:?}",
                goal_state.active_objective
            ),
        );
    }

    emit_startup_banner(
        &args,
        &chat_url,
        &model_id,
        &model_cfg_dir,
        &session,
        is_tuned,
    );

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
        guidance,
        system_content,
        messages,
        profiles,
        goal_state,
        execution_plan: ExecutionPlanSelection::simple_general(),
        active_runtime_task,
        last_stop_outcome: None,
        verbose: false,
        retry_attempt: 0,
        tool_registry: tool_discovery::ToolRegistry::new(),
    }))
}

fn prepare_session(args: &Args) -> Result<SessionPaths> {
    let sessions_root = sessions_root_path(&args.sessions_root)?;
    let session = ensure_session_layout(&sessions_root)?;
    set_trace_log_path(Some(session.root.join("trace_debug.log")));

    install_panic_hook(Some(session.root.clone()));

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

fn build_system_content(
    base_prompt: &str,
    ws: &str,
    ws_brief: &str,
    guidance: &GuidanceSnapshot,
    model_id: &str,
    base_url: &str,
) -> String {
    let mut system_content = base_prompt.to_string();
    if !model_id.trim().is_empty() || !base_url.trim().is_empty() {
        system_content.push_str("\n\nRUNTIME CONTEXT:\n");
        if !model_id.trim().is_empty() {
            system_content.push_str(&format!("model_id: {}\n", model_id.trim()));
        }
        if !base_url.trim().is_empty() {
            system_content.push_str(&format!("base_url: {}\n", base_url.trim()));
        }
    }
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let guidance_text = guidance.render_for_system_prompt();
    if !guidance_text.is_empty() {
        system_content.push_str("\n\nPROJECT GUIDANCE:\n");
        system_content.push_str(&guidance_text);
    }
    system_content
}
