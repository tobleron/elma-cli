# Task 497: Reset Stagnation Signals Per User Turn

**Status:** pending
**Priority:** high
**Primary surfaces:** `src/stop_policy.rs`, `src/tool_loop.rs`
**Related issues:** 4B model reads 1-2 docs then gets force-finalized before reaching source code comparison

## Objective

Allow the model to make multiple unique tool calls across a turn without premature force-finalization. Currently, stagnant signals from previous turns poison the `seen_signals` set, causing legitimate `read` / `shell` calls in a new turn to be classified as stagnation.

## Problem

`stop_policy.rs:381-383` tracks all tool signals in a `HashSet<String>`:
```rust
pub(crate) fn register_signal(&mut self, signal: String) -> bool {
    self.seen_signals.insert(signal)
}
```

This set is **never cleared across user turns**. When turn 0 ("hi") produces `respond` tool calls (which generate signals like `"respond:Hello! I'm Elma..."`), those signals remain in the set. When turn 1 ("read all docs") begins and the model eventually calls `respond` again, if the response text happens to start similarly, the signal is already "seen" → `register_signal` returns `false` → no new signal → stagnation counter increments.

Additionally, a `respond` call in a legitimate evidence-collection turn should not be counted as stagnation at all — the model might `respond` to surface intermediate findings before continuing.

Trace evidence from both crash sessions:
```
stagnation run 1 (tool: unknown) (no new tool signal)
stagnation run 2 (tool: unknown) (no new tool signal)
stagnation threshold reached; forcing finalization
```

## Implementation Plan

1. Add `pub(crate) fn reset_signals(&mut self)` to `StopPolicy`:
   ```rust
   pub(crate) fn reset_signals(&mut self) {
       self.seen_signals.clear();
       self.stagnation_runs = 0;
       self.consecutive_respond_calls = 0;
       self.consecutive_respond_only_turns = 0;
       self.last_failed_tool_name = None;
   }
   ```

2. Call `stop_policy.reset_signals()` at the **start of each user turn** in `tool_loop.rs`. The exact call site is where the tool loop is initialized for a new user request — locate the `ToolLoopBudget::default()` instantiation or the first `tool_loop: starting` trace and add the reset call immediately before or after.

3. Verify that `tool_signal()` for `"respond"` at `tool_loop.rs:513-521` produces distinct signals for distinct answer content (it already does: first 40 chars of answer).

## Success Criteria

- The "read all docs and compare with source code" prompt allows the model to make **6+ unique read calls** without hitting stagnation
- Stagnation only fires on actual repeated/double-read calls (e.g., re-reading the same file path)
- `continuity_score` improves above 0.80 (was consistently 0.78)
