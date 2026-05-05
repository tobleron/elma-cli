# 634 Reduce Permission Prompt Polling

## Summary
`request_permission` polls at 50ms with unconditional `draw()` call while waiting for user y/n input. The prompt text is static, so 20fps redraw is excessive.

## Affected Files
- `src/ui/ui_terminal.rs:311` — `request_permission` loops with `sleep(50ms)` + `draw()`

## Current Behavior
- While waiting for permission (y/n), draws at 20fps unconditionally
- The permission prompt is static text — redraws are wasted
- `pending_draw` guard mitigates somewhat but function structure creates churn

## Proposed Fix
- Only draw when events arrive (event-driven), not on every sleep tick
- Or use a longer sleep (200ms) since the prompt is static
- Use `recv()` with timeout on the event channel instead of sleep + try_recv

## Estimated CPU Savings
Negligible (permission prompts are rare and brief)

## Status
PENDING
