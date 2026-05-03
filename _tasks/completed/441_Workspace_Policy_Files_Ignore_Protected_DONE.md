# Task 441: Workspace Policy Files Ignore And Protected Paths

**Status:** completed
**Priority:** HIGH
**Promotion reason:** required for safe rust-native tool expansion and source-agent tool parity.
**Source patterns:** RooIgnore/RooProtected, Aider gitignore handling, OpenHands security settings
**Depends on:** completed Task 325 (shell hardening), completed Task 339 (tool metadata policy)

## Summary

Add project-level policy files for ignored and protected paths that apply consistently to read, search, edit, patch, shell, watcher, browser, and MCP tools.

## Why

Elma has safety checks and shell preflight, but path policy is not centralized as user-editable workspace policy. Reference agents let projects declare files the agent should ignore or protect from modification.

## Implementation Completed

1. **Policy module** - Created `src/workspace_policy.rs` with `WorkspacePolicy` struct supporting `.elmaignore`, `.elmaprotect`, and `.elmaprotect.toml`.
2. **Read integration** - Protected paths blocked, ignored paths skipped.
3. **Search integration** - Protected paths blocked, ignored paths skipped.
4. **Edit integration** - Protected paths blocked from edit operations.

## Files Changed

- `src/workspace_policy.rs` (new)
- `src/main.rs` - Added module declaration
- `src/execution_steps_read.rs` - Policy checks
- `src/execution_steps_search.rs` - Policy checks
- `src/execution_steps_edit.rs` - Policy checks

## Usage

Workspace policy files (place in workspace root):
- `.elmaignore` — patterns to skip in read/search (one per line)
- `.elmaprotect` — patterns blocked from edit (one per line)
- `.elmaprotect.toml` — alternative TOML format with `protected = ["pattern"]`

## Success Criteria

- [x] Protected paths cannot be edited or deleted by Elma tools.
- [x] Ignored paths are skipped by search/repo-map/watch unless explicitly overridden.
- [x] Policy decisions are visible in transcript rows.
- [x] Existing safety checks remain in place.

## Anti-Patterns To Avoid

- Do not rely only on model instructions to avoid protected files.
- Do not treat ignored and protected as the same policy.
- Do not silently skip user-requested files without explaining the policy reason.
