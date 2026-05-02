//! @efficiency-role: domain-logic
//! Tool Calling Registry

use crate::*;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct ToolExecutionResult {
    pub(crate) tool_call_id: String,
    pub(crate) tool_name: String,
    pub(crate) content: String,
    pub(crate) ok: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) timed_out: bool,
    pub(crate) signal_killed: Option<i32>,
}

/// Build initial tool definitions - only non-deferred tools (default tools)
pub(crate) fn build_tool_definitions(_workdir: &PathBuf) -> Vec<ToolDefinition> {
    crate::tool_registry::build_current_tools()
}

/// Build tool definitions filtered by task context (route/classification).
pub(crate) fn build_tool_definitions_for_context(
    _workdir: &PathBuf,
    context_hint: &str,
) -> Vec<ToolDefinition> {
    if context_hint.is_empty() {
        crate::tool_registry::build_current_tools()
    } else {
        crate::tool_registry::build_tools_for_context(context_hint)
    }
}

/// Get dynamically loaded tools by name
pub(crate) fn get_dynamic_tools(tool_names: &[String]) -> Vec<ToolDefinition> {
    crate::tool_registry::get_registry().get_tools(tool_names)
}

/// Search for tools by query and return their definitions
pub(crate) fn search_tools(query: &str) -> Vec<ToolDefinition> {
    crate::tool_registry::get_registry().search_and_convert(query)
}

/// Get tool names from search (for marking as discovered)
pub(crate) fn search_tool_names(query: &str) -> Vec<String> {
    crate::tool_registry::get_registry().get_tool_names(query)
}

pub(crate) async fn execute_tool_call(
    args: &Args,
    tool_call: &ToolCall,
    workdir: &PathBuf,
    session: &SessionPaths,
    _client: &reqwest::Client,
    _chat_url: &Url,
    _intent: &str,
    tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let call_id = tool_call.id.clone();
    let tool_name = tool_call.function.name.clone();
    let args_value: serde_json::Value = match serde_json::from_str(&tool_call.function.arguments) {
        Ok(v) => v,
        Err(e) => {
            return ToolExecutionResult {
                tool_call_id: call_id,
                tool_name,
                content: format!("Error parsing arguments: {}", e),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    };
    match tool_name.as_str() {
        "observe" => exec_observe(&args_value, workdir, &call_id, tui),
        "tool_search" => exec_tool_search(&args_value, &call_id, tui),
        "shell" => exec_shell(args, &args_value, workdir, session, &call_id, tui).await,
        "read" => exec_read(&args_value, workdir, &call_id, tui),
        "glob" => exec_glob(&args_value, workdir, &call_id, tui),
        "patch" => exec_patch(&args_value, workdir, &call_id, tui),
        "edit" => exec_edit(&args_value, workdir, &call_id, tui),
        "write" => exec_write(&args_value, workdir, &call_id, tui),
        "search" => exec_search(&args_value, workdir, &call_id, tui).await,
        "respond" => exec_respond(&args_value, &call_id, tui),
        "summary" => exec_summary(&args_value, &call_id, tui),
        "update_todo_list" => exec_update_todo_list(&args_value, &call_id, tui),
        "stat" => exec_stat(&args_value, workdir, &call_id, tui),
        "copy" => exec_copy(&args_value, workdir, &call_id, tui),
        "move" => exec_move(&args_value, workdir, &call_id, tui),
        "mkdir" => exec_mkdir(&args_value, workdir, &call_id, tui),
        "trash" => exec_trash(&args_value, workdir, &call_id, tui),
        "touch" => exec_touch(&args_value, workdir, &call_id, tui),
        "file_size" => exec_file_size(&args_value, workdir, &call_id, tui),
        "exists" => exec_exists(&args_value, workdir, &call_id, tui),
        unknown => ToolExecutionResult {
            tool_call_id: call_id,
            tool_name: tool_name.clone(),
            content: format!("Unknown tool: {}", unknown),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        },
    }
}

fn emit_tool_progress(
    _tui: &mut Option<&mut crate::ui_terminal::TerminalUI>,
    _name: &str,
    _message: &str,
) {
    // Progress messages are now implicit via ToolTrace Running state.
}

fn emit_tool_start(tui: &mut Option<&mut crate::ui_terminal::TerminalUI>, name: &str, input: &str) {
    if let Some(t) = tui.as_mut() {
        t.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
            name: name.to_string(),
            command: input.to_string(),
        });
    }
}

fn emit_tool_result(
    tui: &mut Option<&mut crate::ui_terminal::TerminalUI>,
    name: &str,
    success: bool,
    output: &str,
) {
    if let Some(t) = tui.as_mut() {
        t.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
            name: name.to_string(),
            success,
            output: output.to_string(),
        });
    }
}

async fn exec_shell(
    args: &Args,
    av: &serde_json::Value,
    workdir: &PathBuf,
    session: &SessionPaths,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let command = av["command"].as_str().unwrap_or("").to_string();
    if command.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: "Error: empty command".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }
    trace(args, &format!("tool_call: shell command={}", command));

    emit_tool_start(&mut tui, "shell", &command);
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
        let error_msg = format!("Command blocked:\n{}\n\nThe safety preflight detected an issue with this command.\nFix the issue and try again.", guidance);
        emit_tool_result(&mut tui, "shell", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
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
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "shell".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
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
    // Use classify_command to determine actual risk level instead of hardcoding destructive
    let risk = shell_preflight::classify_command(&command);
    let is_dangerous = matches!(risk, shell_preflight::RiskLevel::Dangerous(_));
    emit_tool_progress(&mut tui, "shell", "checking permissions");
    if !permission_gate::check_permission(args, &command, is_dangerous, tui.as_deref_mut()).await {
        trace(args, "tool_call: shell DENIED by permission gate");
        let denied_msg = "Permission denied. You declined to execute this command.\nTo proceed, approve the command or use a safer alternative.".to_string();
        emit_tool_result(&mut tui, "shell", false, &denied_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: denied_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
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
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: budget_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
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
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: hook_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    // Task 119: Dry-run preview — show preview to model before executing destructive commands
    if let Some(preview) = &preflight.dry_run_preview {
        trace(
            args,
            &format!("tool_call: shell DRY-RUN PREVIEW: {}", preview),
        );
        let preview_msg = format!("⚠️ Dry-run preview for this command:\n{}\n\nTo proceed, confirm by running the same command again. To adjust, modify the command and try again.", preview);
        emit_tool_result(&mut tui, "shell", true, &preview_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: preview_msg,
            ok: true,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    // Replace spinner with TUI update for execution.
    emit_tool_progress(&mut tui, "shell", "executing command");

    // Task 458: Snapshot before risky shell commands
    if matches!(preflight.risk, shell_preflight::RiskLevel::Caution | shell_preflight::RiskLevel::Dangerous(_)) {
        match crate::snapshot::create_workspace_snapshot(
            session,
            workdir,
            &format!("pre-shell snapshot before: {}", command),
            true,
        ) {
            Ok(snapshot) => {
                trace(args, &format!("snapshot_saved id={} for risky shell command", snapshot.snapshot_id));
            }
            Err(e) => {
                trace(args, &format!("snapshot_failed: {}", e));
            }
        }
    }

    match run_shell_persistent(&command, workdir).await {
        Ok(er) => {
            let success = er.exit_code == 0;
            if let Some(t) = tui.as_mut() {}

            // Record the command in budget (after successful execution)
            budget.record_command(&preflight.risk);
            // Confirm the command (won't show dry-run again if model re-runs it)
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
            let _ = std::fs::write(
                session.artifacts_dir.join(format!("tool_{}.sh", call_id)),
                &command,
            );
            let _ = std::fs::write(
                session.artifacts_dir.join(format!("tool_{}.out", call_id)),
                output,
            );
            trace(
                args,
                &format!("tool_call: shell exit_code={} lines={}", er.exit_code, lc),
            );
            // Return full output — truncation is handled by tool_result_storage budget
            let content = if er.exit_code == 0 {
                output.clone()
            } else {
                // Run context modifier errors for failed commands
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
                exit_code: Some(er.exit_code),
                timed_out: er.timed_out,
                signal_killed: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Shell execution error: {}", e);
            emit_tool_result(&mut tui, "shell", false, &error_msg);
            let _ = save_tool_display(session, "shell", &command, &error_msg, false);
            let is_timeout = error_msg.to_ascii_lowercase().contains("timed out");
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "shell".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: is_timeout,
                signal_killed: None,
            }
        }
    }
}

fn exec_observe(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("").to_string();
    if path.is_empty() {
        let error_msg = "Error: empty path".to_string();
        emit_tool_result(&mut tui, "observe", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "observe".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full = if std::path::Path::new(&path).is_relative() {
        workdir.join(&path)
    } else if std::path::Path::new(&path).is_absolute() {
        let error_msg = format!("absolute_path_not_allowed: {} — use workspace-relative path", path);
        emit_tool_result(&mut tui, "observe", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "observe".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    } else {
        workdir.join(&path)
    };

    emit_tool_start(&mut tui, "observe", &path);

    let md = match std::fs::symlink_metadata(&full) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let content = format!(
                "path: {}\nexists: false",
                full.display()
            );
            emit_tool_result(&mut tui, "observe", true, &content);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "observe".to_string(),
                content,
                ok: true,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
        Err(e) => {
            let error_msg = format!("Error inspecting {}: {}", full.display(), e);
            emit_tool_result(&mut tui, "observe", false, &error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "observe".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let file_type_str = if md.file_type().is_symlink() {
        "symlink"
    } else if md.file_type().is_dir() {
        "directory"
    } else if md.file_type().is_file() {
        "file"
    } else {
        "other"
    };

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("path: {}", full.display()));
    lines.push(format!("exists: true"));
    lines.push(format!("type: {}", file_type_str));
    lines.push(format!("size: {}", md.len()));
    if let Ok(mtime) = md.modified() {
        match mtime.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => lines.push(format!("modified: {}", d.as_secs())),
            Err(_) => {}
        }
    }
    lines.push(format!("permissions: {:o}", std::os::unix::fs::MetadataExt::mode(&md) & 0o777));
    lines.push(format!("readonly: {}", md.permissions().readonly()));

    // Symlink target
    let mut is_symlink = false;
    if md.file_type().is_symlink() {
        is_symlink = true;
        match std::fs::read_link(&full) {
            Ok(target) => {
                lines.push(format!("symlink_target: {}", target.display()));
            }
            Err(_) => {
                lines.push("symlink_target: <unreadable>".to_string());
            }
        }
    }

    // Directory child count
    if file_type_str == "directory" {
        match std::fs::read_dir(&full) {
            Ok(entries) => {
                let count = entries.filter_map(|e| e.ok()).count();
                lines.push(format!("child_count: {}", count));
            }
            Err(_) => {}
        }
    }

    let content = lines.join("\n");
    emit_tool_result(&mut tui, "observe", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "observe".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_read(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("").to_string();
    if path.is_empty() {
        let error_msg = "Error: empty path".to_string();
        emit_tool_result(&mut tui, "read", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "read".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }
    let full = if std::path::Path::new(&path).is_relative() {
        workdir.join(&path)
    } else if std::path::Path::new(&path).is_absolute() {
        let error_msg = format!("absolute_path_not_allowed: {} — use workspace-relative path", path);
        emit_tool_result(&mut tui, "read", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "read".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    } else {
        workdir.join(&path)
    };

    emit_tool_start(&mut tui, "read", &path);
    emit_tool_progress(&mut tui, "read", "reading file");

    match crate::document_adapter::read_file_smart(&full) {
        Ok((content, header)) => {
            let content = format!("{}\n{}", header, content);
            emit_tool_result(&mut tui, "read", true, &content);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "read".to_string(),
                content,
                ok: true,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Error reading {}: {}", full.display(), e);
            emit_tool_result(&mut tui, "read", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "read".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

fn exec_glob(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let pattern = av["pattern"].as_str().unwrap_or("*").to_string();
    let search_path = av["path"].as_str().map(PathBuf::from);

    emit_tool_start(&mut tui, "glob", &pattern);

    let base = match search_path {
        Some(p) if p.is_absolute() => {
            let error_msg = "absolute_path_not_allowed";
            emit_tool_result(&mut tui, "glob", false, error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "glob".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
        Some(p) => workdir.join(p),
        None => workdir.clone(),
    };

    let walker = glob::glob_with(
        &pattern,
        glob::MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        },
    );

    let mut results = Vec::new();
    let mut count = 0;
    let max_results = 100;

    if let Ok(walker) = walker {
        for entry in walker.filter_map(|e| e.ok()) {
            if count >= max_results {
                break;
            }
            let relative = entry
                .strip_prefix(workdir)
                .unwrap_or(&entry)
                .display()
                .to_string();
            results.push(relative);
            count += 1;
        }
    }

    let output = if results.is_empty() {
        "No files found matching pattern".to_string()
    } else {
        results.join("\n")
    };

    emit_tool_result(&mut tui, "glob", !results.is_empty(), &output);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "glob".to_string(),
        content: output,
        ok: !results.is_empty(),
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_patch(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let patch_content = av["patch"].as_str().unwrap_or("").to_string();
    if patch_content.is_empty() {
        let error_msg = "Error: patch content is empty";
        emit_tool_result(&mut tui, "patch", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "patch".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "patch", "(multi-file patch)");

    use elma_tools::{parse_patch, PatchOperation};

    match parse_patch(&patch_content) {
        Ok(parsed) => {
            let mut results = Vec::new();
            let mut all_ok = true;

            for op in &parsed.operations {
                let (path, result_msg) = match op {
                    PatchOperation::AddFile { path, content } => {
                        let full = workdir.join(path);
                        match std::fs::create_dir_all(full.parent().unwrap_or(&full)) {
                            Ok(_) => match std::fs::write(&full, content) {
                                Ok(_) => (path.clone(), "added".to_string()),
                                Err(e) => {
                                    all_ok = false;
                                    (path.clone(), format!("write failed: {}", e))
                                }
                            },
                            Err(e) => {
                                all_ok = false;
                                (path.clone(), format!("dir create failed: {}", e))
                            }
                        }
                    }
                    PatchOperation::DeleteFile { path } => {
                        let full = workdir.join(path);
                        match std::fs::remove_file(&full) {
                            Ok(_) => (path.clone(), "deleted".to_string()),
                            Err(e) => {
                                all_ok = false;
                                (path.clone(), format!("delete failed: {}", e))
                            }
                        }
                    }
                    PatchOperation::UpdateFile { path, old_string, new_string } => {
                        let full = workdir.join(path);
                        match std::fs::read_to_string(&full) {
                            Ok(original) => {
                                if let Some(pos) = original.find(old_string) {
                                    let mut updated = original.clone();
                                    updated.replace_range(pos..pos + old_string.len(), new_string);
                                    match std::fs::write(&full, &updated) {
                                        Ok(_) => (path.clone(), "updated".to_string()),
                                        Err(e) => {
                                            all_ok = false;
                                            (path.clone(), format!("write failed: {}", e))
                                        }
                                    }
                                } else {
                                    all_ok = false;
                                    (path.clone(), "old_string not found".to_string())
                                }
                            }
                            Err(e) => {
                                all_ok = false;
                                (path.clone(), format!("read failed: {}", e))
                            }
                        }
                    }
                };
                results.push(format!("{}: {}", path, result_msg));
            }

            let output = results.join("\n");
            emit_tool_result(&mut tui, "patch", all_ok, &output);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "patch".to_string(),
                content: output,
                ok: all_ok,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Error parsing patch: {}", e);
            emit_tool_result(&mut tui, "patch", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "patch".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

fn exec_edit(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("").to_string();
    let old_string = av["old_string"].as_str().unwrap_or("").to_string();
    let new_string = av["new_string"].as_str().unwrap_or("").to_string();

    if path.is_empty() {
        let error_msg = "Error: path is required".to_string();
        emit_tool_result(&mut tui, "edit", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "edit".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full = workdir.join(&path);

    if full.is_absolute() {
        let error_msg = "absolute_path_not_allowed";
        emit_tool_result(&mut tui, "edit", false, error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "edit".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "edit", &path);

    let content = match std::fs::read_to_string(&full) {
        Ok(c) => c,
        Err(e) => {
            let error_msg = format!("Error reading file: {}", e);
            emit_tool_result(&mut tui, "edit", false, &error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "edit".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    if !old_string.is_empty() {
        if let Some(pos) = content.find(&old_string) {
            let mut updated = content.clone();
            updated.replace_range(pos..pos + old_string.len(), &new_string);
            if let Err(e) = std::fs::write(&full, &updated) {
                let error_msg = format!("Error writing file: {}", e);
                emit_tool_result(&mut tui, "edit", false, &error_msg);
                return ToolExecutionResult {
                    tool_call_id: call_id.to_string(),
                    tool_name: "edit".to_string(),
                    content: error_msg.to_string(),
                    ok: false,
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
                };
            }
            emit_tool_result(&mut tui, "edit", true, "edited");
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "edit".to_string(),
                content: "edited".to_string(),
                ok: true,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        } else {
            let error_msg = "old_string not found in file".to_string();
            emit_tool_result(&mut tui, "edit", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "edit".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    } else {
        let error_msg = "old_string is required".to_string();
        emit_tool_result(&mut tui, "edit", false, &error_msg);
        ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "edit".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        }
    }
}

fn exec_write(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("").to_string();
    let content = av["content"].as_str().unwrap_or("").to_string();

    if path.is_empty() {
        let error_msg = "Error: path is required".to_string();
        emit_tool_result(&mut tui, "write", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "write".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full = workdir.join(&path);

    if full.is_absolute() {
        let error_msg = "absolute_path_not_allowed";
        emit_tool_result(&mut tui, "write", false, error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "write".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "write", &path);

    if let Some(parent) = full.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            let error_msg = format!("Error creating directory: {}", e);
            emit_tool_result(&mut tui, "write", false, &error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "write".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    }

    match std::fs::write(&full, &content) {
        Ok(_) => {
            emit_tool_result(&mut tui, "write", true, "written");
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "write".to_string(),
                content: "written".to_string(),
                ok: true,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Error writing file: {}", e);
            emit_tool_result(&mut tui, "write", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "write".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

async fn exec_search(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let pattern = av["pattern"].as_str().unwrap_or("").to_string();
    let sp = av["path"].as_str().map(String::from);
    if pattern.is_empty() {
        let error_msg = "Error: empty search pattern".to_string();
        emit_tool_result(&mut tui, "search", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "search".to_string(),
            content: error_msg.to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    if let Some(ref p) = sp {
        if std::path::Path::new(p).is_absolute() {
            let error_msg = format!("absolute_path_not_allowed: {} — use workspace-relative path", p);
            emit_tool_result(&mut tui, "search", false, &error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "search".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    }

    let mut cmd = std::process::Command::new("rg");
    cmd.arg("-i")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--color=never");

    // Task 454: Honor literal_text and include schema fields
    let literal_text = av["literal_text"].as_bool().unwrap_or(false);
    if literal_text {
        cmd.arg("-F"); // Fixed string (literal) search
    }
    cmd.arg(&pattern);

    if let Some(include) = av["include"].as_str() {
        if !include.is_empty() {
            cmd.arg("--glob").arg(include);
        }
    }

    if let Some(p) = &sp {
        let search_path = workdir.join(p);
        if search_path.exists() {
            cmd.arg(&search_path);
        }
    } else {
        cmd.arg(workdir);
    }

    emit_tool_start(&mut tui, "search", &format!("rg pattern={}", pattern));
    emit_tool_progress(&mut tui, "search", "running ripgrep");

    match cmd.output() {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(0);
            let success = exit_code == 0 || exit_code == 1;
            let content = if exit_code == 0 {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else if exit_code == 1 {
                format!("No matches found for: {}", pattern)
            } else {
                format!(
                    "Search failed (exit {}):\n{}",
                    exit_code,
                    String::from_utf8_lossy(&output.stderr)
                )
            };

            emit_tool_result(&mut tui, "search", success, &content);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "search".to_string(),
                content,
                ok: success,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Search error: {}", e);
            emit_tool_result(&mut tui, "search", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "search".to_string(),
                content: error_msg.to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

fn exec_respond(
    av: &serde_json::Value,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let answer = av["answer"]
        .as_str()
        .or_else(|| av["content"].as_str())
        .or_else(|| av["text"].as_str())
        .map(crate::text_utils::strip_thinking_blocks)
        .unwrap_or_default();
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "respond".to_string(),
        content: answer,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_summary(
    av: &serde_json::Value,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let content = av["content"]
        .as_str()
        .map(crate::text_utils::strip_thinking_blocks)
        .unwrap_or_default();
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "summary".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_tool_search(
    av: &serde_json::Value,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let query = av["query"].as_str().unwrap_or("").to_string();
    if query.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "tool_search".to_string(),
            content: "Error: query is required".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let tools = search_tools(&query);
    if tools.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "tool_search".to_string(),
            content: format!("No tools found matching: '{}'", query),
            ok: true,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    // Mark tools as discovered so they become available in future requests
    let tool_names = search_tool_names(&query);
    crate::tool_registry::mark_discovered(&tool_names);

    // Format tool definitions as JSON for the model
    let tools_json = serde_json::to_string_pretty(&tools).unwrap_or_default();
    let content = format!(
        "Found {} tool(s) matching '{}':\n\n{}\n\nThese tools are now loaded and available for use. You can call them directly in your next response.",
        tools.len(),
        query,
        tools_json
    );

    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "tool_search".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn respond_accepts_content_alias() {
        let result = exec_respond(
            &serde_json::json!({"content":"<think>hidden</think>Visible"}),
            "c1",
            None,
        );
        assert_eq!(result.content, "Visible");
    }

    #[test]
    fn observe_empty_path_returns_error() {
        let wd = PathBuf::from("/tmp");
        let result = exec_observe(&serde_json::json!({"path": ""}), &wd, "o1", None);
        assert!(!result.ok);
        assert!(result.content.contains("empty path"));
    }

    #[test]
    fn observe_nonexistent_path_returns_exists_false() {
        let wd = PathBuf::from("/tmp");
        let result = exec_observe(
            &serde_json::json!({"path": "/nonexistent_path_xyzabc123"}),
            &wd,
            "o2",
            None,
        );
        assert!(result.ok);
        assert!(result.content.contains("exists: false"));
    }

    #[test]
    fn observe_file_returns_metadata() {
        let dir = std::env::temp_dir().join("observe_test_file");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test.txt");
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(b"hello world").unwrap();
        f.flush().unwrap();

        let wd = PathBuf::from("/tmp");
        let result = exec_observe(
            &serde_json::json!({"path": file_path.to_str().unwrap()}),
            &wd,
            "o3",
            None,
        );
        assert!(result.ok, "result: {}", result.content);
        assert!(result.content.contains("exists: true"));
        assert!(result.content.contains("type: file"));
        assert!(result.content.contains("size: 11"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn observe_directory_shows_child_count() {
        let dir = std::env::temp_dir().join("observe_test_dir");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();
        std::fs::write(dir.join("c.txt"), "c").unwrap();

        let wd = PathBuf::from("/tmp");
        let result = exec_observe(
            &serde_json::json!({"path": dir.to_str().unwrap()}),
            &wd,
            "o4",
            None,
        );
        assert!(result.ok, "result: {}", result.content);
        assert!(result.content.contains("exists: true"));
        assert!(result.content.contains("type: directory"));
        assert!(result.content.contains("child_count: 3"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn observe_relative_path_resolves_to_workdir() {
        let dir = std::env::temp_dir().join("observe_test_rel");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("rel_file.txt");
        std::fs::write(&file_path, "data").unwrap();

        let result = exec_observe(
            &serde_json::json!({"path": "rel_file.txt"}),
            &dir,
            "o5",
            None,
        );
        assert!(result.ok, "result: {}", result.content);
        assert!(result.content.contains("exists: true"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn observe_symlink_shows_target() {
        let dir = std::env::temp_dir().join("observe_test_sym");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("target.txt");
        let link = dir.join("link.txt");
        std::fs::write(&target, "symlink target content").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).unwrap();
        }
        #[cfg(not(unix))]
        {
            std::fs::hard_link(&target, &link).unwrap();
        }

        let wd = PathBuf::from("/tmp");
        let result = exec_observe(
            &serde_json::json!({"path": link.to_str().unwrap()}),
            &wd,
            "o6",
            None,
        );
        assert!(result.ok, "result: {}", result.content);
        assert!(result.content.contains("type: symlink"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn exec_update_todo_list(
    av: &serde_json::Value,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let action = av["action"].as_str().unwrap_or("").trim().to_string();
    if action.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "update_todo_list".to_string(),
            content: "Error: action is required".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }
    let id = av["id"].as_u64().map(|v| v as u32);
    let text = av["text"].as_str().map(|s| s.to_string());
    let reason = av["reason"].as_str().map(|s| s.to_string());

    let content = match (action.as_str(), tui.as_mut()) {
        ("add", Some(t)) => {
            let desc = text.unwrap_or_else(|| "New task".to_string());
            let new_id = t.todo_add(desc.clone());
            format!("Added task {}: {}", new_id, desc)
        }
        ("update", Some(t)) => {
            if let (Some(id), Some(text)) = (id, text) {
                t.todo_update(id, text.clone());
                format!("Updated task {}: {}", id, text)
            } else {
                "Error: update requires id and text".to_string()
            }
        }
        ("in_progress", Some(t)) => {
            if let Some(id) = id {
                t.todo_start(id);
                format!("Task {} marked in progress", id)
            } else {
                "Error: in_progress requires id".to_string()
            }
        }
        ("completed", Some(t)) => {
            if let Some(id) = id {
                t.todo_complete(id);
                format!("Task {} marked completed", id)
            } else {
                "Error: completed requires id".to_string()
            }
        }
        ("blocked", Some(t)) => {
            if let Some(id) = id {
                t.todo_block(id, reason.clone());
                if let Some(r) = reason {
                    format!("Task {} blocked: {}", id, r)
                } else {
                    format!("Task {} blocked", id)
                }
            } else {
                "Error: blocked requires id".to_string()
            }
        }
        ("remove", Some(t)) => {
            if let Some(id) = id {
                if t.todo_remove(id) {
                    format!("Removed task {}", id)
                } else {
                    format!("Task {} not found", id)
                }
            } else {
                "Error: remove requires id".to_string()
            }
        }
        ("list", Some(t)) => {
            let lines = t.todo_render_lines();
            if lines.is_empty() {
                "No tasks".to_string()
            } else {
                lines.join("\n")
            }
        }
        (_, None) => "Todo updates require interactive TUI mode".to_string(),
        _ => format!("Unknown action: {}", action),
    };

    if let Some(t) = tui.as_mut() {
        t.add_claude_message(crate::claude_ui::ClaudeMessage::ToolResult {
            name: "update_todo_list".to_string(),
            success: !content.starts_with("Error:"),
            output: content.clone(),
            duration_ms: None,
        });
    }

    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "update_todo_list".to_string(),
        ok: !content.starts_with("Error:"),
        content,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_stat(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");
    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "stat", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "stat".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full_path = workdir.join(path);
    if !full_path.exists() {
        let error_msg = format!("Error: path not found: {}", path);
        emit_tool_result(&mut tui, "stat", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "stat".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let metadata = match std::fs::metadata(&full_path) {
        Ok(m) => m,
        Err(e) => {
            let error_msg = format!("Error: {}", e);
            emit_tool_result(&mut tui, "stat", false, &error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "stat".to_string(),
                content: error_msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let file_type = if metadata.is_dir() { "directory" } else if metadata.is_file() { "file" } else { "other" };
    let size = metadata.len();
    let modified = metadata.modified()
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).ok())
        .ok()
        .flatten();

    let content = format!(
        "Type: {}\nSize: {} bytes\nModified: {}",
        file_type,
        size,
        modified.map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string())
    );

    emit_tool_result(&mut tui, "stat", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "stat".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_copy(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let source = av["source"].as_str().unwrap_or("");
    let destination = av["destination"].as_str().unwrap_or("");

    if source.is_empty() || destination.is_empty() {
        let error_msg = "Error: source and destination required".to_string();
        emit_tool_result(&mut tui, "copy", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "copy".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let src = workdir.join(source);
    let dst = workdir.join(destination);

    if !src.exists() {
        let error_msg = format!("Error: source not found: {}", source);
        emit_tool_result(&mut tui, "copy", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "copy".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "copy", &format!("{} -> {}", source, destination));

    fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_recursive(&entry.path(), &dest_path)?;
            } else {
                std::fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }

    let result = if src.is_dir() {
        copy_dir_recursive(&src, &dst)
    } else {
        std::fs::copy(&src, &dst).map(|_| ())
    };

    let content = match &result {
        Ok(_) => format!("Copied {} to {}", source, destination),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "copy", ok, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "copy".to_string(),
        content,
        ok,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_move(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let source = av["source"].as_str().unwrap_or("");
    let destination = av["destination"].as_str().unwrap_or("");

    if source.is_empty() || destination.is_empty() {
        let error_msg = "Error: source and destination required".to_string();
        emit_tool_result(&mut tui, "move", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "move".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let src = workdir.join(source);
    let dst = workdir.join(destination);

    if !src.exists() {
        let error_msg = format!("Error: source not found: {}", source);
        emit_tool_result(&mut tui, "move", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "move".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "move", &format!("{} -> {}", source, destination));

    let result = std::fs::rename(&src, &dst);
    let content = match &result {
        Ok(_) => format!("Moved {} to {}", source, destination),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "move", ok, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "move".to_string(),
        content,
        ok,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_mkdir(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");
    let parents = av["parents"].as_bool().unwrap_or(true);

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "mkdir", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "mkdir".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full_path = workdir.join(path);
    emit_tool_start(&mut tui, "mkdir", path);

    let result = if parents {
        std::fs::create_dir_all(&full_path)
    } else {
        std::fs::create_dir(&full_path)
    };

    let content = match &result {
        Ok(_) => format!("Created directory: {}", path),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "mkdir", ok, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "mkdir".to_string(),
        content,
        ok,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_trash(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "trash", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "trash".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full_path = workdir.join(path);

    if !full_path.exists() {
        let error_msg = format!("Error: path not found: {}", path);
        emit_tool_result(&mut tui, "trash", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "trash".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "trash", path);

    let trash_dir = workdir.join(".trash");
    let _ = std::fs::create_dir_all(&trash_dir);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let trash_path = trash_dir.join(format!("{}_{}", timestamp, path.replace("/", "_")));

    let result = std::fs::rename(&full_path, &trash_path);
    let content = match &result {
        Ok(_) => format!("Moved to trash: {}", path),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "trash", ok, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "trash".to_string(),
        content,
        ok,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_touch(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "touch", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "touch".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full_path = workdir.join(path);
    emit_tool_start(&mut tui, "touch", path);

    let result = std::fs::write(&full_path, "");
    let content = match &result {
        Ok(_) => format!("Touched: {}", path),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "touch", ok, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "touch".to_string(),
        content,
        ok,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_file_size(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "file_size", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "file_size".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full_path = workdir.join(path);

    if !full_path.exists() {
        let error_msg = format!("Error: path not found: {}", path);
        emit_tool_result(&mut tui, "file_size", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "file_size".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    fn dir_size(p: &std::path::Path) -> u64 {
        let mut size = 0u64;
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        size += meta.len();
                    } else if meta.is_dir() {
                        size += dir_size(&entry.path());
                    }
                }
            }
        }
        size
    }

    let size = if full_path.is_dir() {
        dir_size(&full_path)
    } else {
        std::fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0)
    };

    let content = format!("Size: {} bytes", size);
    emit_tool_result(&mut tui, "file_size", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "file_size".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

fn exec_exists(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");
    let check_type = av["type"].as_str().unwrap_or("any");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "exists", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "exists".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let full_path = workdir.join(path);
    let exists = full_path.exists();

    let content = if !exists {
        "exists: false".to_string()
    } else {
        let actual_type = if full_path.is_dir() { "dir" } else if full_path.is_file() { "file" } else { "other" };
        let wanted_type = check_type;
        let matches = wanted_type == "any" || wanted_type == actual_type;
        format!("exists: true, type: {}, matches: {}", actual_type, matches)
    };

    emit_tool_result(&mut tui, "exists", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "exists".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}
