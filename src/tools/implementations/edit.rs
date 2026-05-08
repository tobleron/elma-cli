use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, verify_syntax};
use crate::program_utils::resolve_tool_path;

pub fn exec_edit(
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
        return ToolExecutionResult::new_failed(call_id, "edit", &error_msg);
    }

    let full = match resolve_tool_path(workdir, &path) {
        Ok(p) => p,
        Err(e) => {
            let error_msg = format!("path error: {}", e);
            emit_tool_result(&mut tui, "edit", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "edit", &error_msg);
        }
    };

    emit_tool_start(&mut tui, "edit", &path);

    let content = match std::fs::read_to_string(&full) {
        Ok(c) => c,
        Err(e) => {
            let error_msg = format!("Error reading file: {}", e);
            emit_tool_result(&mut tui, "edit", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "edit", &error_msg);
        }
    };

    if old_string.is_empty() {
        let error_msg = "old_string is required".to_string();
        emit_tool_result(&mut tui, "edit", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "edit", &error_msg);
    }

    if let Some(pos) = content.find(&old_string) {
        let mut updated = content.clone();
        updated.replace_range(pos..pos + old_string.len(), &new_string);
        if let Err(e) = std::fs::write(&full, &updated) {
            let error_msg = format!("Error writing file: {}", e);
            emit_tool_result(&mut tui, "edit", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "edit", &error_msg);
        }
        // Task 543: Verify syntax
        if let Err(e) = verify_syntax(&path, workdir) {
            emit_tool_result(&mut tui, "edit", false, &e);
            return ToolExecutionResult::new_failed(call_id, "edit", &e);
        }
        emit_tool_result(&mut tui, "edit", true, "edited");
        ToolExecutionResult::new_ok(call_id, "edit", "edited")
    } else {
        let error_msg = "old_string not found in file".to_string();
        emit_tool_result(&mut tui, "edit", false, &error_msg);
        ToolExecutionResult::new_failed(call_id, "edit", &error_msg)
    }
}
