# Task 162: Delegate Tool (Subagent Architecture)

## Summary

Implement delegated subagent execution - spawn child agent instances with isolated context, restricted toolsets, and their own terminal sessions. Supports single-task and parallel batch modes.

## Motivation

Complex tasks benefit from decomposition:
- Parent breaks task into subtasks
- Children execute with focused context
- Parent aggregates results

## Source

Hermes `tools/delegate_tool.py`

## Implementation

### Core Types

```rust
struct DelegationConfig {
    goal: String,           // What the subagent should do
    context: Option<String>, // Additional context
    toolsets: Vec<String>,   // Allowed toolsets
    max_iterations: u32,     // Max turns per child
    workspace_path: Option<String>,  // Custom working dir
}

struct DelegateResult {
    success: bool,
    summary: String,
    error: Option<String>,
    iterations_used: u32,
}
```

### Tool Interface

```rust
// Single delegation
fn delegate_task(config: DelegationConfig) -> DelegateResult

// Batch (parallel)
fn delegate_batch(tasks: Vec<DelegationConfig>, max_concurrent: u32) -> Vec<DelegateResult>
```

### Child Agent Properties

Each child gets:
- Fresh conversation (no parent history)
- Own task_id (own terminal session, file ops cache)
- Restricted toolset (configurable, blocked tools always stripped)
- Focused system prompt from goal + context
- MAX_DEPTH = 2 (no recursive delegation)

### Blocked Tools for Children

```rust
const DELEGATE_BLOCKED_TOOLS = frozenset([
    "delegate_task",   // No recursive delegation
    "clarify",       // No user interaction
    "memory",       // No writes to shared memory
    "send_message", // No cross-platform side effects
    "execute_code", // Children should reason step-by-step
])
```

### Parent Display

The parent's context only sees:
- Delegation call start
- Summary result

Never sees child's intermediate tool calls or reasoning.

## Verification

- Single delegation works
- Parallel batch works
- Blocked tools properly stripped
- Max depth enforcement works

## Dependencies

- Existing shell/file tools
- Session management

## Notes

- Similar to Roo-Code's Orchestrator mode
- Key difference: isolated context, not shared history