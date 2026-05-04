# Task 614: Right panel scrollbar doesn't respond to mouse

## Type

Bug

## Severity

High

## Scope

UI (right panel / thinking area scrollbar)

## Session Evidence

From the user's direct report: clicking on the right side panel to scroll up or down, the scrollbar does not move. The main (left) window scrollbar works correctly.

Current code at `src/claude_ui/claude_render.rs:1818-1857`:
```rust
let total_lines = all_lines.len();
let area_height = area.height.saturating_sub(0) as usize;

if total_lines > area_height {
    let max_scroll = total_lines.saturating_sub(area_height);
    if *scroll > max_scroll {
        *scroll = max_scroll;
    }
    // ... render visible slice + scrollbar
}
```

The scrollbar IS rendered (line 1835: `ScrollbarState::new(max_scroll)`) but mouse events are not routed to the thinking area. The main panel's mouse handling at `claude_render.rs` has keyboard shortcuts (PageUp/PageDown, j/k, etc.) and click events, but the thinking area has no input handling.

## Problem

The right panel thinking section renders a scrollbar but mouse wheel events and click-drag on the scrollbar are not captured/processed for this area. Only the main (left) transcript area receives scroll events.

## Root Cause Hypothesis

**Confirmed:** Mouse event handling in `claude_render.rs` only maps clicks to the left panel transcript area. The thinking area's coordinates are not checked in the mouse event handler. Additionally, the `thinking_scroll` field at line 85 exists but is only updated programmatically, never by user input.

## Proposed Solution

1. Add mouse event handling for the right panel thinking area:
   - Track the pixel coordinates of the thinking area (stored during rendering)
   - In the mouse event handler, check if click is within thinking area bounds
   - Map scroll wheel events in the thinking area to `thinking_scroll` changes
   - Map click-drag on the scrollbar thumb to scroll position changes

2. Store `thinking_area_rect` as a field on the renderer so mouse events can be mapped:
```rust
// claude_render.rs
struct ClaudeRenderer {
    // ...
    thinking_area_rect: Option<Rect>,  // stored from last render
}
```

Files to change:
- `src/claude_ui/claude_render.rs` — add right panel mouse handling
- `src/claude_ui/claude_render.rs` — store thinking_area coordinates

## Acceptance Criteria

- [ ] Mouse wheel scroll in the right panel thinking area scrolls the thinking list
- [ ] Click-drag on the thinking scrollbar thumb scrolls the content
- [ ] Scroll position is preserved between renders
- [ ] Left panel scroll continues to work independently of right panel scroll

## Verification Plan

- Manual test: generate enough thinking threads to overflow the right panel, scroll with mouse wheel, verify content moves
- Manual test: click-drag scrollbar thumb, verify content scrolls
- Manual test: verify left panel scrolling is unaffected

## Dependencies

- Task 612 (click-to-expand thinking threads) — both require mouse event routing to the thinking area

## Notes

The scrollbar rendering already exists and is functional. The missing piece is input routing. The mouse event handler needs to check which area (left transcript vs. right thinking vs. other) the mouse is in, and route scroll events accordingly.
