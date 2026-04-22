//! @efficiency-role: util-pure
//!
//! App Bootstrap - Mode Handling and Banners

use crate::*;
use crate::ui_state::{set_final_answer_extractor_profile, set_json_outputter_profile, set_model_behavior_profile};

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
    // Explicit tune/calibrate flags always trigger tuning
    if args.tune || args.calibrate || args.restore_base || args.restore_last {
        return false;
    }

    // DISABLED: Auto-tuning on startup
    // Elma now uses global defaults from config/defaults/ without per-model tuning
    // To enable tuning, use --tune flag explicitly
    false
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
            tune_model(
                args, client, chat_url, base_url, &dir, &mid, &tune_cfg, true,
            )
            .await?;
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

/// Emit startup banner to stderr.
///
/// Note: In interactive chat mode, this info is displayed in the header strip
/// instead. This function is kept for non-interactive / scripted modes.
pub(crate) fn emit_startup_banner(
    _args: &Args,
    _chat_url: &Url,
    _model_id: &str,
    _model_cfg_dir: &Path,
    _session: &SessionPaths,
    _tuned: bool,
) {
    // Banner migrated to header strip in interactive mode.
    // Non-interactive paths may call this, but we suppress output to avoid
    // duplicating what the header already shows.
}
