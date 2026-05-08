use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, verify_syntax};
use elma_tools::{PatchOperation, parse_patch};

pub fn exec_patch(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let patch_content = av["patch"].as_str().unwrap_or("").to_string();
    if patch_content.is_empty() {
        let error_msg = "Error: patch content is empty";
        emit_tool_result(&mut tui, "patch", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "patch", error_msg);
    }

    emit_tool_start(&mut tui, "patch", "(multi-file patch)");

    match parse_patch(&patch_content) {
        Ok(parsed) => {
            let mut results = Vec::new();
            let mut all_ok = true;

            for op in &parsed.operations {
                let (path, result_msg) = match op {
                    PatchOperation::AddFile { path, content } => {
                        let full = workdir.join(path);
                        match std::fs::create_dir_all(full.parent().unwrap_or(&full)) {
                            Ok(_) => match std::fs::write(&full, content) {
                                Ok(_) => (path.clone(), "added".to_string()),
                                Err(e) => {
                                    all_ok = false;
                                    (path.clone(), format!("write failed: {}", e))
                                }
                            },
                            Err(e) => {
                                all_ok = false;
                                (path.clone(), format!("dir create failed: {}", e))
                            }
                        }
                    }
                    PatchOperation::DeleteFile { path } => {
                        let full = workdir.join(path);
                        match std::fs::remove_file(&full) {
                            Ok(_) => (path.clone(), "deleted".to_string()),
                            Err(e) => {
                                all_ok = false;
                                (path.clone(), format!("delete failed: {}", e))
                            }
                        }
                    }
                    PatchOperation::UpdateFile {
                        path,
                        old_string,
                        new_string,
                    } => {
                        let full = workdir.join(path);
                        match std::fs::read_to_string(&full) {
                            Ok(original) => {
                                if let Some(pos) = original.find(old_string) {
                                    let mut updated = original.clone();
                                    updated.replace_range(pos..pos + old_string.len(), new_string);
                                    match std::fs::write(&full, &updated) {
                                        Ok(_) => (path.clone(), "updated".to_string()),
                                        Err(e) => {
                                            all_ok = false;
                                            (path.clone(), format!("write failed: {}", e))
                                        }
                                    }
                                } else {
                                    all_ok = false;
                                    (path.clone(), "old_string not found".to_string())
                                }
                            }
                            Err(e) => {
                                all_ok = false;
                                (path.clone(), format!("read failed: {}", e))
                            }
                        }
                    }
                };
                results.push(format!("{}: {}", path, result_msg));
            }

            // Task 543: Verify syntax if any Rust files were touched
            for op in &parsed.operations {
                let p = match op {
                    PatchOperation::AddFile { path, .. } => path,
                    PatchOperation::UpdateFile { path, .. } => path,
                    PatchOperation::DeleteFile { .. } => continue,
                };
                if let Err(e) = verify_syntax(p, workdir) {
                    all_ok = false;
                    results.push(format!("Verification failed: {}", e));
                    break;
                }
            }

            let output = results.join("\n");
            emit_tool_result(&mut tui, "patch", all_ok, &output);
            if all_ok {
                ToolExecutionResult::new_ok(call_id, "patch", &output)
            } else {
                ToolExecutionResult::new_failed(call_id, "patch", &output)
            }
        }
        Err(e) => {
            let error_msg = format!("Error parsing patch: {}", e);
            emit_tool_result(&mut tui, "patch", false, &error_msg);
            ToolExecutionResult::new_failed(call_id, "patch", &error_msg)
        }
    }
}
