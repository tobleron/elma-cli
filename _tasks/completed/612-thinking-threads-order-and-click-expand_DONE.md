# Task 612: Thinking threads order (oldest→newest) & click-to-expand

## Type

Bug

## Severity

High

## Scope

UI (right panel thinking threads)

## Session Evidence

From the user's direct report: thinking threads on the right panel pile up incorrectly (newest last should be at bottom, not top), and clicking on them does not toggle expand/collapse.

Current code at `src/claude_ui/claude_render.rs:1306-1311`:
```rust
let all_thinking: Vec<&ThinkingEntry> = self
    .thinking_entries
    .iter()
    .rev()       // ← reversed: newest FIRST
    .collect();
```

The entries are collected newest-first. When rendered at lines 1797-1807, they display top-to-bottom as newest→oldest. The user wants oldest→newest (first thread at top, last thread at bottom).

Additionally, the right panel thinking area needs hit-testing (mouse click detection) to toggle expand/collapse. Currently no mouse handling exists for `render_right_panel_thinking`. The left panel handles clicks via `hit_test` in `claude_render.rs`, but the right panel thinking area does not.

## Problem

1. Thinking threads display newest-first, but should display oldest-first (chronological order, newest at bottom)
2. Mouse clicks on thinking entries in the right panel do nothing — no expand/collapse toggle

## Root Cause Hypothesis

**Confirmed:**
1. `all_thinking` collection uses `.rev()` which reverses the natural (chronological) order
2. Right panel thinking rendering (`render_right_panel_thinking`) has no mouse event handling — `hit_test` and click handling only exist for the left panel transcript

## Proposed Solution

### Part A: Fix thread order
In `claude_render.rs`, remove `.rev()` from the thinking collection at line 1310:
```rust
let all_thinking: Vec<&ThinkingEntry> = self
    .thinking_entries
    .iter()
    .collect();  // chronological order: oldest first
```

Auto-scroll to bottom when a new entry is added (during streaming), so the live thread is always visible at the bottom.

### Part B: Add click-to-expand in right panel
Add hit-testing in `render_right_panel_thinking` that maps mouse click coordinates to specific thinking entries. Store the expanded/collapsed state per entry (already exists via `collapsed` field). On click, toggle the entry's `collapsed` flag and re-render expanded content.

For an expanded entry, show all content lines (not just the first line) in the right panel. When collapsed, show only the first line.

Files to change:
- `src/claude_ui/claude_render.rs` — `render_right_panel_thinking` + hit testing + mouse event handling
- `src/claude_ui/claude_render.rs` — `all_thinking` collection (remove `.rev()`)

## Acceptance Criteria

- [ ] Thinking threads display oldest at top, newest at bottom
- [ ] During live streaming, the view scrolls to show the newest (bottom) thread
- [ ] Clicking a collapsed thinking entry in the right panel expands it (shows all content)
- [ ] Clicking an expanded entry collapses it (shows only first line)
- [ ] The expand/collapse toggle works independently for each thread

## Verification Plan

- Unit test: verify `all_thinking` order matches chronological insertion order
- Manual test: start a complex task, observe thinking threads piling up oldest→newest
- Manual test: click on a thinking thread, verify it expands/collapses

## Dependencies

- Task 614 (scrollbar) — complementary fix for right panel interaction

## Notes

The right panel currently shows only the first line of each completed thinking entry. When expanded, it should show the full content. This requires storing the full `content` string (already stored) and rendering multiple lines when `collapsed` is false.
