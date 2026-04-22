# 139 Add AttemptCompletion Tool

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary
Add tool for signaling task completion and reporting results.

## Reference
- Roo-Code: `~/Roo-Code/src/core/tools/AttemptCompletionTool.ts`

## Implementation

### 1. Define AttemptCompletion Tool
File: `src/tools/attempt_completion.rs` (new)
- Tool name: `attempt_completion`
- Parameters:
  - `result`: completion message
  - `rejected`: whether completion was rejected

### 2. Integrate with Tool Registry
File: `src/tool_calling.rs`
- Add `AttemptCompletion` to tool definitions

### 3. Handle in Autonomous Loop
File: `src/orchestration_loop.rs`
- Detect `attempt_completion` tool call
- Signal completion to agent loop
- Trigger reviewer for final review

## Tool Definition JSON
```json
{
  "name": "attempt_completion",
  "description": "Signal task completion with results",
  "parameters": {
    "result": {"type": "string"},
    "rejected": {"type": "boolean"}
  }
}
```

## Verification
- [ ] `cargo build` passes
- [ ] Tool appears in tool list
- [ ] Completion signals agent loop correctly