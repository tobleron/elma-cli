# Task 482: Terminal UI Regression Capture Harness

**Status:** pending
**Source patterns:** Qwen-code terminal capture tests, Claude Code terminal polish, Crush TUI architecture
**Depends on:** completed Task 338 (event log); keybinding customization is optional

## Summary

Add a deterministic terminal capture harness for UI regression tests across common terminal sizes, streaming states, transcript rows, command palettes, diffs, and status/footer rendering.

## Why

Elma's terminal UI is a major product surface and several UI modules are large. Reference agents use terminal capture and golden-output tests to prevent regressions in wrapping, streaming, scrolling, and controls.

## Implementation Plan

1. Add a test harness that renders known UI states into terminal snapshots.
2. Cover narrow, standard, and wide terminal dimensions.
3. Include streaming assistant text, tool rows, collapsible operational rows, diffs, search, and footer.
4. Assert the footer contains only model name, token count, and elapsed time.
5. Keep snapshots stable and easy to review.

## Success Criteria

- [ ] UI snapshots cover at least three terminal sizes.
- [ ] Streaming and final answer states are both tested.
- [ ] Operational rows render without overlapping main content.
- [ ] Footer rule is enforced by a regression test.
- [ ] Snapshot update workflow is documented.

## Anti-Patterns To Avoid

- Do not make tests depend on live provider output.
- Do not accept snapshots with overlapping or truncated controls.
- Do not add footer content to satisfy operational visibility.
