# Task 615: Elma logo — smaller minimalist bold-stroke redesign

## Type

Refactor (UI)

## Severity

Medium

## Scope

UI (right panel logo)

## Session Evidence

From the user's direct report: the logo on the right panel is cropped and too large. They want a smaller, more "minimalistic" version with "bold stroke graphics" not the current big character style.

Current logo at `src/claude_ui/claude_render.rs:1673-1680`:
```rust
let logo = r#"        oooo
        `888
 .ooooo.   888  ooo. .oo.
d88' `88b  888  `888P"Y88b
888ooo888  888   888   888
888    .o  888   888   888
`Y8bod8P' o888o o888o o888o
"#;
```
This is already the compact version (5 lines). But the right panel may be too narrow for even this version. With the panel_constraint at `Constraint::Length(13)` for the info area, the logo may not have enough horizontal space.

## Problem

The logo can still be too wide for narrow right panels. The ASCII art needs to be even more compact — a minimalist bold-stroke version that fits in tight spaces (as little as 20-25 chars wide).

## Proposed Solution

Design a smaller, bolder logo variant. Options:

### Option A: Ultra-compact (3-4 lines, ~20 chars wide)
```
╔═╗╦  ╔╦╗╔═╗
║╣ ║  ║║║╠═╣
╚═╝╩═╝╩ ╩╩ ╩
```

### Option B: Bold minimal (similar to current but narrower)
```
▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄
█ ███ █   ███ █
█ █   █   █ █ █
█ ███ █   ███ █
█ █   █   █ █ █
█ ███ ███ █ █ █
▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
```

### Selected approach: Use Unicode box-drawing characters for a bold, compact look
```
┏━╸╻  ┏┳┓┏━┓
┣╸ ┃  ┃┃┃┣━┫
┗━╸┗━╸╹ ╹╹ ╹
```
Width: 14 chars. Fits any panel width. Bold unicode box-drawing strokes.

Tagline stays the same: "Local first terminal agent v0.1.0"

File to change:
- `src/claude_ui/claude_render.rs` — replace logo lines 1673-1680

## Acceptance Criteria

- [ ] Logo renders cleanly in panels as narrow as 25 chars wide
- [ ] Logo uses bold Unicode box-drawing characters (not ASCII art)
- [ ] Logo is 3-4 lines tall (not 5-7)
- [ ] Logo maintains the Elma branding identity
- [ ] Tagline still fits beneath the logo

## Verification Plan

- Visual inspection: build and run, verify logo is not cropped
- Measure: verify logo width ≤ 25 chars
- Unit test: verify logo fits in minimum panel width

## Dependencies

None.

## Notes

The compact ASCII logo from Task 6 changes (5 lines) may still be too wide. The box-drawing approach guarantees it fits. Keep the tagline color magenta (from Task 6).
