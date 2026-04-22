# 141 Add ExecuteCommand WithIO Tool

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
Add enhanced command execution tool with input/output streaming.

## Reference
- Roo-Code: `~/Roo-Code/src/core/tools/ExecuteCommandTool.ts`

## Implementation

### 1. Enhance Shell Tool
File: `src/tools/shell.rs`
- Add `UseIO` variant for command with input
- Stream input to command stdin
- Capture stdout and stderr separately
- Add timeout parameter

### 2. New Tool Definition
File: `src/tool_calling.rs`
- Modify `shell` tool to support:
  - `command`: shell command
  - `description`: what command does
  - `input`: optional stdin input
  - `timeout_seconds`: execution timeout

### 3. Handle in Execution
File: `src/execution_steps.rs`
- Extend `ShellStep` with input field
- Pass input to command stdin
- Capture both output streams

## Verification
- [ ] `cargo build` passes
- [ ] Commands with stdin work
- [ ] Timeout enforcement works