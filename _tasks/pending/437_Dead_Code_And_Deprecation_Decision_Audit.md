# Task 437: Dead Code And Deprecation Decision Audit

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** postponed Task 064, completed Task 177, completed Task 381

## Summary

Audit unused, orphaned, legacy, and misleading modules, then ask the user which pieces should be removed, deprecated, moved to tests, or kept for planned future work.

## Evidence From Audit

- `src/mod.rs` exists at crate root but this crate is a binary rooted at `src/main.rs`; it appears to contain stale UI submodule declarations rather than live root module wiring.
- `src/ui/ui_render_legacy.rs` is still compiled through `src/ui/mod.rs` while the active renderer path is centered around `src/claude_ui/` and `src/ui/ui_terminal.rs`.
- `TerminalUI::push_stop_notice` is present with no callers, already noted by deferred Task 304.
- Several compatibility and legacy paths remain after completed slim-session, prompt, and tool-calling work.

## User Decision Gate

Before changing code, present the user with a dead-code report grouped by:

- Safe delete candidates.
- Deprecate-with-warning candidates.
- Keep because a pending task depends on them.
- Move-to-test-fixture candidates.

Ask the user to approve the disposition of each group. Do not delete modules only because they have low search hits.

## Implementation Plan

1. Generate a module reachability inventory from `src/main.rs`, nested `mod.rs` files, tests, and public re-exports.
2. Cross-check each suspected dead item against `_tasks/pending/`, `_tasks/postponed/`, docs, and tests.
3. Produce a deprecation map with user choices.
4. Remove or mark only approved items.
5. Add a lightweight dead-code audit check that catches orphan root files like `src/mod.rs`.

## Success Criteria

- [ ] Dead-code report lists evidence and proposed disposition.
- [ ] User approval is recorded before any removal.
- [ ] Removed code is not referenced by pending tasks without task updates.
- [ ] `cargo check --all-targets` passes.
- [ ] `cargo clippy --all-targets` is not made worse.

## Anti-Patterns To Avoid

- Do not remove `src/prompt_core.rs` or alter `TOOL_CALLING_SYSTEM_PROMPT`.
- Do not delete legacy code that is still needed as a migration fallback without documenting the migration cutoff.
- Do not use raw line-count or filename heuristics as the sole deletion basis.
