use std::path::{Path, PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_result};

pub fn exec_file_size(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "file_size", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "file_size", &error_msg);
    }

    let full_path = workdir.join(path);

    if !full_path.exists() {
        let error_msg = format!("Error: path not found: {}", path);
        emit_tool_result(&mut tui, "file_size", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "file_size", &error_msg);
    }

    fn dir_size(p: &Path) -> u64 {
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
    ToolExecutionResult::new_ok(call_id, "file_size", &content)
}
