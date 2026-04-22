# 135 Mode Manager And Switching Logic

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
Implement mode switching capability allowing users to switch between modes during conversation.

## Reference
- Roo-Code: `~/Roo-Code/src/core/config/CustomModesManager.ts`

## Implementation

### 1. Create Mode Manager
File: `src/mode_manager.rs` (new)
- `ModeManager` struct with current_mode, custom_modes
- `switch_mode(new_mode: Mode)` - switches to new mode
- `get_current_mode()` - returns active mode
- `register_custom_mode(name, tools, system_prompt)` - for custom modes

### 2. Add Mode Switching to Session
File: `src/session.rs`
- Store `current_mode: Mode` in `Session`
- Add `switch_mode` method to session
- Include mode in session metadata

### 3. Add `/mode` Command
File: `src/commands.rs`
- `/mode code` - switch to code mode
- `/mode ask` - switch to ask mode
- `/mode list` - list available modes

## Verification
- [ ] `cargo build` passes
- [ ] Mode switching persists in session
- [ ] `/mode` command works