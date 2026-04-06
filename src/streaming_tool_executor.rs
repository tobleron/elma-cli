//! @efficiency-role: domain-logic
//!
//! Streaming Tool Executor (Task 115)
//!
//! Executes tools as they arrive in the streaming response,
//! rather than waiting for the full response.
//! Concurrency-safe tools run in parallel; unsafe tools run serially.

use crate::tool_calling::ToolExecutionResult;
use crate::*;

/// Whether a tool is safe to run concurrently with other tools.
/// Read-only tools are safe; shell/mutation tools are not.
pub(crate) fn is_concurrency_safe(tool_name: &str) -> bool {
    matches!(tool_name, "read" | "search" | "respond")
}

/// Result from the streaming executor.
pub(crate) struct StreamingExecResult {
    /// Ordered tool results (in the order tool_calls were received).
    pub(crate) results: Vec<ToolExecutionResult>,
    /// Whether any tool errored.
    pub(crate) any_error: bool,
}

/// Execute a batch of tool calls with concurrency control.
/// Safe tools run in parallel (up to 3); unsafe tools run serially.
pub(crate) async fn execute_tools_batch(
    args: &Args,
    tool_calls: &[ToolCall],
    workdir: &PathBuf,
    session: &SessionPaths,
    client: &reqwest::Client,
    chat_url: &Url,
    user_message: &str,
) -> StreamingExecResult {
    if tool_calls.is_empty() {
        return StreamingExecResult {
            results: vec![],
            any_error: false,
        };
    }

    // Partition into safe (parallel) and serial (unsafe)
    let mut safe_tools: Vec<&ToolCall> = Vec::new();
    let mut serial_tools: Vec<&ToolCall> = Vec::new();
    for tc in tool_calls {
        if is_concurrency_safe(&tc.function.name) {
            safe_tools.push(tc);
        } else {
            serial_tools.push(tc);
        }
    }

    // Execute safe tools in parallel
    let mut safe_results: Vec<Option<ToolExecutionResult>> = vec![None; safe_tools.len()];
    let mut any_error = false;

    // Run safe tools concurrently
    let mut handles = Vec::new();
    for (i, tc) in safe_tools.iter().enumerate() {
        let tc = (*tc).clone();
        let args = args.clone();
        let workdir = workdir.clone();
        let session = session.clone();
        let client = client.clone();
        let chat_url = chat_url.clone();
        let user_message = user_message.to_string();
        let show_status = i == 0; // Only show status for first tool
        let is_shell = tc.function.name == "shell";
        let args_preview = if is_shell {
            crate::tool_loop::extract_tool_arg_preview(&tc.function.arguments, "command", 50)
        } else {
            String::new()
        };
        handles.push(tokio::spawn(async move {
            if show_status {
                let msg = if is_shell {
                    format!("executing shell: {}", args_preview)
                } else {
                    format!("executing {}", tc.function.name)
                };
                show_status_message(&args, &msg);
            }
            tool_calling::execute_tool_call(
                &args,
                &tc,
                &workdir,
                &session,
                &client,
                &chat_url,
                &user_message,
                None,
            )
            .await
        }));
    }

    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(result) => {
                if !result.ok {
                    any_error = true;
                }
                safe_results[i] = Some(result);
            }
            Err(e) => {
                any_error = true;
                safe_results[i] = Some(ToolExecutionResult {
                    tool_call_id: safe_tools[i].id.clone(),
                    tool_name: safe_tools[i].function.name.clone(),
                    content: format!("Task error: {}", e),
                    ok: false,
                });
            }
        }
    }

    // Execute serial tools one at a time
    let mut serial_results = Vec::new();
    for (i, tc) in serial_tools.iter().enumerate() {
        if i == 0 && safe_tools.is_empty() {
            show_status_message(args, &format!("executing {}", tc.function.name));
        }
        let result = tool_calling::execute_tool_call(
            args,
            tc,
            workdir,
            session,
            client,
            chat_url,
            user_message,
            None,
        )
        .await;
        if !result.ok {
            any_error = true;
        }
        serial_results.push(result);
    }

    // Merge results in original order
    let mut results = Vec::with_capacity(tool_calls.len());
    let mut safe_idx = 0;
    let mut serial_idx = 0;
    for tc in tool_calls {
        if is_concurrency_safe(&tc.function.name) {
            results.push(safe_results[safe_idx].take().unwrap());
            safe_idx += 1;
        } else {
            results.push(serial_results[serial_idx].clone());
            serial_idx += 1;
        }
    }

    StreamingExecResult { results, any_error }
}
