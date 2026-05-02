# Task 456: Absolute Path Whole-System Access User Policy Decision

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 396, pending Task 439, completed Task 198

## Summary

Make an explicit product decision for absolute paths and whole-system read access, then align tool schemas, prompts, permission gates, and file-scout behavior with that decision.

## Why

Elma is local-first and sometimes useful beyond the current repo, but broad file access has security and privacy implications. The current code mixes workspace-scoped metadata with schemas and executors that accept absolute paths.

## Evidence From Audit

- `read` schema allows "Absolute or workspace-relative path".
- `observe`, `read`, legacy `Write`, and legacy `Delete` accept absolute paths in executor code.
- FileScout is designed for broader file-system discovery, but it needs clear boundaries and disclosure.
- pending Task 396 covers ignore/protected policy but does not by itself decide the product-level whole-system access policy.

## User Decision Gate

Ask the user to choose the product policy:

- Workspace-only core tools; FileScout handles explicit whole-system read-only work.
- Absolute reads allowed after transcript-visible disclosure; writes/deletes workspace-only.
- Absolute reads and writes allowed only through explicit permission gates.

This task may only implement after the user chooses.

## Implementation Plan

1. Write the selected policy as a short architecture decision.
2. Update tool schemas to match actual allowed paths.
3. Align executor behavior and permission gates with the policy.
4. Add tests for absolute reads, absolute writes, symlink escape, and FileScout exceptions.
5. Update user-facing error/disclosure messages.

## Success Criteria

- [ ] Tool schemas no longer overpromise or understate file access.
- [ ] Whole-system access is explicit, visible, and permissioned as chosen.
- [ ] FileScout has a clear exception model if retained.
- [ ] Workspace tools cannot accidentally mutate outside approved scope.

## Anti-Patterns To Avoid

- Do not silently narrow existing user capabilities without a decision.
- Do not allow absolute writes through legacy paths by accident.
- Do not hide broad access decisions in debug logs.
