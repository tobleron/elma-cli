# 630 Reduce Per-Frame Allocations

## Status
**PARTIALLY COMPLETE** (4 of 5 sub-tasks done)

## Completed
- **Thinking entries single pass**: `retain_mut` combines collapse + retain + reveal in one pass (claude_render.rs:1491)
- **Logo pre-computed**: `logo_groups: Vec<[&'static str; 4]>` stored as static data, constructed once (claude_render.rs:171-175)
- **Line mapping visible-only**: `last_line_mapping` stores only visible portion via `.skip(start_line).take(height)` (claude_render.rs:1252)
- **Input wrapping cache fields exist**: `cached_input_key` and `cached_wrapped_input` allocated and used for height calculation (claude_render.rs:105-106, 1119-1124)

## Remaining
1. **Input wrapping cache not used for display**: Line 1310 still calls `wrap_input_lines(&self.input_lines, text_width.max(10))` fresh every frame instead of using `self.cached_wrapped_input`. The cache is computed for height but the display path bypasses it.

2. **Slash commands still rebuilt per frame**: `filtered_slash_commands()` (line 597) creates new `Vec`s on every call. Called from `picker_select_down`, `picker_select_up`, `selected_slash_command`, and render path (lines 547, 559, 574, 1097, 1345).

## Files
- `src/claude_ui/claude_render.rs:1310` — fresh wrap_input_lines call (should use cache)
- `src/claude_ui/claude_render.rs:597` — filtered_slash_commands rebuilds Vecs
- `src/claude_ui/claude_input.rs:166` — PickerState::filter_commands rebuilds Vecs
