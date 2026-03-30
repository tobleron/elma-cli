//! @efficiency-role: orchestrator
//!
//! App Chat - Command Handlers

use crate::app::AppRuntime;
use crate::app_chat_helpers::refresh_runtime_workspace;
use crate::*;

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
            eprintln!("Tool discovery failed: {}", error);
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
            print_elma_message(
                &runtime.args,
                &format!("Snapshot failed: {error}"),
            );
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
            print_elma_message(
                &runtime.args,
                &format!("Rollback failed: {error}"),
            );
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
            result.snapshot_id,
            result.restored_files,
            result.removed_files,
            result.verified_files
        ),
    );
    println!();
    Ok(())
}

pub(crate) async fn handle_runtime_tune(runtime: &mut AppRuntime) -> Result<()> {
    operator_trace(
        &runtime.args,
        &format!("tuning {} and activating the best profile set", runtime.model_id),
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
    set_final_answer_extractor_profile(Some(
        runtime.profiles.final_answer_extractor_cfg.clone(),
    ));
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
