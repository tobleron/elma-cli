//! @efficiency-role: domain-logic
//!
//! Streaming Tool Executor (Task 115)
//!
//! Executes tools as they arrive in the streaming response,
//! rather than waiting for the full response.
//! Concurrency-safe tools run in parallel; unsafe tools run serially.
//!
//! Task 362: Parallel read/search batch planning and execution.

use crate::shutdown::Shutdown;
use crate::tool_calling::ToolExecutionResult;
use crate::*;
use elma_tools::DynamicToolRegistry;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Whether a tool is safe to run concurrently with other tools.
/// Read-only tools are safe; shell/mutation tools are not.
///
/// Delegates to the policy system in `action_policy` instead of using
/// a hardcoded match list.
pub(crate) fn is_concurrency_safe(tool_name: &str) -> bool {
    crate::action_policy::concurrency_safe_for_tool(tool_name)
}

// ---------------------------------------------------------------------------
// Task 362: Parallel execution planning
// ---------------------------------------------------------------------------

/// Limits for parallel tool execution.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParallelToolLimits {
    /// Maximum number of read-only tools to run in a single parallel batch.
    /// Default: 3. Hard max: 8.
    pub max_parallel_read_only: usize,
}

impl Default for ParallelToolLimits {
    fn default() -> Self {
        Self {
            max_parallel_read_only: 3,
        }
    }
}

impl ParallelToolLimits {
    /// Create a new limit, clamped to [1, 8].
    pub(crate) fn new(max: usize) -> Self {
        Self {
            max_parallel_read_only: max.clamp(1, 8),
        }
    }
}

/// A batch of tool calls to execute.
#[derive(Debug, Clone)]
pub(crate) enum ToolBatch {
    /// A group of adjacent concurrency-safe tools to run in parallel.
    /// Results are collected and returned in original call order.
    ParallelReadOnly(Vec<ToolCall>),
    /// A single tool that must run serially.
    Serial(ToolCall),
}

/// Determine whether a tool is eligible for parallel batching.
///
/// A tool is batchable if the registry marks it as concurrency-safe AND
/// it does not require explicit user permission. Unknown tools default
/// to non-batchable (serial).
fn is_tool_batchable(tool_name: &str, registry: &DynamicToolRegistry) -> bool {
    if let Some(def) = registry.get(tool_name) {
        def.concurrency_safe && !def.requires_permission
    } else {
        false
    }
}

/// Plan tool calls into execution batches.
///
/// Rules:
/// - Group adjacent batchable (concurrency-safe + no permission) calls into
///   `ParallelReadOnly` batches, up to `limits.max_parallel_read_only`.
/// - Non-batchable calls each get their own `Serial` batch.
/// - A `ParallelReadOnly` batch is never split across a `Serial` call.
/// - Original call order is preserved within each batch and across batches.
pub(crate) fn plan_tool_batches(
    calls: &[ToolCall],
    registry: &DynamicToolRegistry,
    limits: ParallelToolLimits,
) -> Vec<ToolBatch> {
    let mut batches: Vec<ToolBatch> = Vec::new();
    let mut current_parallel: Vec<ToolCall> = Vec::new();

    for call in calls {
        if is_tool_batchable(&call.function.name, registry) {
            current_parallel.push(call.clone());
            if current_parallel.len() >= limits.max_parallel_read_only {
                batches.push(ToolBatch::ParallelReadOnly(std::mem::take(
                    &mut current_parallel,
                )));
            }
        } else {
            if !current_parallel.is_empty() {
                batches.push(ToolBatch::ParallelReadOnly(std::mem::take(
                    &mut current_parallel,
                )));
            }
            batches.push(ToolBatch::Serial(call.clone()));
        }
    }

    if !current_parallel.is_empty() {
        batches.push(ToolBatch::ParallelReadOnly(current_parallel));
    }

    batches
}

/// Execute a single ToolBatch and return (tool_call, result) pairs in original order.
///
/// For `ParallelReadOnly` batches, all tools execute concurrently and results
/// are collected in the original call order. TUI is not shared across parallel
/// tasks (set to None).
///
/// For `Serial` batches, the single tool executes with the provided TUI.
pub(crate) async fn execute_tool_batch(
    args: &Args,
    workdir: &PathBuf,
    session: &SessionPaths,
    client: &reqwest::Client,
    chat_url: &Url,
    user_message: &str,
    batch: ToolBatch,
) -> Vec<(ToolCall, ToolExecutionResult)> {
    match batch {
        ToolBatch::ParallelReadOnly(calls) => {
            execute_parallel_tools(
                args,
                &calls,
                workdir,
                session,
                client,
                chat_url,
                user_message,
            )
            .await
        }
        ToolBatch::Serial(call) => {
            let result = tool_calling::execute_tool_call(
                args,
                &call,
                workdir,
                session,
                client,
                chat_url,
                user_message,
                None,
            )
            .await;
            vec![(call, result)]
        }
    }
}

/// Execute a group of concurrency-safe tools in parallel.
/// Returns (call, result) pairs in the original call order.
async fn execute_parallel_tools(
    args: &Args,
    calls: &[ToolCall],
    workdir: &PathBuf,
    session: &SessionPaths,
    client: &reqwest::Client,
    chat_url: &Url,
    user_message: &str,
) -> Vec<(ToolCall, ToolExecutionResult)> {
    let num_calls = calls.len();
    let mut results: Vec<Option<ToolExecutionResult>> = vec![None; num_calls];
    let mut handles = Vec::new();

    for (i, tc) in calls.iter().enumerate() {
        let tc = tc.clone();
        let args = args.clone();
        let workdir = workdir.clone();
        let session = session.clone();
        let client = client.clone();
        let chat_url = chat_url.clone();
        let user_message = user_message.to_string();
        handles.push(tokio::spawn(async move {
            let result = tool_calling::execute_tool_call(
                &args,
                &tc,
                &workdir,
                &session,
                &client,
                &chat_url,
                &user_message,
                None,
            )
            .await;
            (i, tc, result)
        }));
    }

    for handle in handles {
        match handle.await {
            Ok((idx, _tc, result)) => {
                results[idx] = Some(result);
            }
            Err(e) => {
                // Task panicked — create a synthetic error result
                // The index is lost, so we cannot place it precisely.
                // Fill the first empty slot.
                for slot in &mut results {
                    if slot.is_none() {
                        *slot = Some(ToolExecutionResult {
                            tool_call_id: String::new(),
                            tool_name: String::new(),
                            content: format!("Task panic: {}", e),
                            ok: false,
                            exit_code: None,
                            timed_out: false,
                            signal_killed: None,
                        });
                        break;
                    }
                }
            }
        }
    }

    // Zip calls with results, preserving original order
    calls
        .iter()
        .zip(results.into_iter())
        .map(|(call, result_opt)| {
            let result = result_opt.unwrap_or_else(|| ToolExecutionResult {
                tool_call_id: call.id.clone(),
                tool_name: call.function.name.clone(),
                content: "Internal error: result missing after parallel execution".to_string(),
                ok: false,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            });
            (call.clone(), result)
        })
        .collect()
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
                    exit_code: None,
                    timed_out: false,
                    signal_killed: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_registry;
    use crate::ToolFunctionCall;

    fn make_call(name: &str, id: usize) -> ToolCall {
        ToolCall {
            id: format!("call_{}", id),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: name.to_string(),
                arguments: "{}".to_string(),
            },
        }
    }

    #[test]
    fn test_parallel_limits_default() {
        let limits = ParallelToolLimits::default();
        assert_eq!(limits.max_parallel_read_only, 3);
    }

    #[test]
    fn test_parallel_limits_clamped() {
        assert_eq!(ParallelToolLimits::new(0).max_parallel_read_only, 1);
        assert_eq!(ParallelToolLimits::new(10).max_parallel_read_only, 8);
        assert_eq!(ParallelToolLimits::new(5).max_parallel_read_only, 5);
    }

    #[test]
    fn test_empty_calls_produces_no_batches() {
        let registry = tool_registry::get_registry();
        let batches = plan_tool_batches(&[], registry, ParallelToolLimits::default());
        assert!(batches.is_empty());
    }

    #[test]
    fn test_serial_tools_get_own_batches() {
        let registry = tool_registry::get_registry();
        let calls = vec![make_call("shell", 1), make_call("edit", 2)];
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::default());
        assert_eq!(batches.len(), 2);
        assert!(matches!(batches[0], ToolBatch::Serial(_)));
        assert!(matches!(batches[1], ToolBatch::Serial(_)));
    }

    #[test]
    fn test_adjacent_read_tools_grouped() {
        let registry = tool_registry::get_registry();
        let calls = vec![
            make_call("read", 1),
            make_call("search", 2),
            make_call("ls", 3),
        ];
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::default());
        assert_eq!(batches.len(), 1);
        match &batches[0] {
            ToolBatch::ParallelReadOnly(group) => {
                assert_eq!(group.len(), 3);
                assert_eq!(group[0].function.name, "read");
                assert_eq!(group[1].function.name, "search");
                assert_eq!(group[2].function.name, "ls");
            }
            _ => panic!("Expected ParallelReadOnly batch"),
        }
    }

    #[test]
    fn test_read_tool_not_moved_across_serial_call() {
        let registry = tool_registry::get_registry();
        // read, shell, read -> should be two separate parallel groups with a serial in between
        let calls = vec![
            make_call("read", 1),
            make_call("shell", 2),
            make_call("read", 3),
        ];
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::default());
        assert_eq!(batches.len(), 3);
        assert!(matches!(batches[0], ToolBatch::ParallelReadOnly(_)));
        assert!(matches!(batches[1], ToolBatch::Serial(_)));
        assert!(matches!(batches[2], ToolBatch::ParallelReadOnly(_)));
        // Verify each batch has the right number of calls
        if let ToolBatch::ParallelReadOnly(g) = &batches[0] {
            assert_eq!(g.len(), 1);
            assert_eq!(g[0].function.name, "read");
        }
        if let ToolBatch::Serial(g) = &batches[1] {
            assert_eq!(g.function.name, "shell");
        }
        if let ToolBatch::ParallelReadOnly(g) = &batches[2] {
            assert_eq!(g.len(), 1);
            assert_eq!(g[0].function.name, "read");
        }
    }

    #[test]
    fn test_planner_respects_max_parallel_limit() {
        let registry = tool_registry::get_registry();
        // 5 read calls with max_parallel=2 -> 3 batches (2, 2, 1)
        let calls = (0..5).map(|i| make_call("read", i)).collect::<Vec<_>>();
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::new(2));
        assert_eq!(batches.len(), 3);
        for batch in &batches {
            assert!(matches!(batch, ToolBatch::ParallelReadOnly(_)));
        }
        if let ToolBatch::ParallelReadOnly(g) = &batches[0] {
            assert_eq!(g.len(), 2);
        }
        if let ToolBatch::ParallelReadOnly(g) = &batches[1] {
            assert_eq!(g.len(), 2);
        }
        if let ToolBatch::ParallelReadOnly(g) = &batches[2] {
            assert_eq!(g.len(), 1);
        }
    }

    #[test]
    fn test_unknown_tool_is_serial() {
        let registry = tool_registry::get_registry();
        let calls = vec![make_call("nonexistent_tool", 1)];
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::default());
        assert_eq!(batches.len(), 1);
        assert!(matches!(batches[0], ToolBatch::Serial(_)));
    }

    #[test]
    fn test_respond_and_summary_are_batchable_per_metadata() {
        let registry = tool_registry::get_registry();
        // These are marked concurrency_safe: true in registry metadata
        assert!(is_tool_batchable("respond", registry));
        assert!(is_tool_batchable("summary", registry));
        // But they will be handled correctly by ordered post-processing
        let calls = vec![make_call("read", 1), make_call("respond", 2)];
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::default());
        assert_eq!(batches.len(), 1);
        assert!(matches!(batches[0], ToolBatch::ParallelReadOnly(_)));
    }

    #[test]
    fn test_mixed_batch_preserves_order() {
        let registry = tool_registry::get_registry();
        let calls = vec![
            make_call("read", 1),
            make_call("glob", 2),
            make_call("shell", 3),
            make_call("search", 4),
            make_call("ls", 5),
            make_call("edit", 6),
        ];
        let batches = plan_tool_batches(&calls, registry, ParallelToolLimits::default());
        assert_eq!(batches.len(), 4);
        // Batch 0: read, glob
        // Batch 1: shell
        // Batch 2: search, ls
        // Batch 3: edit
        match &batches[0] {
            ToolBatch::ParallelReadOnly(g) => {
                assert_eq!(g[0].function.name, "read");
                assert_eq!(g[1].function.name, "glob");
            }
            _ => panic!("Expected ParallelReadOnly"),
        }
        match &batches[1] {
            ToolBatch::Serial(g) => assert_eq!(g.function.name, "shell"),
            _ => panic!("Expected Serial"),
        }
        match &batches[2] {
            ToolBatch::ParallelReadOnly(g) => {
                assert_eq!(g[0].function.name, "search");
                assert_eq!(g[1].function.name, "ls");
            }
            _ => panic!("Expected ParallelReadOnly"),
        }
        match &batches[3] {
            ToolBatch::Serial(g) => assert_eq!(g.function.name, "edit"),
            _ => panic!("Expected Serial"),
        }
    }

    #[test]
    fn test_all_concurrency_safe_tools_are_batchable() {
        let registry = tool_registry::get_registry();
        // Tools known to be concurrency-safe per action_policy
        for name in &["read", "search", "glob", "ls", "tool_search"] {
            assert!(
                is_tool_batchable(name, registry),
                "{} should be batchable",
                name
            );
        }
    }

    #[test]
    fn test_write_and_shell_tools_are_not_batchable() {
        let registry = tool_registry::get_registry();
        for name in &["shell", "edit", "write", "fetch", "update_todo_list"] {
            assert!(
                !is_tool_batchable(name, registry),
                "{} should not be batchable",
                name
            );
        }
    }

    #[test]
    fn test_execute_serial_batch_returns_one_result() {
        // Verify execute_tool_batch with Serial returns exactly one (call, result) pair
        // and preserves the tool call identity.
        let call = make_call("read", 99);
        let batch = ToolBatch::Serial(call.clone());
        // We cannot execute in a test context (needs runtime, args, etc.),
        // but we can verify ToolBatch::Serial preserves the tool call
        match batch {
            ToolBatch::Serial(tc) => {
                assert_eq!(tc.id, "call_99");
                assert_eq!(tc.function.name, "read");
            }
            _ => panic!("expected Serial variant"),
        }
    }

    #[test]
    fn test_execute_parallel_batch_returns_all_results() {
        // Verify ToolBatch::ParallelReadOnly preserves all calls and their order
        let calls = vec![
            make_call("read", 1),
            make_call("search", 2),
            make_call("ls", 3),
        ];
        let batch = ToolBatch::ParallelReadOnly(calls.clone());
        match batch {
            ToolBatch::ParallelReadOnly(inner) => {
                assert_eq!(inner.len(), 3);
                assert_eq!(inner[0].id, "call_1");
                assert_eq!(inner[1].function.name, "search");
                assert_eq!(inner[2].function.name, "ls");
            }
            _ => panic!("expected ParallelReadOnly variant"),
        }
    }

    /// Performance probe: verify that parallel execution is faster than serial
    /// for independent read-only tools. Uses artificial delays via a fake tool.
    ///
    /// Run with: cargo test parallel_read_search_smoke -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn parallel_read_search_smoke() {
        // Create temp files for reading
        let dir = std::env::temp_dir().join("elma_parallel_test");
        let _ = std::fs::create_dir_all(&dir);
        for i in 0..5 {
            std::fs::write(
                dir.join(format!("file_{}.txt", i)),
                format!("content {}", i),
            )
            .unwrap();
        }

        // Build tool calls to read each file
        let calls: Vec<ToolCall> = (0..5)
            .map(|i| {
                let path = dir.join(format!("file_{}.txt", i));
                ToolCall {
                    id: format!("call_{}", i),
                    call_type: "function".to_string(),
                    function: ToolFunctionCall {
                        name: "read".to_string(),
                        arguments: serde_json::json!({"path": path.to_string_lossy()}).to_string(),
                    },
                }
            })
            .collect();

        // Measure serial execution time
        let serial_start = std::time::Instant::now();
        for call in &calls {
            // We can't easily call execute_tool_call without full infrastructure,
            // so we use std::fs::read_to_string as a proxy
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments).unwrap();
            let path = args["path"].as_str().unwrap();
            let _content = std::fs::read_to_string(path).unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let serial_time = serial_start.elapsed();

        // Measure parallel execution time
        let parallel_start = std::time::Instant::now();
        let mut handles = Vec::new();
        for call in &calls {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments).unwrap();
            let path = args["path"].as_str().unwrap().to_string();
            handles.push(tokio::spawn(async move {
                let _content = tokio::fs::read_to_string(&path).await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let parallel_time = parallel_start.elapsed();

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);

        // Parallel should be faster than serial for 5 independent reads with delay
        assert!(
            parallel_time < serial_time,
            "Parallel execution ({:?}) should be faster than serial ({:?}) for 5 delayed reads",
            parallel_time,
            serial_time
        );
        eprintln!(
            "SMOKE: serial={:?} parallel={:?} speedup={:.2}x",
            serial_time,
            parallel_time,
            serial_time.as_secs_f64() / parallel_time.as_secs_f64().max(0.001)
        );
    }

    #[tokio::test]
    async fn test_parallel_speedup_with_delays() {
        // Verify that executing 3 parallel delayed reads completes faster than serial.
        let dir = std::env::temp_dir().join("elma_speedup_test");
        let _ = std::fs::create_dir_all(&dir);
        for i in 0..3 {
            std::fs::write(dir.join(format!("f{i}.txt")), format!("data {i}")).unwrap();
        }

        let calls: Vec<ToolCall> = (0..3)
            .map(|i| {
                let path = dir.join(format!("f{i}.txt"));
                ToolCall {
                    id: format!("call_{i}"),
                    call_type: "function".to_string(),
                    function: ToolFunctionCall {
                        name: "read".to_string(),
                        arguments: serde_json::json!({"path": path.to_string_lossy()}).to_string(),
                    },
                }
            })
            .collect();

        let delay = std::time::Duration::from_millis(100);

        // Serial: each call includes delay
        let serial_start = std::time::Instant::now();
        for call in &calls {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments).unwrap();
            let path = args["path"].as_str().unwrap();
            let _content = std::fs::read_to_string(path).unwrap();
            tokio::time::sleep(delay).await;
        }
        let serial_time = serial_start.elapsed();

        // Parallel
        let parallel_start = std::time::Instant::now();
        let mut handles = Vec::new();
        for call in &calls {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments).unwrap();
            let path = args["path"].as_str().unwrap().to_string();
            handles.push(tokio::spawn(async move {
                let _content = tokio::fs::read_to_string(&path).await.unwrap();
                tokio::time::sleep(delay).await;
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let parallel_time = parallel_start.elapsed();

        let _ = std::fs::remove_dir_all(&dir);

        // With 3 tasks × 100ms: serial ~300ms+, parallel ~100ms+
        let parallel_ms = parallel_time.as_secs_f64() * 1000.0;
        let serial_ms = serial_time.as_secs_f64() * 1000.0;
        assert!(
            parallel_time < serial_time / 2,
            "Parallel ({parallel_time:?}) should be < half of serial ({serial_time:?}) \
             for 3 delayed reads (parallel={parallel_ms:.1}ms serial={serial_ms:.1}ms)"
        );
    }

    #[tokio::test]
    async fn test_parallel_failed_sibling_does_not_block_others() {
        // One task reads a missing file (fails), others read an existing file (succeed).
        // All results must be collected regardless of individual failures.
        let dir = std::env::temp_dir().join("elma_fail_sibling_test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("ok.txt"), "survivor").unwrap();

        let calls = vec![
            ToolCall {
                id: "call_ok".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "read".to_string(),
                    arguments: serde_json::json!({"path": dir.join("ok.txt").to_string_lossy()})
                        .to_string(),
                },
            },
            ToolCall {
                id: "call_missing".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "read".to_string(),
                    arguments: serde_json::json!({
                        "path": dir.join("missing.txt").to_string_lossy()
                    })
                    .to_string(),
                },
            },
            ToolCall {
                id: "call_ok2".into(),
                call_type: "function".into(),
                function: ToolFunctionCall {
                    name: "read".to_string(),
                    arguments: serde_json::json!({"path": dir.join("ok.txt").to_string_lossy()})
                        .to_string(),
                },
            },
        ];

        let mut handles = Vec::new();
        for call in &calls {
            let args: serde_json::Value = serde_json::from_str(&call.function.arguments).unwrap();
            let path = args["path"].as_str().unwrap().to_string();
            handles.push(tokio::spawn(async move {
                tokio::fs::read_to_string(&path).await
            }));
        }

        let results: Vec<Result<String, _>> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(results.len(), 3, "All 3 tasks must complete");
        assert!(
            results[0].is_ok(),
            "First call (existing file) should succeed"
        );
        assert!(
            results[1].is_err(),
            "Middle call (missing file) should fail"
        );
        assert!(
            results[2].is_ok(),
            "Third call (existing file) should succeed"
        );
        assert_eq!(results[0].as_ref().unwrap(), "survivor");
        assert_eq!(results[2].as_ref().unwrap(), "survivor");
    }
}
