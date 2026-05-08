//! @efficiency-role: domain-logic
//! Tool Calling Registry — dispatcher for all tool executors.

use crate::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

// ToolExecutionResult lives in crate::tools::types; re-export for backward compat.
pub(crate) use crate::tools::types::ToolExecutionResult;

/// Take a snapshot before a mutating operation. Best-effort (errors are traced, not returned).
fn snapshot_before_mutation(
    args: &Args,
    session: &SessionPaths,
    workdir: &Path,
    tool: &str,
    target: &str,
) {
    let _ = crate::snapshot::create_workspace_snapshot(
        session,
        workdir,
        &format!("pre-{} snapshot before: {}", tool, target),
        true,
    )
    .map(|s| {
        trace(
            args,
            &format!("snapshot_saved id={} for {}", s.snapshot_id, tool),
        )
    })
    .map_err(|e| trace(args, &format!("snapshot_failed for {}: {}", tool, e)));
}

/// Build initial tool definitions - only non-deployed tools (default tools)
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

/// Generates a corrective guidance message after a tool failure.
/// Helps small models recover by providing the correct usage pattern.
/// Includes an exact copyable template for the tool call JSON.
pub(crate) fn format_tool_error_correction(tool_name: &str) -> String {
    let schema = crate::tools::validation::get_tool_schema(tool_name);
    match schema {
        Some(s) => {
            let base = s.format_schema_narrative();
            // Add exact copyable template if available
            if let Some(ref example) = s.usage_example {
                format!(
                    "{}\n\nCopy this exact JSON for your next call:\n{} arguments={}",
                    base, tool_name, example
                )
            } else {
                base
            }
        }
        None => format!(
            "Tool '{}' failed. Check the arguments and try again.",
            tool_name
        ),
    }
}

/// Parses ls tool output to extract file paths and suggests read calls.
/// Returns Some(text) with concrete read call suggestions, or None if no files found.
pub(crate) fn suggest_next_calls(ls_output: &str) -> Option<String> {
    let mut paths: Vec<&str> = Vec::new();

    for line in ls_output.lines() {
        let trimmed = line.trim();
        // Match lines containing file size in bytes (e.g. "  main.rs  (123 B, 2d ago)")
        if let Some(end) = trimmed.rfind("  (") {
            let name_part = &trimmed[..end].trim();
            // Skip directory entries (end with /) and truncated/header lines
            if !name_part.ends_with('/')
                && !name_part.contains("item(s)")
                && !name_part.is_empty()
                && paths.len() < 10
            {
                paths.push(name_part);
            }
        }
    }

    if paths.is_empty() {
        return None;
    }

    let examples: Vec<String> = paths
        .iter()
        .map(|p| format!("read filePath=\"{}\"", p))
        .collect();
    Some(format!(
        "Files available to read:\n- {}\n\nUse the read tool with the correct filePath parameter.",
        examples.join("\n- ")
    ))
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
                        content: format!(
                            "Error parsing arguments after repair attempt: {}",
                            detail
                        ),
                        ok: false,
                        exit_code: None,
                        timed_out: false,
                        status: crate::tools::ToolStatus::Failed,
                        duration_ms: 0,
                        signal_killed: None,
                    };
                }
            }
        }
    };

    // Normalize common aliases before schema validation.
    let args_value = canonicalize_tool_args(&tool_name, args_value);

    // Task 586: Pre-execution arg repair for read and exists tools.
    // If the model called read/exists without filePath/path, try to extract from raw args.
    let args_value = {
        let needs_repair = if tool_name == "read" {
            args_value
                .get("filePath")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
        } else if tool_name == "exists" {
            args_value
                .get("path")
                .and_then(|v| v.as_str())
                .map_or(true, |s| s.is_empty())
        } else {
            false
        };
        if needs_repair {
            let raw = &tool_call.function.arguments;
            let path_re =
                regex::Regex::new(r#"["']([a-zA-Z0-9_./\\-]+(?:\.[a-zA-Z0-9]+)?)["']"#).unwrap();
            if let Some(cap) = path_re.captures(raw) {
                let extracted = cap.get(1).unwrap().as_str().to_string();
                if extracted.contains('.') || extracted.contains('/') {
                    if let Some(obj) = args_value.as_object() {
                        let mut map = obj.clone();
                        let key = if tool_name == "read" {
                            "filePath"
                        } else {
                            "path"
                        };
                        map.insert(key.to_string(), serde_json::Value::String(extracted));
                        if let Ok(repaired) = serde_json::to_value(map) {
                            crate::append_trace_log_line(&format!(
                                "[TOOL_ARG_REPAIR] {}: injected {} from raw args",
                                tool_name, key
                            ));
                            repaired
                        } else {
                            args_value
                        }
                    } else {
                        args_value
                    }
                } else {
                    args_value
                }
            } else {
                args_value
            }
        } else {
            args_value
        }
    };

    // Validate arguments against tool schema before dispatch
    if let Some(schema) = crate::tools::validation::get_tool_schema(&tool_name) {
        let validation = schema.validate(&args_value);
        if !validation.ok {
            let error_msg = validation
                .field_errors
                .iter()
                .map(|fe| format!("{}: {}", fe.field, fe.error))
                .collect::<Vec<_>>()
                .join("; ");
            let rich_content = schema.format_error_with_schema(&error_msg);
            crate::append_trace_log_line(&format!(
                "[TOOL_VALIDATION_ERROR] tool={} error={}",
                tool_name, error_msg
            ));

            // Task 708: For read missing filePath, emit compact correction packet
            // instead of verbose JSON schema dump. The history-based arg repair
            // already ran in the loop; if we're here, no candidate path was found.
            let enriched_content = if crate::tool_repair::is_read_missing_filepath_error(
                &tool_name,
                &error_msg,
            ) {
                let count = crate::tool_repair::record_empty_read_validation_failure();
                crate::append_trace_log_line(&format!(
                    "[TOOL_VALIDATION_ERROR] tool=read repair_source=schema_packet count={}",
                    count
                ));
                crate::tool_repair::build_read_schema_correction_packet(&error_msg, &[])
            } else {
                rich_content
            };

            return ToolExecutionResult {
                tool_call_id: call_id,
                tool_name,
                content: enriched_content,
                ok: false,
                exit_code: None,
                timed_out: false,
                status: crate::tools::ToolStatus::ValidationFailed,
                duration_ms: 0,
                signal_killed: None,
            };
        }
    }

    match tool_name.as_str() {
        "ls" => crate::tools::implementations::ls::exec_ls(&args_value, workdir, &call_id, tui),
        "observe" => crate::tools::implementations::observe::exec_observe(&args_value, workdir, &call_id, tui),
        "tool_search" => crate::tools::implementations::tool_search::exec_tool_search(&args_value, &call_id, tui),
        "shell" => crate::tools::implementations::shell::exec_shell(args, &args_value, workdir, session, &call_id, tui).await,
        "read" => crate::tools::implementations::read::exec_read(&args_value, workdir, &call_id, tui),
        "glob" => crate::tools::implementations::glob::exec_glob(&args_value, workdir, &call_id, tui),
        "patch" => crate::tools::implementations::patch::exec_patch(&args_value, workdir, &call_id, tui),
        "edit" => crate::tools::implementations::edit::exec_edit(&args_value, workdir, &call_id, tui),
        "write" => crate::tools::implementations::write::exec_write(&args_value, workdir, &call_id, tui),
        "search" => crate::tools::implementations::search::exec_search(&args_value, workdir, &call_id, tui).await,
        "respond" => crate::tools::implementations::respond::exec_respond(&args_value, &call_id, tui),
        "update_todo_list" => crate::tools::implementations::update_todo_list::exec_update_todo_list(&args_value, &call_id, tui),
        "stat" => crate::tools::implementations::stat::exec_stat(&args_value, workdir, &call_id, tui),
        "backup" => crate::tools::implementations::backup::exec_backup(&args_value, workdir, &call_id, tui),
        "copy" => crate::tools::implementations::copy::exec_copy(&args_value, workdir, &call_id, tui),
        "move" => crate::tools::implementations::move_::exec_move(&args_value, workdir, &call_id, tui),
        "mkdir" => crate::tools::implementations::mkdir::exec_mkdir(&args_value, workdir, &call_id, tui),
        "trash" => crate::tools::implementations::trash::exec_trash(&args_value, workdir, &call_id, tui),
        "touch" => crate::tools::implementations::touch::exec_touch(&args_value, workdir, &call_id, tui),
        "file_size" => crate::tools::implementations::file_size::exec_file_size(&args_value, workdir, &call_id, tui),
        "workspace_info" => crate::tools::implementations::workspace_info::exec_workspace_info(workdir, &call_id, tui),
        "exists" => crate::tools::implementations::exists::exec_exists(&args_value, workdir, &call_id, tui),
        "repo_map" => crate::tools::implementations::repo_map::exec_repo_map(&args_value, workdir, &call_id, tui).await,
        "git_inspect" => crate::tools::implementations::git_inspect::exec_git_inspect(&args_value, workdir, &call_id, tui).await,
        "run_python" => crate::tools::implementations::run_python::exec_run_python(&args_value, workdir, &call_id, tui).await,
        "run_node" => crate::tools::implementations::run_node::exec_run_node(&args_value, workdir, &call_id, tui).await,
        "job_start" => crate::tools::implementations::job_start::exec_job_start(&args_value, workdir, &call_id, tui).await,
        "job_status" => crate::tools::implementations::job_status::exec_job_status(&args_value, workdir, &call_id, tui).await,
        "job_output" => crate::tools::implementations::job_output::exec_job_output(&args_value, workdir, &call_id, tui).await,
        "job_stop" => crate::tools::implementations::job_stop::exec_job_stop(&args_value, workdir, &call_id, tui).await,
        "fetch" => crate::tools::implementations::fetch::exec_fetch(client, &args_value, &call_id, tui).await,
        unknown => {
            crate::append_trace_log_line(&format!(
                "[TOOL_UNKNOWN] name={:?} args={}",
                unknown,
                &tool_call
                    .function
                    .arguments
                    .chars()
                    .take(200)
                    .collect::<String>()
            ));
            let hint = if unknown.contains("read") || unknown.contains("Read") {
                format!("Unknown tool: {}. Did you mean 'read'?", unknown)
            } else if [
                "list", "ls", "dir", "cat", "head", "tail", "find", "grep", "echo", "sh", "bash",
                "zsh", "which", "where",
            ]
            .contains(&unknown)
            {
                format!("Unknown tool: {}. Did you mean 'shell'?", unknown)
            } else if unknown.contains("search")
                || unknown.contains("Search")
                || unknown.contains("grep")
                || unknown == "rg"
            {
                format!(
                    "Unknown tool: {}. Did you mean 'search' or 'shell' with grep?",
                    unknown
                )
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
                status: crate::tools::ToolStatus::Failed,
                duration_ms: 0,
                signal_killed: None,
            }
        }
    }
}

fn canonicalize_tool_args(tool_name: &str, mut args_value: serde_json::Value) -> serde_json::Value {
    if tool_name == "read" {
        if let Some(obj) = args_value.as_object_mut() {
            let has_file_path = obj
                .get("filePath")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty());
            let has_path = obj
                .get("path")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty());
            if !has_file_path && has_path {
                if let Some(alias_path) = obj
                    .get("path")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string())
                {
                    obj.insert(
                        "filePath".to_string(),
                        serde_json::Value::String(alias_path.clone()),
                    );
                    crate::append_trace_log_line(&format!(
                        "[TOOL_ARG_CANONICALIZE] read: path -> filePath ({})",
                        alias_path
                    ));
                }
            } else if has_file_path && !has_path {
                if let Some(file_path) = obj
                    .get("filePath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string())
                {
                    obj.insert(
                        "path".to_string(),
                        serde_json::Value::String(file_path.clone()),
                    );
                    crate::append_trace_log_line(&format!(
                        "[TOOL_ARG_CANONICALIZE] read: filePath -> path ({})",
                        file_path
                    ));
                }
            }
            let has_paths = obj
                .get("paths")
                .and_then(|v| v.as_array())
                .and_then(|paths| paths.first())
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty());
            let has_file_path = obj
                .get("filePath")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty());
            if has_paths && !has_file_path {
                if let Some(first_path) = obj
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .and_then(|paths| paths.first())
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string())
                {
                    obj.insert(
                        "filePath".to_string(),
                        serde_json::Value::String(first_path.clone()),
                    );
                    crate::append_trace_log_line(&format!(
                        "[TOOL_ARG_CANONICALIZE] read: paths[0] -> filePath ({})",
                        first_path
                    ));
                }
            }
        }
    } else if tool_name == "exists" {
        if let Some(obj) = args_value.as_object_mut() {
            let has_path = obj
                .get("path")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.trim().is_empty());
            let first_paths_entry = obj
                .get("paths")
                .and_then(|v| v.as_array())
                .and_then(|paths| paths.first())
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string());
            if !has_path {
                if let Some(first_path) = first_paths_entry {
                    obj.insert(
                        "path".to_string(),
                        serde_json::Value::String(first_path.clone()),
                    );
                    crate::append_trace_log_line(&format!(
                        "[TOOL_ARG_CANONICALIZE] exists: paths[0] -> path ({})",
                        first_path
                    ));
                }
            }
        }
    }
    args_value
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

