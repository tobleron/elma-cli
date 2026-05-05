# 631 Cache Input Wrapping

## Summary
`wrap_input_lines` re-wraps the entire user input buffer on every frame, walking every character via `str_display_width` and `char_indices`.

## Affected Files
- `src/claude_ui/claude_render.rs:1707` — `wrap_input_lines` iterates chars per frame
- `src/claude_ui/claude_render.rs:1077` — called every frame from `render_ratatui`

## Current Behavior
- On every frame: wraps entire `input_lines` Vec<String> to display width
- For multi-line input: walks char_indices per line, computes display width per character
- Input rarely changes between frames, yet recomputed every draw

## Proposed Fix
- Cache wrapped input as `Vec<String>` in render state
- Invalidate only when `input_lines` content or `terminal_width` changes
- Use a content hash or dirty flag to detect change

## Estimated CPU Savings
~2% of per-frame render CPU

## Status
PENDING
