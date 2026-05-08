use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_move(
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
        return ToolExecutionResult::new_failed(call_id, "move", &error_msg);
    }

    let src = workdir.join(source);
    let dst = workdir.join(destination);

    if !src.exists() {
        let error_msg = format!("Error: source not found: {}", source);
        emit_tool_result(&mut tui, "move", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "move", &error_msg);
    }

    emit_tool_start(&mut tui, "move", &format!("{} -> {}", source, destination));

    let result = std::fs::rename(&src, &dst);
    let content = match &result {
        Ok(_) => format!("Moved {} to {}", source, destination),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "move", ok, &content);
    
    if ok {
        ToolExecutionResult::new_ok(call_id, "move", &content)
    } else {
        ToolExecutionResult::new_failed(call_id, "move", &content)
    }
}
