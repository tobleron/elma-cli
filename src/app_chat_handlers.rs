//! @efficiency-role: orchestrator
//!
//! App Chat - Command Handlers

use crate::app::AppRuntime;
// use crate::app_bootstrap_profiles::save_all_profiles;  // Deprecated
use crate::app_chat_helpers::refresh_runtime_workspace;
use crate::*;

/// Handle /api command - configure endpoint and model settings
pub(crate) fn handle_api_config(runtime: &mut AppRuntime, args: &str) -> Result<()> {
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

    // Update base URL in all profiles
    runtime.profiles.elma_cfg.base_url = new_base_url.to_string();
    runtime.profiles.expert_advisor_cfg.base_url = new_base_url.to_string();
    runtime.profiles.router_cfg.base_url = new_base_url.to_string();
    runtime.profiles.speech_act_cfg.base_url = new_base_url.to_string();
    runtime.profiles.mode_router_cfg.base_url = new_base_url.to_string();
    runtime.profiles.selector_cfg.base_url = new_base_url.to_string();
    runtime.profiles.complexity_cfg.base_url = new_base_url.to_string();
    runtime.profiles.evidence_need_cfg.base_url = new_base_url.to_string();
    runtime.profiles.tools_need_cfg.base_url = new_base_url.to_string();
    runtime.profiles.action_need_cfg.base_url = new_base_url.to_string();
    runtime.profiles.action_selector_cfg.base_url = new_base_url.to_string();
    runtime.profiles.action_formatter_cfg.base_url = new_base_url.to_string();
    runtime.profiles.workflow_planner_cfg.base_url = new_base_url.to_string();
    runtime.profiles.evidence_mode_cfg.base_url = new_base_url.to_string();
    runtime.profiles.command_repair_cfg.base_url = new_base_url.to_string();
    runtime.profiles.orchestrator_cfg.base_url = new_base_url.to_string();
    runtime.profiles.critic_cfg.base_url = new_base_url.to_string();
    runtime.profiles.json_outputter_cfg.base_url = new_base_url.to_string();
    runtime.profiles.result_presenter_cfg.base_url = new_base_url.to_string();
    runtime.profiles.claim_checker_cfg.base_url = new_base_url.to_string();

    // Update model ID if provided
    if let Some(model_id) = new_model_id {
        runtime.model_id = model_id.to_string();
        runtime.profiles.elma_cfg.model = model_id.to_string();
        runtime.profiles.expert_advisor_cfg.model = model_id.to_string();
        runtime.profiles.router_cfg.model = model_id.to_string();
        runtime.profiles.speech_act_cfg.model = model_id.to_string();
        runtime.profiles.mode_router_cfg.model = model_id.to_string();
        runtime.profiles.selector_cfg.model = model_id.to_string();
        runtime.profiles.complexity_cfg.model = model_id.to_string();
        runtime.profiles.evidence_need_cfg.model = model_id.to_string();
        runtime.profiles.tools_need_cfg.model = model_id.to_string();
        runtime.profiles.action_need_cfg.model = model_id.to_string();
        runtime.profiles.action_selector_cfg.model = model_id.to_string();
        runtime.profiles.action_formatter_cfg.model = model_id.to_string();
        runtime.profiles.workflow_planner_cfg.model = model_id.to_string();
        runtime.profiles.evidence_mode_cfg.model = model_id.to_string();
        runtime.profiles.command_repair_cfg.model = model_id.to_string();
        runtime.profiles.orchestrator_cfg.model = model_id.to_string();
        runtime.profiles.critic_cfg.model = model_id.to_string();
        runtime.profiles.json_outputter_cfg.model = model_id.to_string();
        runtime.profiles.result_presenter_cfg.model = model_id.to_string();
        runtime.profiles.claim_checker_cfg.model = model_id.to_string();
    }

    // Save configs to disk
    // save_all_profiles(&runtime.model_cfg_dir, &runtime.profiles)?;  // Deprecated

    // Update chat URL
    let base = Url::parse(new_base_url).context("Invalid base URL")?;
    runtime.chat_url = base
        .join("/v1/chat/completions")
        .context("Failed to build chat URL")?;

    println!("\n✓ API configuration updated");
    println!("  Endpoint: {}", new_base_url);
    println!("  Model:    {}", runtime.model_id);
    println!("  Config:   {}", runtime.model_cfg_dir.display());
    println!();

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
