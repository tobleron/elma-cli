use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_trash(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "trash", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "trash", &error_msg);
    }

    let full_path = workdir.join(path);

    if !full_path.exists() {
        let error_msg = format!("Error: path not found: {}", path);
        emit_tool_result(&mut tui, "trash", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "trash", &error_msg);
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
    
    if ok {
        ToolExecutionResult::new_ok(call_id, "trash", &content)
    } else {
        ToolExecutionResult::new_failed(call_id, "trash", &content)
    }
}
