use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, emit_tool_progress};
use crate::program_utils::resolve_tool_path;

pub fn exec_read(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let paths: Vec<String> = if let Some(arr) = av["paths"].as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        let single = av["path"]
            .as_str()
            .or_else(|| av["filePath"].as_str())
            .unwrap_or("")
            .to_string();
        if single.is_empty() {
            Vec::new()
        } else {
            vec![single]
        }
    };

    if paths.is_empty() {
        let error_msg = "Error: no path or paths provided".to_string();
        emit_tool_result(&mut tui, "read", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "read", &error_msg);
    }

    let is_multi = paths.len() > 1;
    let mut all_content = String::new();
    let mut errors: Vec<String> = Vec::new();

    for (i, tp) in paths.iter().enumerate() {
        let full = match resolve_tool_path(workdir, tp) {
            Ok(p) => p,
            Err(e) => {
                let err = format!("path error: {}", e);
                if is_multi {
                    errors.push(err.clone());
                    all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                    continue;
                } else {
                    emit_tool_result(&mut tui, "read", false, &err);
                    return ToolExecutionResult::new_failed(call_id, "read", &err);
                }
            }
        };
        if !full.exists() {
            let err = format!("file_not_found: {}", tp);
            if is_multi {
                errors.push(err.clone());
                all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                continue;
            } else {
                emit_tool_result(&mut tui, "read", false, &err);
                return ToolExecutionResult::new_failed(call_id, "read", &err);
            }
        }

        match crate::document_adapter::read_file_smart(&full) {
            Ok((content, header)) => {
                let file_block = if is_multi {
                    format!("### File {}: {}\n{}\n\n{}", i + 1, tp, header, content)
                } else {
                    format!("{}\n{}", header, content)
                };
                all_content.push_str(&file_block);
                if i < paths.len() - 1 {
                    all_content.push_str("\n\n");
                }
            }
            Err(e) => {
                let err = format!("Error reading {}: {}", tp, e);
                if is_multi {
                    errors.push(err.clone());
                    all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                } else {
                    emit_tool_result(&mut tui, "read", false, &err);
                    return ToolExecutionResult::new_failed(call_id, "read", &err);
                }
            }
        }
    }

    let ok = errors.is_empty();
    emit_tool_start(&mut tui, "read", &paths[0]);
    emit_tool_progress(&mut tui, "read", "reading file(s)");
    emit_tool_result(&mut tui, "read", ok, &all_content);

    if ok {
        ToolExecutionResult::new_ok(call_id, "read", &all_content)
    } else {
        ToolExecutionResult::new_failed(call_id, "read", &all_content)
    }
}
