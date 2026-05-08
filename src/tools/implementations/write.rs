use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, verify_syntax};
use crate::program_utils::resolve_tool_path;

pub fn exec_write(
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
        return ToolExecutionResult::new_failed(call_id, "write", &error_msg);
    }

    let full = match resolve_tool_path(workdir, &path) {
        Ok(p) => p,
        Err(e) => {
            let error_msg = format!("path error: {}", e);
            emit_tool_result(&mut tui, "write", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "write", &error_msg);
        }
    };

    emit_tool_start(&mut tui, "write", &path);

    if let Some(parent) = full.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            let error_msg = format!("Error creating directory: {}", e);
            emit_tool_result(&mut tui, "write", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "write", &error_msg);
        }
    }

    match std::fs::write(&full, &content) {
        Ok(_) => {
            // Task 543: Verify syntax
            if let Err(e) = verify_syntax(&path, workdir) {
                emit_tool_result(&mut tui, "write", false, &e);
                return ToolExecutionResult::new_failed(call_id, "write", &e);
            }
            emit_tool_result(&mut tui, "write", true, "written");
            ToolExecutionResult::new_ok(call_id, "write", "written")
        }
        Err(e) => {
            let error_msg = format!("Error writing file: {}", e);
            emit_tool_result(&mut tui, "write", false, &error_msg);
            ToolExecutionResult::new_failed(call_id, "write", &error_msg)
        }
    }
}
