# 136 Task Lifecycle Management With Events

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
Enhance Program/Step abstraction with explicit Task class and event emissions for better lifecycle management.

## Reference
- Roo-Code: `~/Roo-Code/src/core/task/Task.ts`

## Implementation

### 1. Create Task Struct
File: `src/task.rs` (new)
```rust
pub struct Task {
    pub id: String,
    pub objective: String,
    pub status: TaskStatus,
    pub steps: Vec<Step>,
    pub created_at: DateTime,
    pub events: EventEmitter<TaskEvent>,
}

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Aborted,
    Paused,
}

pub enum TaskEvent {
    Created,
    Started,
    Completed,
    Aborted,
    Paused,
    Resumed,
    StepStarted(usize),
    StepCompleted(usize),
}
```

### 2. Add Event Emitter Pattern
File: `src/task_events.rs` (new)
- Simple event emitter trait for Task events
- Subscribe/unsubscribe handlers

### 3. Integrate with Orchestration
File: `src/orchestration_core.rs`
- Replace implicit Program execution with explicit Task
- Emit events during execution
- Track task state through lifecycle

## Verification
- [ ] `cargo build` passes
- [ ] Task events emit correctly
- [ ] Task status transitions work