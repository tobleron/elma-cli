# Task 100: Interactive Task Progress Tree

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

## Objective
Visualize the hierarchical execution of agents, sub-tasks, and tools using an animated progress tree.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Tree Data Structure**: 
    - Create a `TaskNode` struct in `src/ui_state.rs` representing an active task/agent.
    - Fields: `name`, `status` (Active, Done, Error), `children` (nested tasks), `is_last` (for drawing).
2. **Recursive Rendering**:
    - Implement a `render_task_tree(root: &TaskNode, depth: usize)` function in `src/ui.rs`.
    - Use branching characters: `\u2514\u2500` (last child) and `\u251c\u2500` (intermediate child).
3. **Animated State Updates**:
    - As `src/orchestration_loop.rs` spawns new sub-tasks or calls tools, push them into the tree.
    - Re-render the tree section of the terminal (using `crossterm` cursor jumps) to show status changes without printing new lines for every update.
4. **Integration**:
    - Update `src/orchestration_core.rs` to notify the UI when a task starts, completes, or fails.

### Proposed Rust Dependencies
- Use `crossterm` for in-place re-renders.

### Verification Strategy
1. **Stress Test**: Verify the tree correctly renders nested tool calls (e.g., an agent calling a shell tool which calls another tool).
2. **Behavior**: 
    - Verify symbols correctly change from "Active" (spinner) to "Done" (check/dot).
    - Verify large trees do not exceed terminal height.
