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
}

pub(crate) fn build_tool_definitions(_workdir: &PathBuf) -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "shell".to_string(),
                description: "Execute a shell command and return its output.".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"command": {"type": "string"}},
                    "required": ["command"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "read".to_string(),
                description: "Read the contents of a file.".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "search".to_string(),
                description: "Search for text patterns in files using ripgrep.".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string"},
                        "path": {"type": "string"}
                    },
                    "required": ["pattern"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "respond".to_string(),
                description: "Provide a final answer to the user.".to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {"answer": {"type": "string"}},
                    "required": ["answer"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "update_todo_list".to_string(),
                description: "Create and update a local task/todo list for multi-step work."
                    .to_string(),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {"type":"string","enum":["add","update","in_progress","completed","blocked","remove","list"]},
                        "id": {"type":"integer"},
                        "text": {"type":"string"},
                        "reason": {"type":"string"}
                    },
                    "required": ["action"]
                })),
            },
        },
    ]
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
            }
        }
    };
    match tool_name.as_str() {
        "shell" => exec_shell(args, &args_value, workdir, session, &call_id, tui).await,
        "read" => exec_read(&args_value, workdir, &call_id, tui),
        "search" => exec_search(&args_value, workdir, &call_id, tui).await,
        "respond" => exec_respond(&args_value, &call_id, tui),
        "update_todo_list" => exec_update_todo_list(&args_value, &call_id, tui),
        unknown => ToolExecutionResult {
            tool_call_id: call_id,
            tool_name: tool_name.clone(),
            content: format!("Unknown tool: {}", unknown),
            ok: false,
        },
    }
}

fn emit_tool_progress(
    tui: &mut Option<&mut crate::ui_terminal::TerminalUI>,
    name: &str,
    message: &str,
) {
    if let Some(t) = tui.as_mut() {
        t.add_claude_message(crate::claude_ui::ClaudeMessage::ToolProgress {
            name: name.to_string(),
            message: message.to_string(),
        });
    }
}

fn emit_tool_start(tui: &mut Option<&mut crate::ui_terminal::TerminalUI>, name: &str, input: &str) {
    if let Some(t) = tui.as_mut() {
        t.add_claude_message(crate::claude_ui::ClaudeMessage::ToolStart {
            name: name.to_string(),
            input: Some(input.to_string()),
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
        t.add_claude_message(crate::claude_ui::ClaudeMessage::ToolResult {
            name: name.to_string(),
            success,
            output: output.to_string(),
            duration_ms: None,
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
            content: error_msg,
            ok: false,
        };
    }

    // Task 118: Log unscoped warnings to trace (warning is in error_guidance)
    if let Some(warning) = &preflight.error_guidance {
        trace(
            args,
            &format!("tool_call: shell UNSCOPED WARNING: {}", warning),
        );
    }

    // Task 117: Permission gate for destructive/caution commands
    emit_tool_progress(&mut tui, "shell", "checking permissions");
    if !permission_gate::check_permission(args, &command, tui.as_deref_mut()).await {
        trace(args, "tool_call: shell DENIED by permission gate");
        let denied_msg = "Permission denied. You declined to execute this command.\nTo proceed, approve the command or use a safer alternative.".to_string();
        emit_tool_result(&mut tui, "shell", false, &denied_msg);
        return ToolExecutionResult {
            tool_call_id: call_id.to_string(),
            tool_name: "shell".to_string(),
            content: denied_msg,
            ok: false,
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
        };
    }

    // Replace spinner with TUI update for execution.
    // The TUI has a persistent status bar, so the spinner is no longer needed.
    // However, I still need to update the TUI when command execution is complete.
    emit_tool_progress(&mut tui, "shell", "executing command");

    match run_shell_one_liner(&command, workdir, None).await {
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
                session.shell_dir.join(format!("tool_{}.sh", call_id)),
                &command,
            );
            let _ = std::fs::write(
                session.shell_dir.join(format!("tool_{}.out", call_id)),
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
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "shell".to_string(),
                content,
                ok: er.exit_code == 0,
            }
        }
        Err(e) => {
            let error_msg = format!("Shell execution error: {}", e);
            emit_tool_result(&mut tui, "shell", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "shell".to_string(),
                content: error_msg,
                ok: false,
            }
        }
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
            content: error_msg,
            ok: false,
        };
    }
    let full = if std::path::Path::new(&path).is_relative() {
        workdir.join(&path)
    } else {
        PathBuf::from(&path)
    };

    emit_tool_start(&mut tui, "read", &path);
    emit_tool_progress(&mut tui, "read", "reading file");

    match std::fs::read_to_string(&full) {
        Ok(c) => {
            let content = format!("File: {}\n{}", full.display(), c);
            emit_tool_result(&mut tui, "read", true, &content);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "read".to_string(),
                content,
                ok: true,
            }
        }
        Err(e) => {
            let error_msg = format!("Error reading {}: {}", full.display(), e);
            emit_tool_result(&mut tui, "read", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "read".to_string(),
                content: error_msg,
                ok: false,
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
            content: error_msg,
            ok: false,
        };
    }
    let cmd = if let Some(p) = &sp {
        format!(
            "rg --line-number --no-heading --color=never '{}' '{}'",
            pattern, p
        )
    } else {
        format!("rg --line-number --no-heading --color=never '{}'", pattern)
    };

    emit_tool_start(&mut tui, "search", &cmd);
    emit_tool_progress(&mut tui, "search", "running ripgrep");

    match run_shell_one_liner(&cmd, workdir, None).await {
        Ok(er) => {
            let success = er.exit_code == 0 || er.exit_code == 1; // ripgrep returns 1 for no matches, which is still a 'success' for the search
            let content = if er.exit_code == 0 {
                er.inline_text
            } else if er.exit_code == 1 {
                format!("No matches found for: {}", pattern)
            } else {
                format!("Search failed (exit {}):\n{}", er.exit_code, er.inline_text)
            };

            emit_tool_result(&mut tui, "search", success, &content);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "search".to_string(),
                content,
                ok: success,
            }
        }
        Err(e) => {
            let error_msg = format!("Search error: {}", e);
            emit_tool_result(&mut tui, "search", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "search".to_string(),
                content: error_msg,
                ok: false,
            }
        }
    }
}

fn exec_respond(
    av: &serde_json::Value,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let answer = av["answer"].as_str().unwrap_or("").to_string();
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "respond".to_string(),
        content: answer,
        ok: true,
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
    }
}
