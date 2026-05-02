# Task 483: UI Renderer And Module Deprecation Decision

**Status:** pending
**Priority:** MEDIUM
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 177, completed Task 331, pending Task 482

## Summary

Decide which UI renderer modules are canonical, which are compatibility layers, and which should be deprecated or moved to tests.

## Evidence From Audit

- Active UI code spans `src/claude_ui/`, `src/ui/ui_terminal.rs`, and many `src/ui/ui_*` modules.
- `src/ui/ui_render_legacy.rs` remains compiled and is over 1000 lines.
- `src/ui/mod.rs` exports the full UI module set.
- root `src/mod.rs` appears to contain stale UI submodule declarations and is not the active crate root.

## User Decision Gate

Ask the user to choose the UI policy:

- Keep legacy renderer until Task 482 screenshots certify parity.
- Move legacy renderer behind a feature/test harness.
- Remove legacy renderer after snapshot coverage is green.

Also ask whether `src/mod.rs` should be deleted immediately if proven orphaned.

## Implementation Plan

1. Map current UI entrypoints and render paths.
2. Prove which modules are used only by tests, legacy compatibility, or active runtime.
3. Present deprecation choices to the user.
4. Apply approved module moves/removals.
5. Add a UI compile/snapshot check that protects the canonical path.

## Success Criteria

- [ ] Canonical UI renderer ownership is documented.
- [ ] Orphan modules are deleted or marked intentionally retained.
- [ ] Task 482 has any needed legacy fixture dependencies documented.
- [ ] `cargo check --all-targets` and UI tests pass.

## Anti-Patterns To Avoid

- Do not remove legacy UI before the user accepts the regression risk.
- Do not hardcode colors outside theme tokens during cleanup.
- Do not edit unrelated rendering behavior while only deprecating modules.
