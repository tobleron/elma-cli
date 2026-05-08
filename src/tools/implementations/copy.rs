use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_copy(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let source = av["source"].as_str().unwrap_or("");
    let destination = av["destination"].as_str().unwrap_or("");

    if source.is_empty() || destination.is_empty() {
        let error_msg = "Error: source and destination required".to_string();
        emit_tool_result(&mut tui, "copy", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "copy", &error_msg);
    }

    let src = workdir.join(source);
    let dst = workdir.join(destination);

    if !src.exists() {
        let error_msg = format!("Error: source not found: {}", source);
        emit_tool_result(&mut tui, "copy", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "copy", &error_msg);
    }

    emit_tool_start(&mut tui, "copy", &format!("{} -> {}", source, destination));

    fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_recursive(&entry.path().to_path_buf(), &dest_path)?;
            } else {
                std::fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }

    let result = if src.is_dir() {
        copy_dir_recursive(&src, &dst)
    } else {
        std::fs::copy(&src, &dst).map(|_| ())
    };

    let content = match &result {
        Ok(_) => format!("Copied {} to {}", source, destination),
        Err(e) => format!("Error: {}", e),
    };

    let ok = result.is_ok();
    emit_tool_result(&mut tui, "copy", ok, &content);
    
    if ok {
        ToolExecutionResult::new_ok(call_id, "copy", &content)
    } else {
        ToolExecutionResult::new_failed(call_id, "copy", &content)
    }
}
