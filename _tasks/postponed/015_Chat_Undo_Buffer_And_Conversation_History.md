# 148 Chat Undo Buffer And Conversation History

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
Implement undo functionality for chat messages and conversation history navigation.

## Reference
- Claude Code: `useInputBuffer` for chat:undo functionality

## Implementation

### 1. Undo Buffer
File: `src/undo_buffer.rs` (new)
```rust
pub struct UndoBuffer {
    messages: Vec<ChatMessage>,
    current_index: usize,
}

impl UndoBuffer {
    pub fn push(&mut self, message: ChatMessage) { ... }
    pub fn undo(&mut self) -> Option<ChatMessage> { ... }
    pub fn redo(&mut self) -> Option<ChatMessage> { ... }
    pub fn can_undo(&self) -> bool { ... }
    pub fn can_redo(&self) -> bool { ... }
}
```

### 2. Undo Manager
File: `src/undo_manager.rs` (new)
- Track messages in current session
- Store undo/redo stacks
- Persist history to session file

### 3. History Navigation
File: `src/history.rs` (new)
- `up_arrow` - previous in history
- `down_arrow` - next in history
- Search mode: type to filter
- Timestamps and session markers

### 4. Commands
File: `src/commands.rs`
- `/undo` - undo last message
- `/redo` - redo undone message
- `/history` - show session history

## Verification
- [ ] `cargo build` passes
- [ ] Undo/redo works correctly
- [ ] History navigation works