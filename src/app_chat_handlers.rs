//! @efficiency-role: orchestrator
//!
//! App Chat - Command Handlers

use crate::app::AppRuntime;
// use crate::app_bootstrap_profiles::save_all_profiles;  // Deprecated
use crate::app_chat_helpers::refresh_runtime_workspace;
use crate::ui_terminal::{TerminalUI, MessageRole};
use crate::*;

/// Returns true if the command was handled (should continue loop), false if not a command.
pub(crate) async fn handle_chat_command(
    runtime: &mut AppRuntime,
    line: &str,
    tui: &mut TerminalUI,
) -> Result<bool> {
    if line.is_empty() {
        return Ok(true);
    }
    macro_rules! handled {
        () => {
            Ok(true)
        };
    }
    match line {
        "/exit" | "/quit" => Ok(false),
        "/clear" => {
            tui.set_modal(crate::ui_state::ModalState::Confirm {
                title: "Clear Transcript".to_string(),
                message: "Are you sure you want to clear the conversation transcript? This will not reset your session state.".to_string(),
            });
            handled!()
        }
        "/models" => {
            let base_url = runtime.config.profiles.elma_cfg.base_url.clone();
            let models = crate::models_api::fetch_all_model_ids(
                &runtime.config.client,
                &Url::parse(&base_url).unwrap(),
            )
            .await?;
            tui.set_modal(crate::ui_state::ModalState::ModelSelector {
                models,
                selected: 0,
                base_url,
            });
            handled!()
        }
        "/confirm-clear" => {
            runtime.state.messages.truncate(1);
            tui.clear_messages();
            tui.add_claude_message(crate::claude_ui::ClaudeMessage::System {
                content: "Conversation cleared".to_string(),
            });
            handled!()
        }
        "/sessions" | "/resume" => {
            open_session_picker(runtime, tui);
            handled!()
        }
        "/tasks" => {
            let lines = tui.todo_render_lines();
            if lines.is_empty() {
                tui.add_message(
                    MessageRole::Assistant,
                    "(no tasks yet — task list appears during multi-step work)".to_string(),
                );
            } else {
                tui.add_message(MessageRole::Assistant, lines.join("\n"));
            }
            handled!()
        }

        "/reset" => {
            tui.set_modal(crate::ui_state::ModalState::Confirm {
                title: "Reset Session".to_string(),
                message: "Are you sure you want to reset the session? This will clear history, permissions, and budgets.".to_string(),
            });
            handled!()
        }
        "/confirm-reset" => {
            runtime.state.messages.truncate(1);
            crate::permission_gate::reset_permission_cache();
            crate::command_budget::reset_budget();
            crate::shell_preflight::clear_confirmation_cache();
            tui.add_message(MessageRole::Assistant, "(history reset, permission cache cleared, command budget reset, confirmation cache cleared)".to_string());
            handled!()
        }
        "/snapshot" => {
            let snapshots = crate::snapshot::list_session_snapshots(&runtime.state.session)?;
            tui.set_modal(crate::ui_state::ModalState::SnapshotList {
                snapshots,
                selected: 0,
            });
            handled!()
        }
        "/tune" => {
            tui.set_modal(crate::ui_state::ModalState::TuneSelector {
                profiles: vec![
                    ("Balanced".to_string(), "Standard performance and cost.".to_string()),
                    ("Fast".to_string(), "Prioritize speed over complexity.".to_string()),
                    ("Creative".to_string(), "Higher temperature for exploration.".to_string()),
                    ("Precise".to_string(), "Low temperature for analytical tasks.".to_string()),
                ],
                selected: 0,
            });
            handled!()
        }
        "/goals" => {
            tui.set_modal(crate::ui_state::ModalState::GoalList {
                objective: runtime.state.goal_state.active_objective.clone(),
                completed: runtime.state.goal_state.completed_subgoals.clone(),
                pending: runtime.state.goal_state.pending_subgoals.clone(),
            });
            handled!()
        }
        "/reset-goals" => {
            runtime.state.goal_state.clear();
            tui.add_message(MessageRole::Assistant, "(goals reset)".to_string());
            handled!()
        }
        "/tools" => {
            let registry = tool_discovery::discover_workspace_tools(&runtime.workspace.repo)?;
            let tools = registry.tools.iter().map(|(name, cap)| {
                (name.clone(), cap.description.clone())
            }).collect();
            tui.set_modal(crate::ui_state::ModalState::ToolList {
                tools,
                selected: 0,
            });
            handled!()
        }
        "/verbose" => {
            runtime.tui.verbose = !runtime.tui.verbose;
            tui.add_message(
                MessageRole::Assistant,
                format!("(verbose {})", if runtime.tui.verbose { "on" } else { "off" }).to_string(),
            );
            handled!()
        }
        "/reasoning" => {
            let new_state = crate::toggle_show_reasoning();
            tui.notify(&format!(
                "Reasoning {}",
                if new_state { "ON" } else { "OFF" }
            ));
            handled!()
        }
        "/expand-thinking" => {
            let expanded = tui.claude_transcript_expanded();
            tui.set_claude_transcript_expanded(!expanded);
            tui.notify(&format!(
                "Thinking {}",
                if !expanded { "EXPANDED" } else { "COLLAPSED" }
            ));
            handled!()
        }
        "/help" => {
            use crate::ui_state::ModalState;
            let help_content = format!(
                "GLOBAL:\n\
                 Ctrl+C     Clear input / quit\n\
                 Ctrl+L     Sessions\n\
                 Ctrl+N     New session\n\
                 Ctrl+Shift+S Toggle mouse capture (scroll vs select text)\n\n\
                 CHAT:\n\
                 Enter      Send message\n\
                Ctrl+J     New line\n\
                 Tab        Cycle autocomplete\n\
                 Page Up/Dn Scroll history\n\
                 Up/Down    History / navigate\n\n\
                 INPUT:\n\
                 Ctrl+←/→   Jump word\n\
                 Ctrl+W     Delete word\n\
                 Ctrl+U     Delete to line start\n\
                 Home/End   Start / end of line\n\n\
                 THINKING:\n\
                 Ctrl+T     Expand/collapse all thinking threads\n\
                 Ctrl+O     Toggle task list\n\n\
                 SLASH COMMANDS:\n\
                 /help      Show this help\n\
                 /models    Switch model/provider\n\
                 /provider  Configure endpoint (IP/port)\n\
                 /usage     Token and cost stats\n\
                 /expand-thinking Expand all thinking\n\
                 /approve   Tool approval policy\n\
                 /compact   Compact context\n\
                 /reset     Clear history\n\
                 /snapshot  Create snapshot\n\
                 /tune      Model tuning\n\
                 /tools     Discover tools\n\
                 /verbose   Toggle verbose\n\
                 /reasoning Toggle reasoning visibility\n\
                 /exit      Quit Elma"
            );
            tui.set_modal(ModalState::Help {
                content: help_content,
            });
            handled!()
        }
        "/settings" => {
            use crate::ui_state::ModalState;
            let settings_content = format!(
                "PROVIDER: {}\n\
                 MODEL: {}\n\
                 ENDPOINT: {}\n\
                 APPROVAL: auto\n\
                 WORKSPACE: {}",
                runtime.config.model_id,
                runtime.config.model_id,
                runtime.config.chat_url,
                if runtime.workspace.ws_brief.is_empty() {
                    "."
                } else {
                    &runtime.workspace.ws_brief
                },
            );
            tui.set_modal(ModalState::Settings {
                content: settings_content,
            });
            handled!()
        }
        "/provider" => {
            let base_url = runtime.config.profiles.elma_cfg.base_url.clone();
            // Optional: load helper URL from config if it exists
            let cfg_root = config_root_path(&runtime.args.config_root)?;
            let helper_url = if let Ok(cfg) = load_or_create_runtime_llm_config(&cfg_root) {
                if cfg.auxiliary_enabled { cfg.auxiliary_base_url } else { String::new() }
            } else {
                String::new()
            };

            tui.set_modal(crate::ui_state::ModalState::ProviderConfig {
                base_url,
                helper_url,
                selected_index: 0,
            });
            handled!()
        }
        "/usage" => {
            let mut input_tokens = 0;
            let mut output_tokens = 0;
            for msg in &runtime.state.messages {
                let est = crate::token_counter::count_tokens(&msg.content) as u64;
                if msg.role == "assistant" {
                    output_tokens += est;
                } else {
                    input_tokens += est;
                }
            }

            tui.set_modal(crate::ui_state::ModalState::UsageReport {
                model: runtime.config.model_id.clone(),
                input_tokens,
                output_tokens,
                context_tokens: input_tokens + output_tokens,
                context_max: runtime.config.ctx_max.unwrap_or(0),
                cost_est: (input_tokens as f64 * 0.000003) + (output_tokens as f64 * 0.000015), // Rough estimate
            });
            handled!()
        }
        "/approve" | "/approve-refresh" => {
            let current = crate::safe_mode::get_safe_mode();
            let session_state = crate::session_state::get_session_state();
            let settings = session_state.safety_settings.lock().unwrap();
            tui.set_modal(crate::ui_state::ModalState::SafetySettings {
                approval_policy: current.display().to_string(),
                shell_preflight: settings.shell_redirection_blocked,
                command_budget: settings.max_shell_calls_per_turn,
                confirm_cache_count: crate::shell_preflight::confirmation_cache_count(),
                selected_index: 0,
            });
            handled!()
        }
        "/compact" => {
            tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactBoundary);
            tui.add_claude_message(crate::claude_ui::ClaudeMessage::CompactSummary {
                message_count: runtime.state.messages.len(),
                context_preview: Some("manual compact".to_string()),
            });
            handled!()
        }
        _ => {
            if let Some(id) = line.strip_prefix("/rollback") {
                handle_manual_rollback(runtime, id.trim())?;
                return handled!();
            }
            if let Some(a) = line.strip_prefix("/api") {
                handle_api_config(runtime, a).await?;
                return handled!();
            }
            Ok(true)
        }
    }
}

/// Open the session picker modal with current session list.
pub(crate) fn open_session_picker(runtime: &mut AppRuntime, tui: &mut TerminalUI) {
    let sessions_root = runtime
        .state
        .session
        .root
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| runtime.state.session.root.clone());
    let current_id = runtime
        .state
        .session
        .root
        .file_name()
        .map(|s| s.to_string_lossy().to_string());
    let entries =
        crate::session_browser::load_session_picker_entries(&sessions_root, current_id.as_deref());
    tui.set_modal(crate::ui_state::ModalState::SessionPicker {
        entries,
        selected: 0,
        filter: String::new(),
        error: None,
    });
}


/// Handle /api command - configure endpoint and model settings
pub(crate) async fn handle_api_config(runtime: &mut AppRuntime, args: &str) -> Result<()> {
    let args_trimmed = args.trim();

    if args_trimmed.is_empty() {
        // Show current config
        println!("\n=== Current API Configuration ===");
        println!("Endpoint: {}", runtime.config.profiles.elma_cfg.base_url);
        println!("Model:    {}", runtime.config.model_id);
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
    let endpoint_profile = probe_endpoint_runtime(&runtime.config.client, &base)
        .await
        .context("Model endpoint health probe failed")?;
    let model_id = new_model_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| endpoint_profile.model_id.clone());

    runtime.config.chat_url = base
        .join("/v1/chat/completions")
        .context("Failed to build chat URL")?;
    runtime.config.ctx_max = endpoint_profile.ctx_max;
    runtime.config.model_id = model_id;

    let cfg_root = config_root_path(&runtime.args.config_root)?;
    runtime.config.model_cfg_dir = ensure_model_config_folder(&cfg_root, new_base_url, &runtime.config.model_id)?;
    runtime.config.profiles = crate::app_bootstrap::load_profiles(&runtime.config.model_cfg_dir)?;
    crate::app_bootstrap::sync_and_upgrade_profiles(
        &runtime.args,
        &runtime.config.model_cfg_dir,
        new_base_url,
        &runtime.config.model_id,
        &mut runtime.config.profiles,
    )?;
    set_json_outputter_profile(Some(runtime.config.profiles.json_outputter_cfg.clone()));
    set_final_answer_extractor_profile(Some(runtime.config.profiles.final_answer_extractor_cfg.clone()));

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
    let provider = crate::llm_provider::LlmProvider::detect(new_base_url, Some(&runtime.config.model_id));
    let provider_profile = crate::models_api::ProviderProfile::from_endpoint(
        new_base_url,
        &endpoint_profile,
        &provider.to_string(),
        &crate::model_capability_probe::ModelRuntimeKind::Unknown,
        false,
        false,
        "reconfig",
    );
    let _ = crate::models_api::save_provider_profile(&runtime.config.model_cfg_dir, &provider_profile);

    Ok(())
}

pub(crate) async fn configure_auxiliary_endpoint(runtime: &AppRuntime, helper_url: &str) -> Result<()> {
    let base = Url::parse(helper_url).context("Invalid helper endpoint URL")?;
    let endpoint_profile = probe_endpoint_runtime(&runtime.config.client, &base)
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
    if !runtime.state.goal_state.has_active_goal() {
        eprintln!("No active goal. Start by giving me a task!");
        return Ok(());
    }

    println!("\n=== Current Goal ===");
    if let Some(ref objective) = runtime.state.goal_state.active_objective {
        println!("Objective: {}", objective);
    }

    if !runtime.state.goal_state.completed_subgoals.is_empty() {
        println!("\nCompleted:");
        for subgoal in &runtime.state.goal_state.completed_subgoals {
            println!("  ✓ {}", subgoal);
        }
    }

    if !runtime.state.goal_state.pending_subgoals.is_empty() {
        println!("\nPending:");
        for subgoal in &runtime.state.goal_state.pending_subgoals {
            println!("  ○ {}", subgoal);
        }
    }

    if let Some(ref reason) = runtime.state.goal_state.blocked_reason {
        println!("\n⚠ Blocked: {}", reason);
    }

    println!();
    Ok(())
}

/// Discover and show available tools (Task 015: Autonomous Tool Discovery)
pub(crate) fn handle_discover_tools(runtime: &AppRuntime) -> Result<()> {
    println!("\nDiscovering workspace tools...");

    match tool_discovery::discover_workspace_tools(&runtime.workspace.repo) {
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
        &runtime.state.session,
        &runtime.workspace.repo,
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
    let result = match rollback_workspace_snapshot(&runtime.state.session, &runtime.workspace.repo, snapshot_id) {
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
            runtime.config.model_id
        ),
    );
    let mut tune_args = runtime.args.clone();
    tune_args.tune = true;
    tune_args.calibrate = false;
    let winner = optimize_model(
        &tune_args,
        &runtime.config.client,
        &runtime.config.chat_url,
        &runtime.config.profiles.elma_cfg.base_url,
        &runtime.config.model_cfg_dir,
        &runtime.config.model_id,
    )
    .await?;

    runtime.config.profiles = app_bootstrap::load_profiles(&runtime.config.model_cfg_dir)?;
    set_json_outputter_profile(Some(runtime.config.profiles.json_outputter_cfg.clone()));
    set_final_answer_extractor_profile(Some(runtime.config.profiles.final_answer_extractor_cfg.clone()));
    refresh_runtime_workspace(runtime)?;

    print_elma_message(
        &runtime.args,
        &format!(
            "Tuning complete for {}. Activated score {:.3}. Certified: {}.",
            runtime.config.model_id, winner.score, winner.report.summary.certified
        ),
    );
    println!();
    Ok(())
}
