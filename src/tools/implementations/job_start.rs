use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::{execution_profiles, background_task};

pub async fn exec_job_start(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let command = av["command"].as_str().unwrap_or("").to_string();
    if command.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "job_start", "Error: empty command");
    }

    // Task 460: Check execution profile for command restrictions
    if let Some(profile) = execution_profiles::get_execution_profile() {
        if !execution_profiles::is_command_allowed(profile, &command) {
            let msg = format!(
                "Job start blocked by execution profile '{}': command not allowed",
                profile.name
            );
            emit_tool_result(&mut tui, "job_start", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "job_start", &msg);
        }
    }

    let name = av["name"].as_str().unwrap_or("").to_string();
    let memory_limit_mb = av["memory_limit_mb"].as_u64();
    let timeout_seconds = av["timeout_seconds"].as_u64();

    let task_manager = match background_task::get_task_manager() {
        Some(tm) => tm.clone(),
        None => {
            let error_msg = "Error: TaskManager not initialized";
            return ToolExecutionResult::new_failed(call_id, "job_start", error_msg);
        }
    };

    emit_tool_start(&mut tui, "job_start", &command);

    let id = match task_manager
        .create_task(
            if name.is_empty() {
                "background_job".to_string()
            } else {
                name
            },
            command.clone(),
            workdir.clone(),
            memory_limit_mb,
            timeout_seconds,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            let msg = format!("Failed to create task: {}", e);
            emit_tool_result(&mut tui, "job_start", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "job_start", &msg);
        }
    };

    if let Err(e) = task_manager.start_task(&id).await {
        let msg = format!("Failed to start task: {}", e);
        emit_tool_result(&mut tui, "job_start", false, &msg);
        return ToolExecutionResult::new_failed(call_id, "job_start", &msg);
    }

    let content = format!("Job started with ID: {}", id);
    emit_tool_result(&mut tui, "job_start", true, &content);
    ToolExecutionResult::new_ok(call_id, "job_start", &content)
}
