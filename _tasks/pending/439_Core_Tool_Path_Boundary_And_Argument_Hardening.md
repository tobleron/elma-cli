# Task 439: Core Tool Path Boundary And Argument Hardening

**Status:** pending
**Priority:** CRITICAL
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 396, pending Task 432, completed Task 340, completed Task 325

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

## Implementation Plan

1. Define one path resolver that canonicalizes paths, handles symlinks, and applies the selected policy.
2. Replace ad hoc path resolution in core tool and legacy step handlers.
3. Rewrite shell-backed search to argv-safe process execution or a Rust-native search path.
4. Add tests for absolute paths, `..`, symlinks, protected paths, and quote injection patterns.
5. Surface blocked path decisions as transcript rows, not only trace logs.

## Success Criteria

- [ ] Core tools consistently enforce the selected path policy.
- [ ] Search patterns containing quotes cannot escape into shell syntax.
- [ ] Writes/deletes cannot mutate outside the approved scope.
- [ ] Whole-system access, if retained, is explicit and visible.
- [ ] Tests cover read, observe, search, write, and delete.

## Anti-Patterns To Avoid

- Do not rely on string prefix checks after canonicalization is needed.
- Do not weaken file-scout behavior without a user-approved replacement.
- Do not use shell quoting as the primary defense when argv APIs are available.
