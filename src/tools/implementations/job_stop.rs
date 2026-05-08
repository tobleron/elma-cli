use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::{background_task};

pub async fn exec_job_stop(
    av: &serde_json::Value,
    _workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let job_id = av["job_id"].as_str().unwrap_or("").to_string();
    if job_id.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "job_stop", "Error: empty job_id");
    }

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult::new_failed(call_id, "job_stop", "Error: TaskManager not initialized");
        }
    };

    emit_tool_start(&mut tui, "job_stop", &job_id);

    if let Err(e) = task_manager.cancel_task(&job_id).await {
        let msg = format!("Failed to stop job: {}", e);
        emit_tool_result(&mut tui, "job_stop", false, &msg);
        return ToolExecutionResult::new_failed(call_id, "job_stop", &msg);
    }

    let content = format!("Job {} stopped", job_id);
    emit_tool_result(&mut tui, "job_stop", true, &content);
    ToolExecutionResult::new_ok(call_id, "job_stop", &content)
}
