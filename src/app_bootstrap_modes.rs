//! @efficiency-role: util-pure
//!
//! App Bootstrap - Mode Handling and Banners

use crate::ui_state::{
    set_final_answer_extractor_profile, set_json_outputter_profile, set_model_behavior_profile,
};
use crate::*;

pub(crate) fn validate_mode_flags(args: &Args) -> Result<()> {
    let mode_flags = [
        args.tune,
        args.calibrate,
        args.restore_base,
        args.restore_last,
    ];
    if mode_flags.into_iter().filter(|v| *v).count() > 1 {
        return Err(crate::diagnostics::ElmaDiagnostic::InvalidModeCombination.into());
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
        tracing::info!(
            "Restored baseline profiles for {} from {}",
            model_id,
            baseline_dir.display()
        );
        return Ok(true);
    }

    if args.restore_last {
        let fallback_dir = model_fallback_last_active_dir(model_cfg_dir);
        if !fallback_dir.exists() {
            return Err(
                crate::diagnostics::ElmaDiagnostic::ProfileSnapshotNotFound {
                    model_id: model_id.to_string(),
                    path: fallback_dir.display().to_string(),
                }
                .into(),
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
        tracing::info!(
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
            tracing::info!(
                "Activated tuned profiles for {} with score {:.3} (certified: {}).",
                mid,
                winner.score,
                winner.report.summary.certified
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
    model_id: &str,
    _model_cfg_dir: &Path,
    _session: &SessionPaths,
    _tuned: bool,
) {
    display_logo_splash();
    eprintln!("elma · model: {}", model_id);
}

/// Display the ELMA logo splash using jp2a with a 2-second timeout.
/// Output is centered on the terminal.
fn display_logo_splash() {
    let logo_path = Path::new("logo/elma_square.png");
    if !logo_path.exists() {
        return;
    }
    if which::which("jp2a").is_err() {
        return;
    }

    let (term_w, _) = match term_size() {
        Some(dims) => dims,
        None => return,
    };

    match std::process::Command::new("jp2a")
        .args(["--color", "--height=47"])
        .arg(logo_path)
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                return;
            }
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() || lines.len() > 100 {
                return;
            }

            // Print a blank line, then centered logo, then blank line
            eprintln!();
            for line in &lines {
                let stripped = strip_ansi_escapes::strip(line.as_bytes())
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .unwrap_or_else(|| line.to_string());
                let visible_len = stripped.chars().count();
                let padding = term_w.saturating_sub(visible_len) / 2;
                eprintln!("{:>padding$}{}", "", line);
            }
            eprintln!();

            // Brief pause so user can see it (non-blocking, just sleeps)
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        Err(_) => {}
    }
}

/// Get terminal dimensions as (width, height) in characters.
fn term_size() -> Option<(usize, usize)> {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return None;
    }
    crossterm::terminal::size().ok().map(|(w, h)| (w as usize, h as usize))
}
