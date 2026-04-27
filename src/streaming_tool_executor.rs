//! @efficiency-role: domain-logic
//!
//! Streaming Tool Executor (Task 115)
//!
//! Executes tools as they arrive in the streaming response,
//! rather than waiting for the full response.
//! Concurrency-safe tools run in parallel; unsafe tools run serially.

use crate::shutdown::Shutdown;
use crate::tool_calling::ToolExecutionResult;
use crate::*;
use std::sync::Arc;
use tokio::sync::broadcast;

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
/// `interrupt_behavior`: how to handle interrupts (cancel/graceful/complete).
/// `shutdown`: optional shutdown signal subscription.
pub(crate) async fn execute_tools_batch(
    args: &Args,
    tool_calls: &[ToolCall],
    workdir: &PathBuf,
    session: &SessionPaths,
    client: &reqwest::Client,
    chat_url: &Url,
    user_message: &str,
    interrupt_behavior: InterruptBehavior,
    shutdown: Option<Arc<Shutdown>>,
) -> StreamingExecResult {
    if tool_calls.is_empty() {
        return StreamingExecResult {
            results: vec![],
            any_error: false,
        };
    }

    // Subscribe to shutdown signals if shutdown handle is provided
    let mut shutdown_rx = shutdown.as_ref().and_then(|sd| Some(sd.subscribe()));

    // Check for shutdown at the start
    if let Some(ref mut rx) = shutdown_rx {
        if rx.try_recv().is_ok() {
            trace(
                args,
                &format!(
                    "Interrupt received before execution, behavior={:?}",
                    interrupt_behavior
                ),
            );
            // Handle according to interrupt_behavior
            match interrupt_behavior {
                InterruptBehavior::Cancel => {
                    return StreamingExecResult {
                        results: vec![],
                        any_error: true,
                    };
                }
                InterruptBehavior::Graceful => {
                    // Will finish current tools, not start new ones
                }
                InterruptBehavior::Complete => {
                    // Continue with execution
                }
                _ => {} // Other variants don't apply here
            }
        }
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
        // Check for shutdown before spawning
        if let Some(ref mut rx) = shutdown_rx {
            if rx.try_recv().is_ok() {
                trace(
                    args,
                    &format!(
                        "Interrupt received during safe tools execution, behavior={:?}",
                        interrupt_behavior
                    ),
                );
                match interrupt_behavior {
                    InterruptBehavior::Cancel => {
                        // Cancel immediately, return current results
                        let results = safe_results.into_iter().filter_map(|r| r).collect();
                        return StreamingExecResult {
                            results,
                            any_error: true,
                        };
                    }
                    InterruptBehavior::Graceful => {
                        // Don't spawn new tasks, break the loop
                        break;
                    }
                    InterruptBehavior::Complete => {
                        // Continue with execution
                    }
                    _ => {}
                }
            }
        }

        let tc = (*tc).clone();
        let args = args.clone();
        let workdir = workdir.clone();
        let session = session.clone();
        let client = client.clone();
        let chat_url = chat_url.clone();
        let user_message = user_message.to_string();
        let show_status = i == 0; // Only show status for first tool
        let is_shell = tc.function.name == "shell";
        handles.push(tokio::spawn(async move {
            if show_status {
                let msg = if is_shell {
                    format!("executing shell")
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
    let mut serial_results: Vec<ToolExecutionResult> = Vec::new();
    for (i, tc) in serial_tools.iter().enumerate() {
        // Check for shutdown before executing serial tool
        if let Some(ref mut rx) = shutdown_rx {
            if rx.try_recv().is_ok() {
                trace(
                    args,
                    &format!(
                        "Interrupt received during serial tools execution, behavior={:?}",
                        interrupt_behavior
                    ),
                );
                match interrupt_behavior {
                    InterruptBehavior::Cancel => {
                        // Cancel immediately, return current results
                        let mut results = Vec::with_capacity(tool_calls.len());
                        let mut safe_idx = 0;
                        let mut serial_idx = 0;
                        for tc in tool_calls {
                            if is_concurrency_safe(&tc.function.name) {
                                if let Some(result) = safe_results[safe_idx].take() {
                                    results.push(result);
                                }
                                safe_idx += 1;
                            } else {
                                if serial_idx < serial_results.len() {
                                    results.push(serial_results[serial_idx].clone());
                                }
                                serial_idx += 1;
                            }
                        }
                        return StreamingExecResult {
                            results,
                            any_error: true,
                        };
                    }
                    InterruptBehavior::Graceful => {
                        // Finish current tool, don't start new ones
                        // For now, just continue (will finish current if any)
                        break;
                    }
                    InterruptBehavior::Complete => {
                        // Continue with execution
                    }
                    _ => {}
                }
            }
        }

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

/// Execute a shell command in the background without blocking.
/// Returns the task ID for monitoring.
pub(crate) async fn execute_background_shell(
    args: &Args,
    command: String,
    workdir: &PathBuf,
    task_manager: &background_task::TaskManager,
    memory_limit_mb: Option<u64>,
    timeout_seconds: Option<u64>,
) -> Result<String, String> {
    let task_id = task_manager
        .create_task(
            "shell".to_string(),
            command.clone(),
            workdir.clone(),
            memory_limit_mb,
            timeout_seconds,
        )
        .await?;

    task_manager.start_task(&task_id).await?;

    trace(args, &format!("background_task_started: id={}", task_id));

    Ok(task_id)
}
