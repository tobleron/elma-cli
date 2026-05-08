use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::repo_map;

pub async fn exec_repo_map(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let token_budget = av["token_budget"].as_u64().unwrap_or(2000) as usize;
    let max_files = av["max_files"].as_u64().unwrap_or(50) as usize;

    emit_tool_start(&mut tui, "repo_map", "building repo map");

    let (output, _tokens_used) = repo_map::build_repo_map(workdir, token_budget, max_files);

    let content = format!("{}", output);

    emit_tool_result(&mut tui, "repo_map", true, &content);
    ToolExecutionResult::new_ok(call_id, "repo_map", &content)
}
