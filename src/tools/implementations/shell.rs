use std::path::{PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use crate::tools::types::{ToolExecutionResult, ToolStatus};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, emit_tool_progress};
use crate::ui_trace::trace;
use crate::session_display::save_tool_display;
use crate::{Args, SessionPaths, ShellExecutionResult, shell_preflight, execution_profiles, permission_gate};

pub async fn exec_shell(
    args: &Args,
    av: &serde_json::Value,
    workdir: &PathBuf,
    session: &SessionPaths,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let command = av["command"].as_str().unwrap_or("").to_string();
    if command.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "shell", "Error: empty command");
    }
    trace(args, &format!("tool_call: shell command={}", command));

    emit_tool_start(&mut tui, "shell", &command);
    if let Some(ref mut t) = tui {
        let _ = t.pump_ui();
    }
    emit_tool_progress(&mut tui, "shell", "running safety preflight");

    // Task 116: Preflight validation before execution
    let preflight = shell_preflight::preflight_command(&command, workdir);
    if !preflight.can_execute() {
        let guidance = preflight
            .error_guidance
            .unwrap_or_else(|| "Command blocked by safety preflight.".to_string());
        trace(
            args,
            &format!("tool_call: shell PREFLIGHT BLOCKED: {}", guidance),
        );
        let error_msg = format!(
            "Command blocked:\n{}\n\nThe safety preflight detected an issue with this command.\nFix the issue and try again.",
            guidance
        );
        emit_tool_result(&mut tui, "shell", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "shell", &error_msg);
    }

    // Task 459: Check execution profile for command restrictions
    if let Some(profile) = execution_profiles::get_execution_profile() {
        if !execution_profiles::is_command_allowed(profile, &command) {
            let msg = format!(
                "Command blocked by execution profile '{}': command not allowed",
                profile.name
            );
            trace(args, &format!("tool_call: shell PROFILE BLOCKED: {}", msg));
            emit_tool_result(&mut tui, "shell", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "shell", &msg);
        }
    }

    // Task 118: Log unscoped warnings to trace (warning is in error_guidance)
    if let Some(warning) = &preflight.error_guidance {
        trace(
            args,
            &format!("tool_call: shell UNSCOPED WARNING: {}", warning),
        );
    }

    // Task 117: Permission gate for destructive/caution commands
    let risk = shell_preflight::classify_command(&command);
    let is_dangerous = matches!(risk, shell_preflight::RiskLevel::Dangerous(_));
    emit_tool_progress(&mut tui, "shell", "checking permissions");
    if !permission_gate::check_permission(args, &command, is_dangerous, tui.as_deref_mut()).await {
        trace(args, "tool_call: shell DENIED by permission gate");
        let denied_msg = "Permission denied. You declined to execute this command.\nTo proceed, approve the command or use a safer alternative.".to_string();
        emit_tool_result(&mut tui, "shell", false, &denied_msg);
        return ToolExecutionResult::new_failed(call_id, "shell", &denied_msg);
    }

    // Task 121: Budget check before execution
    emit_tool_progress(&mut tui, "shell", "checking command budget");
    let budget = crate::command_budget::get_budget();
    if let Err(msg) = budget.check_budget(&preflight.risk) {
        trace(args, &format!("tool_call: shell BUDGET BLOCKED: {}", msg));
        let budget_msg = format!(
            "Command blocked by session budget:\n{}\n\nBudget status: {}",
            msg,
            budget.status()
        );
        emit_tool_result(&mut tui, "shell", false, &budget_msg);
        return ToolExecutionResult::new_failed(call_id, "shell", &budget_msg);
    }

    // Tasks 123/124/125: Run pre-tool hooks
    emit_tool_progress(&mut tui, "shell", "running safety hooks");
    let hooks = crate::hook_system::get_hook_registry();
    if let Some(block_msg) = hooks.run_pre_hooks(&command, workdir) {
        trace(
            args,
            &format!("tool_call: shell PRE-HOOK BLOCKED: {}", block_msg),
        );
        let hook_msg = format!("Command blocked by safety hook:\n{}", block_msg);
        emit_tool_result(&mut tui, "shell", false, &hook_msg);
        return ToolExecutionResult::new_failed(call_id, "shell", &hook_msg);
    }

    // Task 119: Dry-run preview — show preview to model before executing destructive commands
    if let Some(preview) = &preflight.dry_run_preview {
        trace(
            args,
            &format!("tool_call: shell DRY-RUN PREVIEW: {}", preview),
        );
        let preview_msg = format!(
            "! Dry-run preview for this command:\n{}\n\nTo proceed, confirm by running the same command again. To adjust, modify the command and try again.",
            preview
        );
        emit_tool_result(&mut tui, "shell", true, &preview_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: preview_msg,
            ok: true,
            exit_code: None,
            timed_out: false,
            status: ToolStatus::Failed, // Keep original weirdness for now
            duration_ms: 0,
            signal_killed: None,
        };
    }

    // Task 458: Snapshot before risky shell commands
    if matches!(
        preflight.risk,
        shell_preflight::RiskLevel::Caution | shell_preflight::RiskLevel::Dangerous(_)
    ) {
        match crate::snapshot::create_workspace_snapshot(
            session,
            workdir,
            &format!("pre-shell snapshot before: {}", command),
            true,
        ) {
            Ok(snapshot) => {
                trace(
                    args,
                    &format!(
                        "snapshot_saved id={} for risky shell command",
                        snapshot.snapshot_id
                    ),
                );
            }
            Err(e) => {
                trace(args, &format!("snapshot_failed: {}", e));
            }
        }
    }

    let elapsed_secs = Arc::new(AtomicU64::new(0));
    let cancelled = Arc::new(AtomicBool::new(false));
    let c2 = cancelled.clone();
    let e2 = elapsed_secs.clone();

    let cmd = command.clone();
    let wd = workdir.clone();
    let mut handle = tokio::task::spawn_blocking(move || {
        crate::program_utils::run_shell_persistent_blocking(&cmd, &wd, &*c2, &*e2)
    });

    let shell_result: Result<ShellExecutionResult, anyhow::Error> = loop {
        tokio::select! {
            result = &mut handle => {
                match result {
                    Ok(r) => break r,
                    Err(join_err) => {
                        break Err(anyhow::anyhow!("Shell task panicked: {}", join_err));
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                let secs = elapsed_secs.load(Ordering::Relaxed);
                if secs > 0 {
                    let d = secs / 86400;
                    let h = (secs % 86400) / 3600;
                    let m = (secs % 3600) / 60;
                    let s = secs % 60;
                    let elapsed_str = format!(
                        "{}{}{}{}s",
                        if d > 0 { format!("{}d ", d) } else { String::new() },
                        if h > 0 { format!("{}h ", h) } else { String::new() },
                        if m > 0 { format!("{}m ", m) } else { String::new() },
                        s
                    );
                    if let Some(t) = tui.as_mut() {
                        t.handle_ui_event(crate::claude_ui::UiEvent::ToolProgress {
                            name: "shell".to_string(),
                            message: format!("running ({})", elapsed_str),
                        });
                        let _ = t.pump_ui();
                    }
                }
                #[cfg(not(windows))]
                if crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false) {
                    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                        if key.code == crossterm::event::KeyCode::Char('k')
                            && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                        {
                            cancelled.store(true, Ordering::SeqCst);
                            let d = secs / 86400;
                            let h = (secs % 86400) / 3600;
                            let m = (secs % 3600) / 60;
                            let s = secs % 60;
                            let cancel_str = format!(
                                "{}{}{}{}s",
                                if d > 0 { format!("{}d ", d) } else { String::new() },
                                if h > 0 { format!("{}h ", h) } else { String::new() },
                                if m > 0 { format!("{}m ", m) } else { String::new() },
                                s
                            );
                            if let Some(t) = tui.as_mut() {
                                t.handle_ui_event(crate::claude_ui::UiEvent::ToolProgress {
                                    name: "shell".to_string(),
                                    message: format!("cancelling ({})", cancel_str),
                                });
                                let _ = t.pump_ui();
                            }
                        }
                    }
                }
            }
        }
    };

    match shell_result {
        Ok(er) => {
            let success = er.exit_code == 0;

            // Record the command in budget (after successful execution)
            budget.record_command(&preflight.risk);
            // Confirm the command
            shell_preflight::confirm_command(&command);
            trace(
                args,
                &format!("tool_call: shell budget status: {}", budget.status()),
            );

            // Tasks 123/124/125: Run post-tool hooks
            let hooks = crate::hook_system::get_hook_registry();
            let post_results = hooks.run_post_hooks(&command, er.exit_code == 0, &er.inline_text);
            for pr in &post_results {
                if let Some(msg) = &pr.message {
                    trace(
                        args,
                        &format!("tool_call: shell POST-HOOK [{}]: {}", pr.hook_name, msg),
                    );
                }
            }

            // Tasks 123/124/125: Run context modifiers
            let modifier_msgs =
                hooks.run_context_modifiers(&command, er.exit_code == 0, &er.inline_text);
            for msg in &modifier_msgs {
                trace(args, &format!("tool_call: shell CONTEXT MODIFIER: {}", msg));
            }

            let output = &er.inline_text;
            let lc = output.lines().count();

            // Task 538: Detect silent truncation by head/tail/limiters
            let mut output_with_warning = output.clone();
            if er.exit_code == 0 {
                if let Some(limit) = extract_line_limit(&command) {
                    if lc >= limit {
                        output_with_warning.push_str(&format!(
                            "\n\n! [TRUNCATED] Output matches line limit ({} lines). Full output may contain more content. Increase the limit or refine your command if needed.",
                            limit
                        ));
                    }
                }
            }

            let _ = std::fs::write(
                session.artifacts_dir.join(format!("tool_{}.sh", call_id)),
                &command,
            );
            let _ = std::fs::write(
                session.artifacts_dir.join(format!("tool_{}.out", call_id)),
                &output_with_warning,
            );
            trace(
                args,
                &format!("tool_call: shell exit_code={} lines={}", er.exit_code, lc),
            );
            let content = if er.exit_code == 0 {
                output_with_warning
            } else {
                let error_msgs = hooks.run_context_modifier_errors(&command, output);
                let error_context = if error_msgs.is_empty() {
                    String::new()
                } else {
                    format!("\n\nContext guidance:\n{}", error_msgs.join("\n"))
                };
                format!(
                    "Command failed (exit code {}):\n{}{}",
                    er.exit_code, output, error_context
                )
            };
            emit_tool_result(&mut tui, "shell", success, &content);
            let _ = save_tool_display(session, "shell", &command, &content, success);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "shell".to_string(),
                content,
                ok: er.exit_code == 0,
                status: if er.exit_code == 0 {
                    ToolStatus::Success
                } else {
                    ToolStatus::ExecutionError
                },
                exit_code: Some(er.exit_code),
                timed_out: er.timed_out,
                signal_killed: None,
                duration_ms: 0,
            }
        }
        Err(e) => {
            let error_msg = format!("Shell execution error: {}", e);
            emit_tool_result(&mut tui, "shell", false, &error_msg);
            let _ = save_tool_display(session, "shell", &command, &error_msg, false);
            let is_timeout = error_msg.to_ascii_lowercase().contains("timed out")
                || error_msg.to_ascii_lowercase().contains("idle timeout")
                || error_msg.to_ascii_lowercase().contains("cancelled");
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "shell".to_string(),
                content: error_msg.to_string(),
                status: if is_timeout {
                    ToolStatus::TimedOut
                } else {
                    ToolStatus::ExecutionError
                },
                ok: false,
                exit_code: None,
                timed_out: is_timeout,
                signal_killed: None,
                duration_ms: 0,
            }
        }
    }
}

fn extract_line_limit(command: &str) -> Option<usize> {
    // Check for | head -N, | head -n N, | tail -N, | tail -n N
    let patterns = [
        r"\|\s*head\s*-n\s*(\d+)",
        r"\|\s*head\s*-(\d+)",
        r"\|\s*tail\s*-n\s*(\d+)",
        r"\|\s*tail\s*-(\d+)",
    ];

    for p in patterns {
        if let Ok(re) = regex::Regex::new(p) {
            if let Some(caps) = re.captures(command) {
                if let Some(m) = caps.get(1) {
                    if let Ok(limit) = m.as_str().parse::<usize>() {
                        return Some(limit);
                    }
                }
            }
        }
    }
    None
}
