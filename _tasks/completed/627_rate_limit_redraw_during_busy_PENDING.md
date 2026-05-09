# 627 Rate-Limit Redraws During Busy/Idle Phases

## Summary
`await_with_busy_queue` polls at 40ms with full UI redraw, and `pump_ui` forces draws on every call when status thread is active. During a 30s model call that's ~750 full-screen redraws even when only a spinner changes.

## Affected Files
- `src/app_chat_loop.rs:23` — `await_with_busy_queue` polls at 40ms with full draw
- `src/ui/ui_terminal.rs:841` — `pump_ui` forces `pending_draw = true` when status thread active
- `src/ui/ui_terminal.rs:1007` — `run_input_loop` calls `draw()` before event check
- `src/tool_loop.rs` — `pump_ui` called at 19+ locations within tool loop

## Current Behavior
- Status thread active → `pump_ui` → `pending_draw = true` → draw on next 40ms tick
- Effectively 25fps during model calls, even when only a spinner character changes
- Full `render_ratatui` pipeline executes per draw (markdown parse, line wrap, logo animation)

## Proposed Fix
- Add `last_draw: Instant` to `TerminalUI`
- In `pump_ui`: only set `pending_draw = true` if time since last draw > min_frame_interval
- Use 100ms (10fps) during non-streaming phases, 40ms (25fps) during streaming
- In `run_input_loop`: move `draw()` after event processing, only redraw if pending OR time elapsed
- In `await_with_busy_queue`: use 100ms polling when not streaming

## Estimated CPU Savings
~30% of render CPU during model calls

## Status
PENDING
