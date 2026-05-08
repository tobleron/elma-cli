use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_result};

pub fn exec_exists(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let paths: Vec<String> = if let Some(arr) = av["paths"].as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.trim().is_empty())
            .collect()
    } else {
        av["path"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default()
    };
    let check_type = av["type"].as_str().unwrap_or("any");

    if paths.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "exists", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "exists", &error_msg);
    }

    let mut rows = Vec::new();
    for path in paths {
        let full_path = workdir.join(&path);
        let exists = full_path.exists();
        let row = if !exists {
            format!("{}: exists: false", path)
        } else {
            let actual_type = if full_path.is_dir() {
                "dir"
            } else if full_path.is_file() {
                "file"
            } else {
                "other"
            };
            let matches = check_type == "any" || check_type == actual_type;
            format!(
                "{}: exists: true, type: {}, matches: {}",
                path, actual_type, matches
            )
        };
        rows.push(row);
    }
    let content = rows.join("\n");

    emit_tool_result(&mut tui, "exists", true, &content);
    ToolExecutionResult::new_ok(call_id, "exists", &content)
}
