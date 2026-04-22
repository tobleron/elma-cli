# Task 106: System & Terminal Notifications

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
Alert the user when long-running tasks complete or require attention, even if they've switched to another terminal tab or window.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Notification Library**:
    - Use the `notify-rust` crate to trigger OS-level notifications (macOS, Linux, Windows).
2. **Detection of "Long-Running" Tasks**:
    - Implement a `TaskDurationTracker` in `src/metrics.rs`.
    - If a task (e.g., an LLM query, a deep search) takes longer than a threshold (e.g., 10 seconds), mark it for notification upon completion.
3. **UI/Terminal Notifications**:
    - Implement a `send_ui_notification(message: &str)` in `src/ui.rs`.
    - Support two types:
        - OS-level: Popup bubble via `notify-rust`.
        - Terminal Bell: Send `\x07` (ASCII BEL) to the terminal if configured.
4. **Integration**:
    - Hook into `src/orchestration_loop.rs` to trigger a notification when the final answer is ready for a long task.
5. **Configuration**:
    - Add `enable_notifications: bool` and `notification_threshold_secs: u64` to `profiles.toml`.

### Proposed Rust Dependencies
- `notify-rust = "4.10"`: Industry-standard for cross-platform desktop notifications.

### Verification Strategy
1. **Behavior**: 
    - Trigger a long-running search and switch to another app; confirm a notification popup appears when finished.
    - Confirm the terminal bell rings if enabled in a quiet environment.
2. **Platform Compatibility**:
    - Verify behavior on macOS and confirm it respects "Do Not Disturb" settings.
3. **Safety**:
    - Ensure it fails gracefully if the notification daemon is missing or fails.
