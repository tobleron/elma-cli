# Task 442: Core Tool Path Boundary And Argument Hardening

**Status:** completed
**Priority:** CRITICAL
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 441, pending Task 446, completed Task 340, completed Task 325

## Summary

Harden core read/search/observe/write/delete paths and shell-backed argument construction so local tools obey an explicit workspace and whole-system policy.

## Evidence From Audit

- `exec_read` and `exec_observe` accept absolute paths directly.
- legacy `Step::Write` and `Step::Delete` handlers accept absolute paths directly.
- `exec_search` builds a shell command with single-quoted user-controlled `pattern` and `path` values instead of using argv-safe `Command` args.
- `handle_search_step` already uses `std::process::Command`, showing a safer pattern exists in the codebase.
- `read` tool schema says "Absolute or workspace-relative path", but metadata marks the tool workspace-scoped.

## User Decision Gate

Ask the user to choose the intended access model:

- Workspace-only by default, with explicit whole-system file-scout mode.
- Whole-system reads allowed, but writes/deletes workspace-only.
- Current absolute-path behavior retained, but made transcript-visible and permission-gated.

Document the selected policy before implementation.

## Implementation Completed

1. **exec_read** - Rejects absolute paths, uses policy check.
2. **exec_observe** - Rejects absolute paths.
3. **handle_write_step** - Rejects absolute paths, uses policy check.
4. **handle_delete_step** - Rejects absolute paths, uses policy check.
5. **exec_search** - Removed shell injection risk by using argv-safe `std::process::Command` instead of string formatting.

## Files Changed

- `src/tool_calling.rs` - exec_read, exec_observe, exec_search hardened
- `src/execution_steps.rs` - handle_write_step, handle_delete_step hardened
- `src/execution_steps_read.rs` - Already hardened in Task 441
- `src/execution_steps_search.rs` - Already hardened in Task 441
- `src/execution_steps_edit.rs` - Already hardened in Task 441

## Shell Argument Hardening

Search previously built shell command via string formatting:
```
"rg -i --line-number '{}' '{}'"
```
Now uses argv-safe process construction:
```
cmd.arg("-i").arg("--line-number").arg(&pattern);
cmd.arg(&search_path);
```

## Success Criteria

- [x] Core tools consistently enforce workspace-only path policy.
- [x] Search patterns containing quotes cannot escape into shell syntax.
- [x] Writes/deletes cannot mutate outside the approved scope.
- [x] Whole-system access is explicit and visible (via FileScout).

## Policy Applied

This task implements the workspace-only policy from Task 440 across all core tools.

## Anti-Patterns To Avoid

- Do not rely on string prefix checks after canonicalization is needed.
- Do not weaken file-scout behavior without a user-approved replacement.
- Do not use shell quoting as the primary defense when argv APIs are available.
