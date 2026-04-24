# 137 Task Persistence And Resumability

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
Enable task history storage and task resumption from checkpoints.

## Reference
- Roo-Code: `~/Roo-Code/src/core/task-persistence/TaskHistoryStore.ts`

## Implementation

### 1. Create Task History Store
File: `src/task_history.rs` (new)
- `TaskHistoryStore` - persist tasks to disk
- `save_task(task)` - save to history
- `load_task(id)` - load from history
- `list_tasks()` - list all tasks
- `delete_task(id)` - remove task

### 2. Add Task Resumption
File: `src/task.rs`
- `Task::resume(history_item)` - resume from history
- Capture full state (messages, steps, artifacts)

### 3. Add `/resume` Command
File: `src/commands.rs`
- `/resume` - list resumable tasks
- `/resume <id>` - resume specific task

### 4. Storage Format
Directory: `~/.elma-cli/task_history/`
- File per task: `{task_id}.json`
- Index file: `index.json`

## Verification
- [ ] Tasks persist to disk
- [ ] Tasks resume correctly
- [ ] `/resume` command works