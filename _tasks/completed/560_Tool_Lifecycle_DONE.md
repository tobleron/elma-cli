# 560 — Centralize Tool Execution Lifecycle with Pre/Post Hooks

- **Priority**: High
- **Category**: Architecture
- **Depends on**: 552 (split tool_calling.rs)
- **Blocks**: None

## Problem Statement

Tool execution currently has ad-hoc lifecycle management spread across multiple files:

- **Pre-execution** (shell only): Preflight in `shell_preflight.rs`, permission gate in `permission_gate.rs`, budget check in `command_budget.rs`, hooks in `hook_system.rs`
- **Execution**: Tool-specific executors in `tool_calling.rs` (soon `tools/` directory)
- **Post-execution**: Evidence ledger entry in `tool_loop.rs:1282-1331`, session flush in `session_flush.rs`, event log in `event_log.rs`, hook post-execution in `hook_system.rs`

Only the `shell` tool gets the full lifecycle. Other mutating tools (`edit`, `write`, `patch`, `move`, `trash`) bypass preflight, permission gate, and hooks entirely. Read-only tools have no lifecycle management at all.

Each tool executor is responsible for some lifecycle steps (e.g., `exec_shell` calls preflight, permission gate, budget, hooks) but the pattern is inconsistent and error-prone.

## Why This Matters for Small Local LLMs

Small models cause more tool failures. A centralized lifecycle means:
- Every tool gets pre-execution validation (not just shell)
- Failed tool calls get consistent error formatting for the model
- Post-execution evidence collection happens reliably (not just in tool_loop)
- Retry logic is consistent across all tools

## Current Behavior

| Lifecycle Step | shell | edit | write | patch | read | search | glob | ls |
|----------------|-------|------|-------|-------|------|--------|------|----|
| Preflight validation | Yes | No | No | No | No | No | No | No |
| Permission gate | Yes | No | No | No | No | No | No | No |
| Budget check | Yes | No | No | No | No | No | No | No |
| Pre-hooks | Yes | No | No | No | No | No | No | No |
| Snapshot (risky) | Yes | No | No | No | No | No | No | No |
| Evidence ledger | Yes (tool_loop) | Yes (tool_loop) | Yes (tool_loop) | Yes (tool_loop) | Yes (tool_loop) | Yes (tool_loop) | No | No |
| Post-hooks | Yes | No | No | No | No | No | No | No |
| Session flush | Yes | Yes | Yes | Yes | Yes | Yes | ? | ? |

## Recommended Target Behavior

Create a `ToolLifecycle` struct that wraps tool execution:

```rust
pub struct ToolLifecycle {
    preflight: PreflightPipeline,
    hooks: HookRegistry,
    evidence: EvidenceIntegrator,
    events: EventRecorder,
}

impl ToolLifecycle {
    pub async fn execute(
        &mut self,
        call: &ToolCall,
        args: &Args,
        workdir: &PathBuf,
        session: &SessionPaths,
        client: &reqwest::Client,
        tui: Option<&mut TerminalUI>,
    ) -> ToolExecutionResult {
        // 1. Pre-execution: validate arguments, check permissions, run pre-hooks
        // 2. Execution: delegate to tool-specific executor
        // 3. Post-execution: record evidence, flush session, run post-hooks, log events
    }
}
```

## Source Files That Need Modification

- `src/tool_calling.rs` → `src/tools/mod.rs` — Replace `execute_tool_call` with lifecycle-managed execution
- `src/tool_loop.rs` — Remove evidence ledger integration from tool loop, delegate to lifecycle
- `src/shell_preflight.rs` — Generalize preflight to work with non-shell tools (path validation)
- `src/permission_gate.rs` — Generalize to accept any tool call, not just shell commands
- `src/hook_system.rs` — Generalize to all tool types
- `src/command_budget.rs` — Generalize budget to cover all mutating operations

## New Files/Modules

- `src/tool_lifecycle.rs` — `ToolLifecycle` struct with pre/post execution pipeline
- `src/tool_lifecycle_hooks.rs` — Hook execution orchestration

## Step-by-Step Implementation Plan

1. Analyze the complete lifecycle for each tool type (what steps apply to what tools)
2. Create `ToolLifecycle` struct with configurable pipeline stages
3. Define trait `ToolLifecycleStage`:
   ```rust
   trait ToolLifecycleStage: Send + Sync {
       fn execute(&self, ctx: &ToolExecutionContext) -> Result<(), ToolLifecycleError>;
   }
   ```
4. Implement stages: `ArgValidationStage`, `PermissionGateStage`, `PreflightStage`, `SnapshotStage`, `BudgetCheckStage`, `EvidenceStage`, `SessionFlushStage`, `EventLogStage`, `PostHookStage`
5. Configure per-tool pipeline in tool registry (which stages apply to which tool)
6. Replace `execute_tool_call` dispatch with lifecycle-managed execution
7. Remove lifecycle logic from individual tool executors (executors become pure execution functions)
8. Remove lifecycle logic from `tool_loop.rs`
9. Run full test suite

## Recommended Crates

- `async-trait` — already a dependency, for async lifecycle stages

## Validation/Sanitization Strategy

- Each lifecycle stage must be independently testable
- Pipeline ordering is enforced at compile time (builder pattern)
- Failed stages halt the pipeline and return structured errors
- All stages log their decisions to trace output

## Testing Plan

1. Unit test each lifecycle stage independently
2. Integration test: full pipeline for shell, edit, read tools
3. Test that failed preflight prevents execution
4. Test that failed permission gate prevents execution
5. Test that evidence is recorded for all evidence-collecting tools
6. Test that hooks fire for all tool types that support them
7. Test that snapshot is created before risky operations

## Acceptance Criteria

- All tool executions pass through the centralized lifecycle
- Permission gate applies to `edit`, `write`, `patch`, `move`, `trash` (not just shell)
- Evidence is recorded for all evidence-collecting tools (not just shell/read/search)
- Hooks can be registered for any tool type
- Existing tool behavior is unchanged
- No lifecycle logic remains in individual tool executors

## Risks and Migration Notes

- **Breaking change**: Moving permission gate to all mutating tools will cause more permission prompts. Mitigate with a migration flag or gradual rollout.
- **Performance**: Adding lifecycle stages to every tool call adds overhead. Ensure stages are lightweight (most are no-ops for read-only tools).
- **Shell tool complexity**: The shell executor currently implements its own lifecycle. Extract this carefully to avoid breaking the preflight→permission→budget→hooks chain.
- Pair with Task 552 (split tool_calling.rs) to add lifecycle during the split rather than before/after.
