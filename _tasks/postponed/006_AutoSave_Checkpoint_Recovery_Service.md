# 143 AutoSave Checkpoint Recovery Service

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
Implement automatic checkpoint saving for crash recovery.

## Reference
- Roo-Code: `~/Roo-Code/src/services/checkpoints/ShadowCheckpointService.ts`

## Implementation

### 1. Checkpoint Service
File: `src/checkpoint.rs` (new)
- `CheckpointService` - auto-save task state
- `save_checkpoint(task_state)` - save current state
- `load_checkpoint()` - load latest checkpoint
- `clear_checkpoints()` - cleanup old checkpoints

### 2. Configuration
File: `src/types_core.rs`
- Add `checkpoint_timeout_seconds: u64` (default: 15)
- Add `enable_checkpoints: bool` (default: true)
- Add to runtime args

### 3. Auto-save Trigger
File: `src/checkpoint.rs`
- Timer-based auto-save every N seconds
- Save on significant events (step completion, tool call)
- Store: messages, current step, artifacts

### 4. Recovery on Restart
File: `src/main.rs`
- Detect existing checkpoint on startup
- Prompt user: "Resume last session?"
- Load checkpoint state if confirmed

### 5. Storage
Directory: `~/.elma-cli/checkpoints/`
- File: `{session_id}_checkpoint.json`
- Keep last 3 checkpoints per session

## Verification
- [ ] `cargo build` passes
- [ ] Checkpoints auto-save during execution
- [ ] Recovery resumes from checkpoint