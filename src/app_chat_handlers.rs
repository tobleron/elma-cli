//! @efficiency-role: orchestrator
//!
//! App Chat - Command Handlers

use crate::app::AppRuntime;
// use crate::app_bootstrap_profiles::save_all_profiles;  // Deprecated
use crate::app_chat_helpers::refresh_runtime_workspace;
use crate::*;

/// Handle /provider command - interactive endpoint configuration.
pub(crate) async fn handle_provider_config(runtime: &mut AppRuntime) -> Result<()> {
    use crate::ui_interact::prompt_text;
    use std::io::IsTerminal;

    if !std::io::stderr().is_terminal() {
        eprintln!("Error: /provider requires interactive terminal");
        return Ok(());
    }

    println!();
    println!("=== Provider Configuration ===");
    println!();
    println!("Current endpoint: {}", runtime.profiles.elma_cfg.base_url);
    println!("Detected model:   {}", runtime.model_id);
    println!(
        "Context window:   {}",
        runtime
            .ctx_max
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!();

    let base_url = match prompt_text("Enter endpoint URL (or press Enter to keep current):") {
        Some(url) if !url.is_empty() => url,
        Some(_) => runtime.profiles.elma_cfg.base_url.clone(),
        None => {
            println!("(cancelled)");
            return Ok(());
        }
    };

    let helper_url = match prompt_text("Optional helper endpoint URL (Enter to disable/skip):") {
        Some(url) => url,
        None => {
            println!("(cancelled)");
            return Ok(());
        }
    };

    handle_api_config(runtime, &base_url).await?;
    if !helper_url.trim().is_empty() {
        configure_auxiliary_endpoint(runtime, helper_url.trim()).await?;
    }

    Ok(())
}

/// Handle /api command - configure endpoint and model settings
pub(crate) async fn handle_api_config(runtime: &mut AppRuntime, args: &str) -> Result<()> {
    let args_trimmed = args.trim();

    if args_trimmed.is_empty() {
        // Show current config
        println!("\n=== Current API Configuration ===");
        println!("Endpoint: {}", runtime.profiles.elma_cfg.base_url);
        println!("Model:    {}", runtime.model_id);
        println!();
        println!("Usage: /api <endpoint_url> [model_id]");
        println!("  /api http://localhost:8080/v1");
        println!("  /api http://localhost:8080/v1 llama-3.2-3b-instruct");
        println!("If model_id is omitted, Elma discovers it from /v1/models.");
        println!();
        return Ok(());
    }

    // Parse arguments
    let mut parts = args_trimmed.split_whitespace();
    let new_base_url = parts.next().unwrap_or("http://localhost:8080/v1");
    let new_model_id = parts.next();

    // Validate URL
    if !new_base_url.starts_with("http://") && !new_base_url.starts_with("https://") {
        eprintln!("Error: Invalid URL. Must start with http:// or https://");
        return Ok(());
    }

    let base = Url::parse(new_base_url).context("Invalid base URL")?;
    let endpoint_profile = probe_endpoint_runtime(&runtime.client, &base)
        .await
        .context("Model endpoint health probe failed")?;
    let model_id = new_model_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| endpoint_profile.model_id.clone());

    runtime.chat_url = base
        .join("/v1/chat/completions")
        .context("Failed to build chat URL")?;
    runtime.ctx_max = endpoint_profile.ctx_max;
    runtime.model_id = model_id;

    let cfg_root = config_root_path(&runtime.args.config_root)?;
    runtime.model_cfg_dir = ensure_model_config_folder(&cfg_root, new_base_url, &runtime.model_id)?;
    runtime.profiles = crate::app_bootstrap::load_profiles(&runtime.model_cfg_dir)?;
    crate::app_bootstrap::sync_and_upgrade_profiles(
        &runtime.args,
        &runtime.model_cfg_dir,
        new_base_url,
        &runtime.model_id,
        &mut runtime.profiles,
    )?;
    set_json_outputter_profile(Some(runtime.profiles.json_outputter_cfg.clone()));
    set_final_answer_extractor_profile(Some(runtime.profiles.final_answer_extractor_cfg.clone()));

    if let Ok(elma_path) = elma_config_path() {
        let cfg = ElmaProjectConfig {
            base_url: new_base_url.to_string(),
            model: String::new(),
        };
        if let Ok(s) = toml::to_string_pretty(&cfg) {
            let _ = std::fs::write(&elma_path, s.as_bytes());
        }
    }

    // Persist updated provider profile
    let provider = crate::llm_provider::LlmProvider::detect(new_base_url, Some(&runtime.model_id));
    let provider_profile = crate::models_api::ProviderProfile::from_endpoint(
        new_base_url,
        &endpoint_profile,
        &provider.to_string(),
        &crate::model_capability_probe::ModelRuntimeKind::Unknown,
        false,
        false,
        "reconfig",
    );
    let _ = crate::models_api::save_provider_profile(&runtime.model_cfg_dir, &provider_profile);

    println!("\nAPI configuration updated");
    println!("  Endpoint: {}", new_base_url);
    println!("  Model:    {} (detected)", runtime.model_id);
    println!(
        "  Context:  {}",
        runtime
            .ctx_max
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("  Config:   {}", runtime.model_cfg_dir.display());
    println!();

    Ok(())
}

async fn configure_auxiliary_endpoint(runtime: &AppRuntime, helper_url: &str) -> Result<()> {
    let base = Url::parse(helper_url).context("Invalid helper endpoint URL")?;
    let endpoint_profile = probe_endpoint_runtime(&runtime.client, &base)
        .await
        .context("Helper endpoint health probe failed")?;
    let cfg_root = config_root_path(&runtime.args.config_root)?;
    let mut cfg = load_or_create_runtime_llm_config(&cfg_root)?;
    cfg.auxiliary_enabled = true;
    cfg.auxiliary_base_url = helper_url.to_string();
    cfg.auxiliary_model = endpoint_profile.model_id;
    save_runtime_llm_config(&cfg_root, &cfg)?;
    println!("  Helper:   {} ({})", helper_url, cfg.auxiliary_model);
    Ok(())
}

/// Show current goal state (Task 014: Multi-Turn Goal Persistence)
pub(crate) fn handle_show_goals(runtime: &AppRuntime) -> Result<()> {
    if !runtime.goal_state.has_active_goal() {
        eprintln!("No active goal. Start by giving me a task!");
        return Ok(());
    }

    println!("\n=== Current Goal ===");
    if let Some(ref objective) = runtime.goal_state.active_objective {
        println!("Objective: {}", objective);
    }

    if !runtime.goal_state.completed_subgoals.is_empty() {
        println!("\nCompleted:");
        for subgoal in &runtime.goal_state.completed_subgoals {
            println!("  ✓ {}", subgoal);
        }
    }

    if !runtime.goal_state.pending_subgoals.is_empty() {
        println!("\nPending:");
        for subgoal in &runtime.goal_state.pending_subgoals {
            println!("  ○ {}", subgoal);
        }
    }

    if let Some(ref reason) = runtime.goal_state.blocked_reason {
        println!("\n⚠ Blocked: {}", reason);
    }

    println!();
    Ok(())
}

/// Discover and show available tools (Task 015: Autonomous Tool Discovery)
pub(crate) fn handle_discover_tools(runtime: &AppRuntime) -> Result<()> {
    println!("\nDiscovering workspace tools...");

    match tool_discovery::discover_workspace_tools(&runtime.repo) {
        Ok(registry) => {
            println!("{}", registry.format_for_display());
            println!("(tools cached for this session)");
        }
        Err(error) => {
            tracing::error!("Tool discovery failed: {}", error);
        }
    }

    Ok(())
}

pub(crate) fn handle_manual_snapshot(runtime: &mut AppRuntime) -> Result<()> {
    operator_trace(&runtime.args, "creating a recovery snapshot");
    let snapshot = match create_workspace_snapshot(
        &runtime.session,
        &runtime.repo,
        "manual snapshot",
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            print_elma_message(&runtime.args, &format!("Snapshot failed: {error}"));
            println!();
            return Ok(());
        }
    };
    trace(
        &runtime.args,
        &format!(
            "snapshot_saved id={} path={} files={} automatic={}",
            snapshot.snapshot_id,
            snapshot.snapshot_dir.display(),
            snapshot.file_count,
            snapshot.automatic
        ),
    );
    print_elma_message(
        &runtime.args,
        &format!(
            "Created snapshot {} with {} files. Manifest: {}",
            snapshot.snapshot_id,
            snapshot.file_count,
            snapshot.manifest_path.display()
        ),
    );
    println!();
    Ok(())
}

pub(crate) fn handle_manual_rollback(runtime: &mut AppRuntime, snapshot_id: &str) -> Result<()> {
    let snapshot_id = snapshot_id.trim();
    if snapshot_id.is_empty() {
        print_elma_message(&runtime.args, "Usage: /rollback <snapshot_id>");
        println!();
        return Ok(());
    }
    operator_trace(
        &runtime.args,
        &format!("rolling back to snapshot {}", snapshot_id),
    );
    let result = match rollback_workspace_snapshot(&runtime.session, &runtime.repo, snapshot_id) {
        Ok(result) => result,
        Err(error) => {
            print_elma_message(&runtime.args, &format!("Rollback failed: {error}"));
            println!();
            return Ok(());
        }
    };
    trace(
        &runtime.args,
        &format!(
            "rollback_completed id={} restored={} removed={} verified={} manifest={}",
            result.snapshot_id,
            result.restored_files,
            result.removed_files,
            result.verified_files,
            result.manifest_path.display()
        ),
    );
    refresh_runtime_workspace(runtime)?;
    print_elma_message(
        &runtime.args,
        &format!(
            "Rolled back to {}. Restored {} files, removed {} files, verified {} files.",
            result.snapshot_id, result.restored_files, result.removed_files, result.verified_files
        ),
    );
    println!();
    Ok(())
}

pub(crate) async fn handle_runtime_tune(runtime: &mut AppRuntime) -> Result<()> {
    operator_trace(
        &runtime.args,
        &format!(
            "tuning {} and activating the best profile set",
            runtime.model_id
        ),
    );
    let mut tune_args = runtime.args.clone();
    tune_args.tune = true;
    tune_args.calibrate = false;
    let winner = optimize_model(
        &tune_args,
        &runtime.client,
        &runtime.chat_url,
        &runtime.profiles.elma_cfg.base_url,
        &runtime.model_cfg_dir,
        &runtime.model_id,
    )
    .await?;

    runtime.profiles = app_bootstrap::load_profiles(&runtime.model_cfg_dir)?;
    set_json_outputter_profile(Some(runtime.profiles.json_outputter_cfg.clone()));
    set_final_answer_extractor_profile(Some(runtime.profiles.final_answer_extractor_cfg.clone()));
    refresh_runtime_workspace(runtime)?;

    print_elma_message(
        &runtime.args,
        &format!(
            "Tuning complete for {}. Activated score {:.3}. Certified: {}.",
            runtime.model_id, winner.score, winner.report.summary.certified
        ),
    );
    println!();
    Ok(())
}
