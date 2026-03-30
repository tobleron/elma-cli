//! @efficiency-role: orchestrator
//!
//! App Bootstrap - Mode Handling and Banners

use crate::*;

pub(crate) fn validate_mode_flags(args: &Args) -> Result<()> {
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

pub(crate) fn should_auto_tune_on_startup(args: &Args, model_cfg_dir: &Path) -> bool {
    if args.tune || args.calibrate || args.restore_base || args.restore_last {
        return false;
    }
    !model_active_manifest_path(model_cfg_dir).exists()
}

pub(crate) fn emit_auto_tune_banner(args: &Args, model_id: &str, model_cfg_dir: &Path) {
    let tune_mode = if args.tune_mode == "quick" { "quick (5 scenarios)" } else { "full" };
    if args.no_color {
        eprintln!("First-time model setup");
        eprintln!("  model    {}", model_id);
        eprintln!("  config   {}", model_cfg_dir.display());
        eprintln!("  action   {} tuning before chat startup", tune_mode);
    } else {
        eprintln!("{}", ansi_orange("First-time model setup"));
        eprintln!("{} {}", ansi_grey("  model   "), model_id);
        eprintln!("{} {}", ansi_grey("  config  "), model_cfg_dir.display());
        eprintln!("{} {} {}", ansi_grey("  action  "), tune_mode, "tuning before chat startup");
    }
}

pub(crate) async fn handle_special_modes(
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
            "manual_restore_base",
            0.0,
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
            "manual_restore_last",
            0.0,
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
        let behavior =
            ensure_model_behavior_profile(client, chat_url, base_url, &dir, &mid).await?;
        set_model_behavior_profile(Some(behavior));
        set_json_outputter_profile(Some(load_agent_config(&dir.join("json_outputter.toml"))?));
        set_final_answer_extractor_profile(Some(load_agent_config(
            &dir.join("final_answer_extractor.toml"),
        )?));
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

pub(crate) fn emit_startup_banner(
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
        eprintln!("  commands /exit  /reset  /snapshot  /rollback <id>  /tune\n");
        return;
    }

    eprintln!("{}", ansi_orange("Elma"));
    eprintln!("{} {target}", ansi_grey("  target  "));
    eprintln!("{} {model_id}", ansi_grey("  model   "));
    eprintln!("{} {}", ansi_grey("  config  "), model_cfg_dir.display());
    eprintln!("{} {session_name}", ansi_grey("  session "));
    eprintln!(
        "{} /exit  /reset  /snapshot  /rollback <id>  /tune\n",
        ansi_grey("  commands")
    );
}
