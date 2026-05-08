use crate::tools::types::{ToolExecutionResult};

pub fn exec_respond(
    av: &serde_json::Value,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let answer = av["answer"]
        .as_str()
        .or_else(|| av["content"].as_str())
        .or_else(|| av["text"].as_str())
        .map(crate::text_utils::strip_thinking_blocks)
        .unwrap_or_default();
    
    ToolExecutionResult::new_ok(call_id, "respond", &answer)
}
