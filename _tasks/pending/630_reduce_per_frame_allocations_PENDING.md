# 630 Eliminate Per-Frame Allocations

## Summary
Multiple unnecessary allocations happen every frame: thinking_entries double iteration + collect, ELMA logo string slicing, line_mapping clone, filtered_slash_commands rebuild.

## Affected Files
- `src/claude_ui/claude_render.rs:1444` — `thinking_entries` iterated twice per frame (collapse + retain)
- `src/claude_ui/claude_render.rs:1459` — `all_thinking` Vec<&ThinkingEntry> collect per frame
- `src/claude_ui/claude_render.rs:1862` — ELMA logo re-sliced into char groups + String joins per frame
- `src/claude_ui/claude_render.rs:1205` — `last_line_mapping.clone()` of entire Vec<usize> per frame
- `src/claude_ui/claude_render.rs:1053` — `filtered_slash_commands()` rebuilds Vecs per frame while picker active

## Current Behavior
- Two passes over thinking_entries (mutate, then retain)
- New Vec allocated for all_thinking references every frame
- Logo: each row → Vec<char> → 3-char slice → String join → Span::styled → per frame
- line_mapping cloned in full every frame (500+ usize entries for large transcript)
- Slash picker: exact + prefix match Vecs rebuilt each frame

## Proposed Fix
- Fold collapse + retain into single `retain_mut` pass
- Pass `&[ThinkingEntry]` instead of `Vec<&ThinkingEntry>` to render function
- Pre-compute logo styled lines as `[Line<'static>; 4]` at construction, swap highlight style per frame
- Store only visible range of line_mapping, or use Arc<[usize]>
- Cache filtered slash commands, recompute only when query changes

## Estimated CPU Savings
~10% of per-frame render CPU (small individually, adds up together)

## Status
PENDING
