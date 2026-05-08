use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::{background_task};

pub async fn exec_job_status(
    av: &serde_json::Value,
    _workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let job_id = av["job_id"].as_str().unwrap_or("").to_string();
    if job_id.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "job_status", "Error: empty job_id");
    }

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult::new_failed(call_id, "job_status", "Error: TaskManager not initialized");
        }
    };

    emit_tool_start(&mut tui, "job_status", &job_id);

    let task = match task_manager.get_task(&job_id).await {
        Some(t) => t,
        None => {
            let msg = format!("Job not found: {}", job_id);
            emit_tool_result(&mut tui, "job_status", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "job_status", &msg);
        }
    };

    let runtime = task.runtime_seconds().unwrap_or(0);
    let content = format!(
        "Job ID: {}\nName: {}\nStatus: {}\nExit code: {}\nRuntime: {}s\nMemory: {}MB",
        task.id,
        task.name,
        task.status,
        task.exit_code.map_or("N/A".to_string(), |c| c.to_string()),
        runtime,
        task.memory_usage_mb
    );

    emit_tool_result(&mut tui, "job_status", true, &content);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "job_status".to_string(),
        content,
        ok: true,
        exit_code: task.exit_code,
        timed_out: false,
        status: crate::tools::ToolStatus::Success,
        duration_ms: 0,
        signal_killed: None,
    }
}
