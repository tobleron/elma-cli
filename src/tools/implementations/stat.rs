use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_result};

pub fn exec_stat(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");
    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "stat", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "stat", &error_msg);
    }

    let full_path = workdir.join(path);
    if !full_path.exists() {
        let error_msg = format!("Error: path not found: {}", path);
        emit_tool_result(&mut tui, "stat", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "stat", &error_msg);
    }

    let metadata = match std::fs::metadata(&full_path) {
        Ok(m) => m,
        Err(e) => {
            let error_msg = format!("Error: {}", e);
            emit_tool_result(&mut tui, "stat", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "stat", &error_msg);
        }
    };

    let file_type = if metadata.is_dir() {
        "directory"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };
    let size = metadata.len();
    let modified = metadata
        .modified()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .ok()
        })
        .ok()
        .flatten();

    let content = format!(
        "Type: {}\nSize: {} bytes\nModified: {}",
        file_type,
        size,
        modified
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );

    emit_tool_result(&mut tui, "stat", true, &content);
    ToolExecutionResult::new_ok(call_id, "stat", &content)
}
