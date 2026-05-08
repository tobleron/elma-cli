use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_touch(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "touch", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "touch", &error_msg);
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
    
    if ok {
        ToolExecutionResult::new_ok(call_id, "touch", &content)
    } else {
        ToolExecutionResult::new_failed(call_id, "touch", &content)
    }
}
