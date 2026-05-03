# Task 591: Reset Identical-Error Tracker After Strategy Change

## Session Evidence
Session `s_1777822834_658323000`: After 3 `read` failures, the `is_identical_error_loop()` check fired and injected the shell fallback strategy shift. The model successfully switched to `shell cat` commands. But the trace shows that `identical-error loop detected for read` continued to fire on EVERY subsequent iteration (lines 62, 72, 84, 94, 106, 117, 127, 138, 150, 161...), even after the model had switched strategies and was executing successful `cat` commands.

The `is_identical_error_loop()` method in `stop_policy.rs` checks `consecutive_identical_errors >= 3`, but this counter never resets. Once the model switches strategies and starts succeeding, the counter should reset for that tool.

## Problem
The `consecutive_identical_errors` counter in `StopPolicy` is monotonic — it increments on identical failures but never resets. Once triggered, the strategy-shift message fires on every single iteration for the rest of the tool loop, polluting the model's context and the user's terminal with redundant warnings.

## Solution
Add a reset mechanism for `consecutive_identical_errors`:

1. When a tool CALL succeeds (not just any tool, but a different tool from the one that was failing), reset `consecutive_identical_errors` and clear `last_tool_error_text`
2. In `record_tool_result()`, after `tool_failures.push(...)`, check if the tool that just failed is different from `last_failed_tool_name` — if it changed, the model HAS already shifted strategy
3. Specific reset trigger: If the last successful tool call was NOT `read` (after a sequence of read failures), consider the identical-error loop resolved

Implementation: In `tool_loop.rs`, after a tool succeeds, call a method on `StopPolicy` to check if the strategy changed from the failing tool and reset accordingly.
