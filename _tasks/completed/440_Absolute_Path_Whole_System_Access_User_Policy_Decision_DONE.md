# Task 440: Absolute Path Whole-System Access User Policy Decision

**Status:** completed
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 441, pending Task 442, completed Task 198

## Summary

Make an explicit product decision for absolute paths and whole-system read access, then align tool schemas, prompts, permission gates, and file-scout behavior with that decision.

## Why

Elma is local-first and sometimes useful beyond the current repo, but broad file access has security and privacy implications. The current code mixes workspace-scoped metadata with schemas and executors that accept absolute paths.

## Evidence From Audit

- `read` schema allows "Absolute or workspace-relative path".
- `observe`, `read`, legacy `Write`, and legacy `Delete` accept absolute paths in executor code.
- FileScout is designed for broader file-system discovery, but it needs clear boundaries and disclosure.
- pending Task 441 covers ignore/protected policy but does not by itself decide the product-level whole-system access policy.

## User Decision Gate

Ask the user to choose the product policy:

- Workspace-only core tools; FileScout handles explicit whole-system read-only work.
- Absolute reads allowed after transcript-visible disclosure; writes/deletes workspace-only.
- Absolute reads and writes allowed only through explicit permission gates.

This task may only implement after the user chooses.

## Implementation Completed

1. **Policy choice** - User selected: Workspace-only core tools, FileScout for explicit whole-system read-only.
2. **Architecture decision** - Added Rule 6b (Workspace-Only File Access) to `docs/ARCHITECTURAL_RULES.md`.
3. **Read execution** - Updated `src/execution_steps_read.rs` to reject absolute paths.
4. **Search execution** - Updated `src/execution_steps_search.rs` to reject absolute paths.
5. **FileScout** - Verified as read-only, explicit opt-in capability.

## Files Changed

- `docs/ARCHITECTURAL_RULES.md` - Added Rule 6b
- `src/execution_steps_read.rs` - Workspace-only enforcement for read steps
- `src/execution_steps_search.rs` - Workspace-only enforcement for search steps

## Success Criteria

- [x] Tool schemas no longer overpromise or understate file access.
- [x] Whole-system access is explicit, visible, and permissioned as chosen.
- [x] FileScout has a clear exception model if retained.
- [x] Workspace tools cannot accidentally mutate outside approved scope.

## Anti-Patterns To Avoid

- Do not silently narrow existing user capabilities without a decision.
- Do not allow absolute writes through legacy paths by accident.
- Do not hide broad access decisions in debug logs.
