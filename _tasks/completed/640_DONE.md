# Task 640: UI Renderer Ownership And Legacy Deprecation Decision

**Status:** pending
**Priority:** HIGH
**Type:** Refactor / UI Internals
**Scope:** `src/claude_ui/`, `src/ui/`, `src/ui/mod.rs`, `src/main.rs`
**Source:** old pending task 483, completed duplicate task drift, user request to keep UI architecture tasks pending

## Summary

Decide and enforce which UI renderer modules are canonical, compatibility-only, test-only, or removable after snapshot coverage is available.

## Evidence And Gap

- Task 483 exists both as pending and completed in historical inventory, creating backlog ambiguity.
- Active UI code spans `src/claude_ui/`, `src/ui/ui_terminal.rs`, and many `src/ui/ui_*` modules.
- The old task mentioned stale root-level `src/mod.rs`; current module declarations are in `src/main.rs`.

## Implementation Plan

1. Map every UI module to active runtime, compatibility, modal/widget, or dead/test-only use.
2. Document canonical ownership in a short UI architecture note or module comment.
3. Move compatibility-only code behind test/feature boundaries or delete with user-approved risk.
4. Keep theme tokens centralized in `src/ui/ui_theme.rs`; do not add visual changes.
5. Update module exports to expose only intended UI surfaces.

## Acceptance Criteria

- [ ] No duplicate pending/completed task ambiguity remains for renderer deprecation.
- [ ] Canonical UI entrypoints are documented and tested.
- [ ] Dead UI modules are removed or explicitly retained for fixtures.
- [ ] `cargo check --all-targets` and Task 639 snapshots pass.

## Verification Plan

Run `cargo check --all-targets`, UI snapshot tests, and `rg` for orphan UI module exports.

