//! @efficiency-role: domain-logic
//! Tool Calling Registry — dispatcher for all tool executors.

use crate::*;

// ToolExecutionResult lives in crate::tools::types; re-export for backward compat.
pub(crate) use crate::tools::types::ToolExecutionResult;

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
    client: &reqwest::Client,
    _chat_url: &Url,
    _intent: &str,
    tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let call_id = tool_call.id.clone();
    let tool_name = tool_call.function.name.clone();
    // First try direct parse; on failure, try the model JSON repair pipeline
    let args_value: serde_json::Value = match serde_json::from_str(&tool_call.function.arguments) {
        Ok(v) => v,
        Err(_first_err) => {
            let raw = &tool_call.function.arguments;
            // Attempt repair via parse_model_json
            match crate::json_parser::parse_model_json::<serde_json::Value>(raw) {
                Ok(v) => {
                    crate::append_trace_log_line(&format!(
                        "[TOOL_PARSE_REPAIRED] tool={} raw preview={:?}",
                        tool_name,
                        raw.chars().take(100).collect::<String>()
                    ));
                    v
                }
                Err(_) => {
                    let preview: String = raw.chars().take(300).collect();
                    let detail = if raw.len() > 300 {
                        format!("{}…", preview)
                    } else {
                        preview
                    };
                    crate::append_trace_log_line(&format!(
                        "[TOOL_PARSE_ERROR] tool={} raw={:?}",
                        tool_name, detail
                    ));
                    return ToolExecutionResult {
                        tool_call_id: call_id,
                        tool_name,
                        content: format!("Error parsing arguments after repair attempt: {}", detail),
                        ok: false,
                        exit_code: None,
                        timed_out: false,
                        signal_killed: None,
                    }
                }
            }
        }
    };

    // Validate arguments against tool schema before dispatch
    if let Some(schema) = crate::tools::validation::get_tool_schema(&tool_name) {
        let validation = schema.validate(&args_value);
        if !validation.ok {
            let error_msg = validation.field_errors
                .iter()
                .map(|fe| format!("{}: {}", fe.field, fe.error))
                .collect::<Vec<_>>()
                .join("; ");
            return ToolExecutionResult {
                tool_call_id: call_id,
                tool_name,
                content: format!("Argument validation failed: {}", error_msg),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    }

    match tool_name.as_str() {
        "ls" => exec_ls(&args_value, workdir, &call_id, tui),
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
        "workspace_info" => exec_workspace_info(workdir, &call_id, tui),
        "exists" => exec_exists(&args_value, workdir, &call_id, tui),
        "repo_map" => exec_repo_map(&args_value, workdir, &call_id, tui).await,
        "git_inspect" => exec_git_inspect(&args_value, workdir, &call_id, tui).await,
        "run_python" => exec_run_python(&args_value, workdir, &call_id, tui).await,
        "run_node" => exec_run_node(&args_value, workdir, &call_id, tui).await,
        "job_start" => exec_job_start(&args_value, workdir, &call_id, tui).await,
        "job_status" => exec_job_status(&args_value, workdir, &call_id, tui).await,
        "job_output" => exec_job_output(&args_value, workdir, &call_id, tui).await,
        "job_stop" => exec_job_stop(&args_value, workdir, &call_id, tui).await,
        "fetch" => exec_fetch(client, &args_value, &call_id, tui).await,
        unknown => {
            crate::append_trace_log_line(&format!(
                "[TOOL_UNKNOWN] name={:?} args={}",
                unknown,
                &tool_call.function.arguments.chars().take(200).collect::<String>()
            ));
            let hint = if unknown.contains("read") || unknown.contains("Read") {
                format!("Unknown tool: {}. Did you mean 'read'?", unknown)
            } else if ["list", "ls", "dir", "cat", "head", "tail", "find", "grep", "echo", "sh", "bash", "zsh", "which", "where"].contains(&unknown) {
                format!("Unknown tool: {}. Did you mean 'shell'?", unknown)
            } else if unknown.contains("search") || unknown.contains("Search") || unknown.contains("grep") || unknown == "rg" {
                format!("Unknown tool: {}. Did you mean 'search' or 'shell' with grep?", unknown)
            } else if unknown.contains("glob") || unknown.contains("Glob") {
                format!("Unknown tool: {}. Did you mean 'glob'?", unknown)
            } else {
                format!("Unknown tool: {}", unknown)
            };
            ToolExecutionResult {
                tool_call_id: call_id,
                tool_name: tool_name.clone(),
                content: hint,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
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
            
            // Task 538: Detect silent truncation by head/tail/limiters
            let mut output_with_warning = output.clone();
            if er.exit_code == 0 {
                if let Some(limit) = extract_line_limit(&command) {
                    if lc >= limit {
                        output_with_warning.push_str(&format!(
                            "\n\n⚠️ [TRUNCATED] Output matches line limit ({} lines). Full output may contain more content. Increase the limit or refine your command if needed.",
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
            // Return full output — truncation is handled by tool_result_storage budget
            let content = if er.exit_code == 0 {
                output_with_warning
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

fn exec_ls(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let raw_path = av["path"].as_str().unwrap_or("").to_string();
    let depth = av["depth"].as_i64().unwrap_or(2).clamp(1, 5) as usize;
    let ignore_patterns: Vec<String> = av["ignore"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let target = if raw_path.is_empty() {
        workdir.clone()
    } else if std::path::Path::new(&raw_path).is_absolute() {
        let error_msg = "absolute_path_not_allowed: use workspace-relative path".to_string();
        emit_tool_result(&mut tui, "ls", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "ls".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    } else {
        workdir.join(&raw_path)
    };

    emit_tool_start(&mut tui, "ls", &raw_path);

    let md = match std::fs::symlink_metadata(&target) {
        Ok(m) => m,
        Err(e) => {
            let error_msg = format!("Error accessing {}: {}", target.display(), e);
            emit_tool_result(&mut tui, "ls", false, &error_msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "ls".to_string(),
                content: error_msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    if md.is_file() {
        let modified = md.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| format_time(d.as_secs()))
            .unwrap_or_default();
        let content = format!(
            "File: {}  ({} B, modified {})",
            target.file_name().unwrap_or_default().to_string_lossy(),
            md.len(),
            modified
        );
        emit_tool_result(&mut tui, "ls", true, &content);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "ls".to_string(),
            content,
            ok: true,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    if !md.is_dir() {
        let error_msg = format!("Not a directory or file: {}", target.display());
        emit_tool_result(&mut tui, "ls", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "ls".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let mut entries: Vec<LsEntry> = Vec::new();
    let total_count = collect_entries(&target, &target, depth, &ignore_patterns, &mut entries);

    let max_entries = 1000;
    let truncated = entries.len() > max_entries;
    if truncated {
        entries.truncate(max_entries);
    }

    let mut lines = Vec::new();
    let display_name = if raw_path.is_empty() {
        ".".to_string()
    } else {
        raw_path.clone()
    };
    lines.push(format!("{}/  ({} item(s))", display_name, total_count));

    for entry in &entries {
        let indent = "    ".repeat(entry.depth);
        let modified = format_time(entry.modified_secs);
        let size_str = if entry.is_dir {
            String::new()
        } else {
            format!("  ({} B, {})", entry.size, modified)
        };
        let suffix = if entry.is_dir { "/" } else { "" };
        lines.push(format!("{}{}{}{}", indent, entry.name, suffix, size_str));
    }

    if truncated {
        lines.push(format!("... and {} more entries", total_count.saturating_sub(max_entries)));
    }

    let content = lines.join("\n");
    emit_tool_result(&mut tui, "ls", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "ls".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

struct LsEntry {
    name: String,
    depth: usize,
    is_dir: bool,
    size: u64,
    modified_secs: u64,
}

fn collect_entries(
    root: &std::path::Path,
    dir: &std::path::Path,
    max_depth: usize,
    ignore_patterns: &[String],
    entries: &mut Vec<LsEntry>,
) -> usize {
    let current_depth = if dir == root {
        0
    } else {
        dir.strip_prefix(root)
            .map(|p| p.components().count())
            .unwrap_or(0)
    };

    if current_depth > max_depth {
        return 0;
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return 0,
    };

    let mut local: Vec<LsEntry> = Vec::new();
    let mut total: usize = 0;

    for entry in read_dir.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if is_ignored(&name, ignore_patterns) {
            continue;
        }
        total += 1;

        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_dir = ft.is_dir();
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified_secs = md.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        local.push(LsEntry {
            name,
            depth: current_depth,
            is_dir,
            size: md.len(),
            modified_secs,
        });

        if is_dir && current_depth < max_depth {
            total += collect_entries(root, &entry.path(), max_depth, ignore_patterns, entries);
        }
    }

    local.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.cmp(&b.name)
        }
    });

    entries.extend(local);
    total
}

fn is_ignored(name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if let Ok(true) = glob::Pattern::new(pattern).map(|p| p.matches(name)) {
            return true;
        }
    }
    false
}

fn format_time(secs: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let diff = now.saturating_sub(secs);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604800 {
        format!("{}d ago", diff / 86400)
    } else {
        match chrono::DateTime::from_timestamp(secs as i64, 0) {
            Some(dt) => dt.format("%b %d").to_string(),
            None => "unknown".to_string(),
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
    #[cfg(unix)]
    lines.push(format!("permissions: {:o}", std::os::unix::fs::MetadataExt::mode(&md) & 0o777));
    #[cfg(not(unix))]
    lines.push(format!("permissions: {:?}", md.permissions()));
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
    let paths: Vec<String> = if let Some(arr) = av["paths"].as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        let single = av["path"].as_str().unwrap_or("").to_string();
        if single.is_empty() {
            Vec::new()
        } else {
            vec![single]
        }
    };

    if paths.is_empty() {
        let error_msg = "Error: no path or paths provided".to_string();
        emit_tool_result(&mut tui, "read", false, &error_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "read".to_string(),
            content: error_msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let is_multi = paths.len() > 1;
    let mut all_content = String::new();
    let mut errors: Vec<String> = Vec::new();

    for (i, tp) in paths.iter().enumerate() {
        if std::path::Path::new(tp).is_absolute() {
            let err = format!("absolute_path_not_allowed: {} — use workspace-relative path", tp);
            if is_multi {
                errors.push(err.clone());
                all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                continue;
            } else {
                emit_tool_result(&mut tui, "read", false, &err);
                return ToolExecutionResult {
                    tool_call_id: call_id.to_string(),
                    tool_name: "read".to_string(),
                    content: err,
                    ok: false,
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
                };
            }
        }

        let full = workdir.join(tp);
        if !full.exists() {
            let err = format!("file_not_found: {}", tp);
            if is_multi {
                errors.push(err.clone());
                all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                continue;
            } else {
                emit_tool_result(&mut tui, "read", false, &err);
                return ToolExecutionResult {
                    tool_call_id: call_id.to_string(),
                    tool_name: "read".to_string(),
                    content: err,
                    ok: false,
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
                };
            }
        }

        match crate::document_adapter::read_file_smart(&full) {
            Ok((content, header)) => {
                let file_block = if is_multi {
                    format!("### File {}: {}\n{}\n\n{}", i + 1, tp, header, content)
                } else {
                    format!("{}\n{}", header, content)
                };
                all_content.push_str(&file_block);
                if i < paths.len() - 1 {
                    all_content.push_str("\n\n");
                }
            }
            Err(e) => {
                let err = format!("Error reading {}: {}", tp, e);
                if is_multi {
                    errors.push(err.clone());
                    all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                } else {
                    emit_tool_result(&mut tui, "read", false, &err);
                    return ToolExecutionResult {
                        tool_call_id: call_id.to_string(),
                        tool_name: "read".to_string(),
                        content: err,
                        ok: false,
                        exit_code: None,
                        timed_out: false,
                        signal_killed: None,
                    };
                }
            }
        }
    }

    let ok = errors.is_empty();
    emit_tool_start(&mut tui, "read", &paths[0]);
    emit_tool_progress(&mut tui, "read", "reading file(s)");
    emit_tool_result(&mut tui, "read", ok, &all_content);

    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "read".to_string(),
        content: all_content,
        ok,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
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

fn verify_syntax(path: &str, workdir: &PathBuf) -> Result<(), String> {
    if path.ends_with(".rs") {
        let mut curr = workdir.clone();
        let mut found_cargo = false;
        for _ in 0..5 {
            if curr.join("Cargo.toml").exists() {
                found_cargo = true;
                break;
            }
            if let Some(parent) = curr.parent() {
                curr = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        if found_cargo {
            let output = std::process::Command::new("cargo")
                .arg("check")
                .arg("--message-format=short")
                .current_dir(&curr)
                .output();
            
            match output {
                Ok(out) if !out.status.success() => {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let combined = format!("{}\n{}", stdout, stderr);
                    return Err(format!("Cargo check failed after mutation:\n{}", combined.trim()));
                }
                Err(e) => return Err(format!("Failed to run cargo check: {}", e)),
                _ => {}
            }
        }
    }
    Ok(())
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

            // Task 543: Verify syntax if any Rust files were touched
            for op in &parsed.operations {
                let p = match op {
                    PatchOperation::AddFile { path, .. } => path,
                    PatchOperation::UpdateFile { path, .. } => path,
                    PatchOperation::DeleteFile { .. } => continue,
                };
                if let Err(e) = verify_syntax(p, workdir) {
                    all_ok = false;
                    results.push(format!("Verification failed: {}", e));
                    break;
                }
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
            // Task 543: Verify syntax
            if let Err(e) = verify_syntax(&path, workdir) {
                emit_tool_result(&mut tui, "edit", false, &e);
                return ToolExecutionResult {
                    tool_call_id: call_id.to_string(),
                    tool_name: "edit".to_string(),
                    content: e,
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
            // Task 543: Verify syntax
            if let Err(e) = verify_syntax(&path, workdir) {
                emit_tool_result(&mut tui, "write", false, &e);
                return ToolExecutionResult {
                    tool_call_id: call_id.to_string(),
                    tool_name: "write".to_string(),
                    content: e,
                    ok: false,
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
                };
            }
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
    } else {
        // Task 542: Exclude _knowledge_base by default to avoid noise in audits
        cmd.arg("--glob").arg("!_knowledge_base/**");
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
            let mut content = if exit_code == 0 {
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

            // Task 542: Add annotation if matches include _knowledge_base
            if content.contains("_knowledge_base/") {
                let kb_count = content.lines().filter(|l| l.contains("_knowledge_base/")).count();
                let total_count = content.lines().count();
                content.push_str(&format!(
                    "\n\nℹ️ NOTE: {} of {} matches are in _knowledge_base/ (third-party reference code). Exclude these from risk analysis of Elma's own codebase.",
                    kb_count, total_count
                ));
            }

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

// --- Repo Map Tool (Task 463) ---

async fn exec_repo_map(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let token_budget = av["token_budget"].as_u64().unwrap_or(2000) as usize;
    let max_files = av["max_files"].as_u64().unwrap_or(50) as usize;

    emit_tool_start(&mut tui, "repo_map", "building repo map");

    let (output, tokens_used) = repo_map::build_repo_map(workdir, token_budget, max_files);

    let content = format!(
        "{}",
        output
    );

    emit_tool_result(&mut tui, "repo_map", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "repo_map".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

// --- Git Inspection Tool (Task 462) ---

async fn exec_git_inspect(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let mode = av["mode"].as_str().unwrap_or("status").to_string();
    let path = av["path"].as_str().unwrap_or("").to_string();

    let workdir = if path.is_empty() {
        workdir.clone()
    } else {
        workdir.join(&path)
    };

    let (args, description) = match mode.as_str() {
        "status" => (vec!["status", "--porcelain"], "git status"),
        "branch" => (vec!["branch", "-vv"], "git branch"),
        "changed_files" => (vec!["diff", "--name-only"], "git changed files"),
        "diffstat" => (vec!["diff", "--stat"], "git diff stat"),
        "recent_commits" => (vec!["log", "--oneline", "-10"], "git recent commits"),
        _ => {
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "git_inspect".to_string(),
                content: format!("Error: unknown mode '{}'", mode),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    emit_tool_start(&mut tui, "git_inspect", &description);

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(&workdir)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            let exit_code = o.status.code().unwrap_or(-1);
            let ok = o.status.success();

            let mut content = format!("Git {} (mode: {})\n", description, mode);
            if !stdout.is_empty() {
                content.push_str("--- stdout ---\n");
                content.push_str(&stdout);
            }
            if !stderr.is_empty() {
                content.push_str("\n--- stderr ---\n");
                content.push_str(&stderr);
            }
            content.push_str(&format!("\nExit code: {}", exit_code));

            emit_tool_result(&mut tui, "git_inspect", ok, &content);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "git_inspect".to_string(),
                content,
                ok,
                exit_code: Some(exit_code),
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let msg = format!("Failed to execute git: {}", e);
            emit_tool_result(&mut tui, "git_inspect", false, &msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "git_inspect".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

// --- Interpreter Tools (Task 461) ---

async fn exec_run_python(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let code = av["code"].as_str().unwrap_or("").to_string();
    if code.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "run_python".to_string(),
            content: "Error: empty code".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let timeout_seconds = av["timeout_seconds"].as_u64().unwrap_or(30);

    emit_tool_start(&mut tui, "run_python", &code[..code.len().min(50)]);

    match interpreter_tools::execute_code("python", &code, workdir, timeout_seconds, 1000).await {
        Ok((stdout, stderr, exit_code)) => {
            let mut output = String::new();
            if !stdout.is_empty() {
                output.push_str("--- stdout ---\n");
                output.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str("--- stderr ---\n");
                output.push_str(&stderr);
            }
            let ok = exit_code == 0;
            emit_tool_result(&mut tui, "run_python", ok, &output);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "run_python".to_string(),
                content: output,
                ok,
                exit_code: Some(exit_code),
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let msg = format!("Python execution error: {}", e);
            emit_tool_result(&mut tui, "run_python", false, &msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "run_python".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

async fn exec_run_node(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let code = av["code"].as_str().unwrap_or("").to_string();
    if code.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "run_node".to_string(),
            content: "Error: empty code".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let timeout_seconds = av["timeout_seconds"].as_u64().unwrap_or(30);

    emit_tool_start(&mut tui, "run_node", &code[..code.len().min(50)]);

    match interpreter_tools::execute_code("node", &code, workdir, timeout_seconds, 1000).await {
        Ok((stdout, stderr, exit_code)) => {
            let mut output = String::new();
            if !stdout.is_empty() {
                output.push_str("--- stdout ---\n");
                output.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str("--- stderr ---\n");
                output.push_str(&stderr);
            }
            let ok = exit_code == 0;
            emit_tool_result(&mut tui, "run_node", ok, &output);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "run_node".to_string(),
                content: output,
                ok,
                exit_code: Some(exit_code),
                timed_out: false,
                signal_killed: None,
            }
        }
        Err(e) => {
            let msg = format!("Node execution error: {}", e);
            emit_tool_result(&mut tui, "run_node", false, &msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "run_node".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }
        }
    }
}

// --- Background Job Tools (Task 460) ---

async fn exec_job_start(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let command = av["command"].as_str().unwrap_or("").to_string();
    if command.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "job_start".to_string(),
            content: "Error: empty command".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    // Task 460: Check execution profile for command restrictions
    if let Some(profile) = execution_profiles::get_execution_profile() {
        if !execution_profiles::is_command_allowed(profile, &command) {
            let msg = format!(
                "Job start blocked by execution profile '{}': command not allowed",
                profile.name
            );
            emit_tool_result(&mut tui, "job_start", false, &msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_start".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    }

    let name = av["name"].as_str().unwrap_or("").to_string();
    let memory_limit_mb = av["memory_limit_mb"].as_u64();
    let timeout_seconds = av["timeout_seconds"].as_u64();

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_start".to_string(),
                content: "Error: TaskManager not initialized".to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    emit_tool_start(&mut tui, "job_start", &command);

    let id = match task_manager.create_task(
        if name.is_empty() { "background_job".to_string() } else { name },
        command.clone(),
        workdir.clone(),
        memory_limit_mb,
        timeout_seconds,
    ).await {
        Ok(id) => id,
        Err(e) => {
            let msg = format!("Failed to create task: {}", e);
            emit_tool_result(&mut tui, "job_start", false, &msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_start".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    if let Err(e) = task_manager.start_task(&id).await {
        let msg = format!("Failed to start task: {}", e);
        emit_tool_result(&mut tui, "job_start", false, &msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "job_start".to_string(),
            content: msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let content = format!("Job started with ID: {}", id);
    emit_tool_result(&mut tui, "job_start", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "job_start".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

async fn exec_job_status(
    av: &serde_json::Value,
    _workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let job_id = av["job_id"].as_str().unwrap_or("").to_string();
    if job_id.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "job_status".to_string(),
            content: "Error: empty job_id".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_status".to_string(),
                content: "Error: TaskManager not initialized".to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    emit_tool_start(&mut tui, "job_status", &job_id);

    let task = match task_manager.get_task(&job_id).await {
        Some(t) => t,
        None => {
            let msg = format!("Job not found: {}", job_id);
            emit_tool_result(&mut tui, "job_status", false, &msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_status".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let runtime = task.runtime_seconds().unwrap_or(0);
    let content = format!(
        "Job ID: {}\nName: {}\nStatus: {}\nExit code: {}\nRuntime: {}s\nMemory: {}MB",
        task.id,
        task.name,
        task.status,
        task.exit_code.map_or("N/A".to_string(), |c| c.to_string()),
        runtime,
        task.memory_usage_mb
    );

    emit_tool_result(&mut tui, "job_status", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "job_status".to_string(),
        content,
        ok: true,
        exit_code: task.exit_code,
        timed_out: false,
        signal_killed: None,
    }
}

async fn exec_job_output(
    av: &serde_json::Value,
    _workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let job_id = av["job_id"].as_str().unwrap_or("").to_string();
    if job_id.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "job_output".to_string(),
            content: "Error: empty job_id".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_output".to_string(),
                content: "Error: TaskManager not initialized".to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    emit_tool_start(&mut tui, "job_output", &job_id);

    let task = match task_manager.get_task(&job_id).await {
        Some(t) => t,
        None => {
            let msg = format!("Job not found: {}", job_id);
            emit_tool_result(&mut tui, "job_output", false, &msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_output".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let mut output = String::new();
    output.push_str("--- stdout ---\n");
    for line in &task.stdout_buffer {
        output.push_str(line);
        output.push('\n');
    }
    output.push_str("\n--- stderr ---\n");
    for line in &task.stderr_buffer {
        output.push_str(line);
        output.push('\n');
    }

    emit_tool_result(&mut tui, "job_output", true, &output);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "job_output".to_string(),
        content: output,
        ok: true,
        exit_code: task.exit_code,
        timed_out: false,
        signal_killed: None,
    }
}

async fn exec_job_stop(
    av: &serde_json::Value,
    _workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let job_id = av["job_id"].as_str().unwrap_or("").to_string();
    if job_id.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "job_stop".to_string(),
            content: "Error: empty job_id".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "job_stop".to_string(),
                content: "Error: TaskManager not initialized".to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    emit_tool_start(&mut tui, "job_stop", &job_id);

    if let Err(e) = task_manager.cancel_task(&job_id).await {
        let msg = format!("Failed to stop job: {}", e);
        emit_tool_result(&mut tui, "job_stop", false, &msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "job_stop".to_string(),
            content: msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let content = format!("Job {} stopped", job_id);
    emit_tool_result(&mut tui, "job_stop", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "job_stop".to_string(),
        content,
        ok: true,
        exit_code: None,
        timed_out: false,
        signal_killed: None,
    }
}

async fn exec_fetch(
    client: &reqwest::Client,
    av: &serde_json::Value,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let url_str = av["url"].as_str().unwrap_or("").to_string();
    let format = av["format"].as_str().unwrap_or("text").to_string();
    let timeout_secs = av["timeout"].as_u64().unwrap_or(120);

    if url_str.is_empty() {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "fetch".to_string(),
            content: "Error: empty URL".to_string(),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let parsed_url = match url::Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "fetch".to_string(),
                content: format!("Error: invalid URL: {}", e),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let scheme = parsed_url.scheme();
    if scheme != "http" && scheme != "https" {
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "fetch".to_string(),
            content: format!("Error: only http and https schemes are allowed, got '{}'", scheme),
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    emit_tool_start(&mut tui, "fetch", &url_str);

    let request = client
        .get(parsed_url.as_str())
        .timeout(std::time::Duration::from_secs(timeout_secs.min(120)))
        .header("User-Agent", "ElmaCLI/1.0")
        .send();

    let response = match request.await {
        Ok(r) => r,
        Err(e) => {
            let msg = if e.is_timeout() {
                format!("Error: request timed out after {}s", timeout_secs)
            } else {
                format!("Error: request failed: {}", e)
            };
            emit_tool_result(&mut tui, "fetch", false, &msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "fetch".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let status = response.status();
    if !status.is_success() {
        let msg = format!("Error: HTTP {}", status);
        emit_tool_result(&mut tui, "fetch", false, &msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "fetch".to_string(),
            content: msg,
            ok: false,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
        };
    }

    let raw_bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            let msg = format!("Error: failed to read response body: {}", e);
            emit_tool_result(&mut tui, "fetch", false, &msg);
            return ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "fetch".to_string(),
                content: msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            };
        }
    };

    let capped = &raw_bytes[..raw_bytes.len().min(100_000)];

    let content = match format.as_str() {
        "markdown" => {
            html2text::from_read(capped, 120).unwrap_or_else(|_| String::from_utf8_lossy(capped).to_string())
        }
        "html" => {
            String::from_utf8_lossy(capped).to_string()
        }
        _ => {
            String::from_utf8_lossy(capped).to_string()
        }
    };

    let truncated = if raw_bytes.len() > 100_000 {
        format!("{}\n\n[Content truncated at 100KB — fetched {} bytes total]", content, raw_bytes.len())
    } else {
        content
    };

    emit_tool_result(&mut tui, "fetch", true, &truncated);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "fetch".to_string(),
        content: truncated,
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
        let wd = std::env::temp_dir();
        let result = exec_observe(&serde_json::json!({"path": ""}), &wd, "o1", None);
        assert!(!result.ok);
        assert!(result.content.contains("empty path"));
    }

    #[test]
    fn observe_nonexistent_path_returns_exists_false() {
        let wd = std::env::temp_dir();
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

        let wd = std::env::temp_dir();
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

        let wd = std::env::temp_dir();
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

fn exec_workspace_info(
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    emit_tool_start(&mut tui, "workspace_info", "");
    let mut info = String::new();

    info.push_str(&format!("## Workspace Root\n{}\n\n", workdir.display()));

    info.push_str("## Directory Structure\n");
    if let Ok(entries) = std::fs::read_dir(workdir) {
        let mut items: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') || name == "target" || name == "node_modules"
                || name == "dist" || name == "build"
            {
                continue;
            }
            let marker = if path.is_dir() { "/" } else { "" };
            if path.is_dir() {
                let mut sub_items = String::new();
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    let mut subs: Vec<String> = sub_entries
                        .flatten()
                        .filter_map(|e| {
                            let sp = e.path();
                            let sn = sp.file_name()?.to_string_lossy().to_string();
                            if sn.starts_with('.') { return None; }
                            let sm = if sp.is_dir() { "/" } else { "" };
                            Some(format!("    {}{}", sn, sm))
                        })
                        .take(20)
                        .collect();
                    subs.sort();
                    if !subs.is_empty() {
                        sub_items = format!("\n{}", subs.join("\n"));
                    }
                }
                items.push(format!("  {}{}{}", name, marker, sub_items));
            } else {
                items.push(format!("  {}{}", name, marker));
            }
            if items.len() >= 100 { break; }
        }
        items.sort();
        info.push_str(&items.join("\n"));
    }
    info.push_str("\n\n");

    info.push_str("## Project Type\n");
    let checks: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js/JavaScript/TypeScript"),
        ("pyproject.toml", "Python"),
        ("setup.py", "Python"),
        ("go.mod", "Go"),
        ("Makefile", "Make-based project"),
        ("CMakeLists.txt", "CMake/C++"),
        ("Gemfile", "Ruby"),
        ("composer.json", "PHP"),
        ("pom.xml", "Java/Maven"),
        ("build.gradle", "Java/Gradle"),
        ("requirements.txt", "Python"),
        ("Dockerfile", "Docker container"),
        ("docker-compose.yml", "Docker Compose"),
        (".github/workflows", "GitHub Actions CI"),
    ];
    let mut found = false;
    for (file, label) in checks {
        if workdir.join(file).exists() {
            info.push_str(&format!("- {} ({})\n", label, file));
            found = true;
        }
    }
    if !found {
        info.push_str("- Generic (no recognized project file)\n");
    }

    if workdir.join(".git").exists() {
        info.push_str("\n## Git Status\n");
        let branch = std::process::Command::new("git")
            .args(["-C", &workdir.display().to_string(), "branch", "--show-current"])
            .output();
        if let Ok(out) = branch {
            let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !b.is_empty() {
                info.push_str(&format!("Branch: {}\n", b));
            }
        }
        let status = std::process::Command::new("git")
            .args(["-C", &workdir.display().to_string(), "status", "--short"])
            .output();
        if let Ok(out) = status {
            let text = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() {
                info.push_str("Working tree clean\n");
            } else {
                let modified = lines.iter().filter(|l| l.starts_with(" M") || l.starts_with("M ")).count();
                let untracked = lines.iter().filter(|l| l.starts_with("??")).count();
                let staged = lines.iter().filter(|l| l.starts_with("M ") || l.starts_with("A ")).count();
                info.push_str(&format!(
                    "{} staged, {} modified, {} untracked files\n",
                    staged, modified, untracked
                ));
                info.push_str("Recent changes:\n");
                for line in lines.iter().take(20) {
                    info.push_str(&format!("  {}\n", line));
                }
                if lines.len() > 20 {
                    info.push_str(&format!("  ... and {} more\n", lines.len() - 20));
                }
            }
        }
    }

    let guidance_files = [
        ("AGENTS.md", 1600usize),
        ("_tasks/TASKS.md", 1200),
    ];
    let mut guidance_section = String::new();
    for (rel_path, max_chars) in &guidance_files {
        let full_path = workdir.join(rel_path);
        if full_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let trimmed: String = content.chars().take(*max_chars).collect();
                guidance_section.push_str(&format!(
                    "\n### {}\n```\n{}\n```\n",
                    rel_path, trimmed
                ));
                if content.chars().count() > *max_chars {
                    guidance_section.push_str("...(truncated)\n");
                }
            }
        }
    }
    let active_dir = workdir.join("_tasks").join("active");
    if active_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&active_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let trimmed: String = content.chars().take(800).collect();
                        let name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        guidance_section.push_str(&format!(
                            "\n### Active task: {}\n```\n{}\n```\n",
                            name, trimmed
                        ));
                    }
                    break;
                }
            }
        }
    }
    if !guidance_section.is_empty() {
        info.push_str("\n## Project Guidance\n");
        info.push_str(&guidance_section);
    }

    emit_tool_result(&mut tui, "workspace_info", true, &info);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "workspace_info".to_string(),
        content: info,
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

/// Task 538: Helper to extract line limits from shell commands (e.g. head -N, tail -N)
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
