# 188: Recursive File Picker Workspace Discovery

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

## Status
Pending

## Priority
Low

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Make the `@` file picker recursively discover files in the workspace, not just the top-level directory.

## Current State
In `claude_render.rs`, `discover_workspace_files()` only reads the top-level directory.

## Target Behavior
Claude Code's file picker (`QuickOpenDialog`) uses `ripgrep` or similar to recursively find files, respecting `.gitignore`.

## Implementation Notes

### Suggested Approach
1. Use `walkdir` or `ignore` crate (already in dependency tree via `similar` or other deps)
2. Walk the workspace recursively
3. Respect `.gitignore` patterns
4. Skip common directories: `target/`, `node_modules/`, `.git/`, `.idea/`, etc.
5. Collect file paths relative to workspace root
6. Limit total files to avoid memory issues (e.g., 10,000 files max)

### Filtering
- Skip hidden files/directories (starting with `.`)
- Skip known build/output directories
- Skip binary files (check extensions or MIME type)
- Respect `.gitignore`

### Performance
- Cache the file list and refresh periodically (e.g., on `@` activation)
- Use async discovery if the workspace is large
- Show a "scanning..." indicator while discovering

## Files Likely Touched
- `src/claude_ui/claude_render.rs` — `discover_workspace_files()`
- `Cargo.toml` — may need to add `ignore` or `walkdir` dependency

## Verification
- [ ] PTY fixture: `@` picker shows nested files
- [ ] PTY fixture: `@` picker respects `.gitignore`
- [ ] PTY fixture: `@` picker skips `target/` and `node_modules/`
- [ ] `cargo test --test ui_parity`

## Related Tasks
- Task 166 (master plan)
- Task 173 (file mentions)

---
*Created: 2026-04-22*
