# 138 Add UpdateTodoList Tool

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
Add Todo tool for explicit TODO item management during task execution.

## Reference
- Roo-Code: `~/Roo-Code/src/core/tools/UpdateTodoListTool.ts`

## Implementation

### 1. Define Todo Tool
File: `src/tools/todo.rs` (new)
- Tool name: `update_todo_list`
- Actions: add, remove, complete, list
- Parameters:
  - `action`: "add" | "remove" | "complete" | "list"
  - `content`: todo item text
  - `id`: optional for remove/complete

### 2. Integrate with Tool Registry
File: `src/tool_calling.rs`
- Add `UpdateTodoList` to tool definitions
- Register in `all_tool_definitions()`

### 3. Persistent TODO Storage
File: `src/todo_store.rs` (new)
- Store TODOs in session directory
- Load/save with session
- Share with autonomous loop

## Tool Definition JSON
```json
{
  "name": "update_todo_list",
  "description": "Manage TODO items for task tracking",
  "parameters": {
    "action": {"type": "string", "enum": ["add", "remove", "complete", "list"]},
    "content": {"type": "string"},
    "id": {"type": "string"}
  }
}
```

## Verification
- [ ] `cargo build` passes
- [ ] Tool appears in tool list
- [ ] TODO items persist correctly