use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::interpreter_tools;

pub async fn exec_run_node(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let code = av["code"].as_str().unwrap_or("").to_string();
    if code.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "run_node", "Error: empty code");
    }

    let timeout_seconds = av["timeout_seconds"].as_u64().unwrap_or(30);

    emit_tool_start(&mut tui, "run_node", &code[..code.len().min(50)]);

    match interpreter_tools::execute_code("node", &code, workdir, timeout_seconds, 1000).await {
        Ok((stdout, stderr, exit_code)) => {
            let mut output = String::new();
            if !stdout.is_empty() {
                output.push_str("--- stdout ---\n");
                output.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str("--- stderr ---\n");
                output.push_str(&stderr);
            }
            let ok = exit_code == 0;
            emit_tool_result(&mut tui, "run_node", ok, &output);
            if ok {
                ToolExecutionResult::new_ok(call_id, "run_node", &output)
            } else {
                ToolExecutionResult::new_failed(call_id, "run_node", &output)
            }
        }
        Err(e) => {
            let msg = format!("Node execution error: {}", e);
            emit_tool_result(&mut tui, "run_node", false, &msg);
            ToolExecutionResult::new_failed(call_id, "run_node", &msg)
        }
    }
}
