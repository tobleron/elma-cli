# Fix Budget Approaching Iteration Counter Bug

## Problem
In session `s_1777807006_86051000`, the terminal transcript showed the budget warning message incorrectly:
```
◦ NOTICE (Budget): Approaching iteration limit (1/3)
...
◦ NOTICE (Budget): Approaching iteration limit (1/3)
...
◦ NOTICE (Budget): Approaching iteration limit (1/3)
```

The loop iterations tracked in the backend `trace_debug.log` incremented correctly (`1/3`, `2/3`, `3/3`), but the frontend UI message was stuck emitting `1/3` for every warning.

## Required Actions
1. **Fix UI State Sync:** Locate the event emitter for the budget notice in the tool orchestration loop. Ensure it passes the actual current iteration counter to the `TerminalUI` rather than hardcoding `1` or reading from a stale initial state.
2. **Review TUI Notice Logic:** Check the rendering code for the `NOTICE (Budget)` block to confirm it dynamically interpolates the iteration value from the incoming event payload.
