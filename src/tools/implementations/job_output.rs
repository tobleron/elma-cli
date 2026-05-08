use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::{background_task};

pub async fn exec_job_output(
    av: &serde_json::Value,
    _workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let job_id = av["job_id"].as_str().unwrap_or("").to_string();
    if job_id.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "job_output", "Error: empty job_id");
    }

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            return ToolExecutionResult::new_failed(call_id, "job_output", "Error: TaskManager not initialized");
        }
    };

    emit_tool_start(&mut tui, "job_output", &job_id);

    let task = match task_manager.get_task(&job_id).await {
        Some(t) => t,
        None => {
            let msg = format!("Job not found: {}", job_id);
            emit_tool_result(&mut tui, "job_output", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "job_output", &msg);
        }
    };

    let mut output = String::new();
    output.push_str("--- stdout ---\n");
    for line in &task.stdout_buffer {
        output.push_str(line);
        output.push('\n');
    }
    output.push_str("\n--- stderr ---\n");
    for line in &task.stderr_buffer {
        output.push_str(line);
        output.push('\n');
    }

    emit_tool_result(&mut tui, "job_output", true, &output);
    ToolExecutionResult {
        tool_call_id: call_id.to_string(),
        tool_name: "job_output".to_string(),
        content: output,
        ok: true,
        exit_code: task.exit_code,
        timed_out: false,
        status: crate::tools::ToolStatus::Success,
        duration_ms: 0,
        signal_killed: None,
    }
}
