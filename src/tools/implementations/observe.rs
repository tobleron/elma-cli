use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::program_utils::resolve_tool_path;

pub fn exec_observe(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let path = av["path"].as_str().unwrap_or("").to_string();
    if path.is_empty() {
        let error_msg = "Error: empty path".to_string();
        emit_tool_result(&mut tui, "observe", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "observe", &error_msg);
    }

    let full = match resolve_tool_path(workdir, &path) {
        Ok(p) => p,
        Err(e) => {
            let error_msg = format!("path error: {}", e);
            emit_tool_result(&mut tui, "observe", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "observe", &error_msg);
        }
    };

    emit_tool_start(&mut tui, "observe", &path);

    let md = match std::fs::symlink_metadata(&full) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let content = format!("path: {}\nexists: false", full.display());
            emit_tool_result(&mut tui, "observe", true, &content);
            return ToolExecutionResult::new_ok(call_id, "observe", &content);
        }
        Err(e) => {
            let error_msg = format!("Error inspecting {}: {}", full.display(), e);
            emit_tool_result(&mut tui, "observe", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "observe", &error_msg);
        }
    };

    let file_type_str = if md.file_type().is_symlink() {
        "symlink"
    } else if md.file_type().is_dir() {
        "directory"
    } else if md.file_type().is_file() {
        "file"
    } else {
        "other"
    };

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("path: {}", full.display()));
    lines.push(format!("exists: true"));
    lines.push(format!("type: {}", file_type_str));
    lines.push(format!("size: {}", md.len()));
    if let Ok(mtime) = md.modified() {
        match mtime.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => lines.push(format!("modified: {}", d.as_secs())),
            Err(_) => {}
        }
    }
    #[cfg(unix)]
    lines.push(format!(
        "permissions: {:o}",
        std::os::unix::fs::MetadataExt::mode(&md) & 0o777
    ));
    #[cfg(not(unix))]
    lines.push(format!("permissions: {:?}", md.permissions()));
    lines.push(format!("readonly: {}", md.permissions().readonly()));

    // Symlink target
    if md.file_type().is_symlink() {
        match std::fs::read_link(&full) {
            Ok(target) => {
                lines.push(format!("symlink_target: {}", target.display()));
            }
            Err(_) => {
                lines.push("symlink_target: <unreadable>".to_string());
            }
        }
    }

    // Directory child count
    if file_type_str == "directory" {
        match std::fs::read_dir(&full) {
            Ok(entries) => {
                let count = entries.filter_map(|e| e.ok()).count();
                lines.push(format!("child_count: {}", count));
            }
            Err(_) => {}
        }
    }

    let content = lines.join("\n");
    emit_tool_result(&mut tui, "observe", true, &content);
    ToolExecutionResult::new_ok(call_id, "observe", &content)
}
