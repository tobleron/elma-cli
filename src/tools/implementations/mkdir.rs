use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_mkdir(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("");
    let parents = av["parents"].as_bool().unwrap_or(true);

    if path.is_empty() {
        let error_msg = "Error: path required".to_string();
        emit_tool_result(&mut tui, "mkdir", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "mkdir", &error_msg);
    }

    let full_path = workdir.join(path);
    emit_tool_start(&mut tui, "mkdir", path);

    let result = if parents {
        std::fs::create_dir_all(&full_path)
    } else {
        std::fs::create_dir(&full_path)
    };

    let content = match &result {
        Ok(_) => format!("Created directory: {}", path),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "mkdir", ok, &content);
    
    if ok {
        ToolExecutionResult::new_ok(call_id, "mkdir", &content)
    } else {
        ToolExecutionResult::new_failed(call_id, "mkdir", &content)
    }
}
