# Task 639: Terminal UI Regression Capture Harness

**Status:** pending
**Priority:** HIGH
**Type:** Test Coverage / UI Internals
**Scope:** `src/claude_ui/`, `src/ui/`, `_testing_reports/`, scenario fixtures
**Source:** old pending task 482

## Summary

Build a deterministic terminal UI capture harness before deleting or moving renderer modules.

## Evidence And Gap

- Historical Task 482 asked for terminal UI regression capture.
- Legacy and current UI modules still coexist across `src/claude_ui/` and `src/ui/`.
- Renderer deprecation decisions need objective coverage rather than visual intuition.

## Implementation Plan

1. Add terminal frame fixtures for startup, user input, streaming assistant, tool lifecycle, permission prompt, search/model modal, thinking panel, compaction row, and narrow width.
2. Render fixtures without requiring a real TTY.
3. Store text/frame snapshots in a stable format with sanitized timing and spinner fields.
4. Add a helper to compare canonical rows rather than raw ANSI noise where appropriate.

## Acceptance Criteria

- [ ] UI snapshots run in CI/headless mode.
- [ ] Fixtures cover the prompt, transcript, right panel, footer, modal, and tool rows.
- [ ] Snapshot updates require intentional review.
- [ ] The harness can certify module removal in Task 640.

## Verification Plan

Run `cargo test ui_snapshot` and inspect generated fixture diffs for stable, meaningful changes.

