# Task 589: Stagnation Threshold Reduction for Repeated Same-Error Failures

## Session Evidence
Session `s_1777822834_658323000`: The stagnation threshold is set to `stagnation_threshold=8` in the tool loop trace. The model called `read` 7 times before stagnation was detected, then got 1 more attempt that also failed (8 total). Each cycle burned 8 iterations on identical broken calls. Across both cycles, 16 iterations were wasted.

The stagnation detection works as:
- `stop_policy.register_signal(sig)` returns `false` for duplicate signals → stagnation counter increases
- At `stagnation_run >= max_stagnation_cycles` (default 8), stagnation triggers finalization

## Problem
The stagnation threshold (8) is based on the DEFAULT `StageBudget` value. But when repeated failures produce the EXACT SAME error (same tool, same missing parameter), the system should escalate faster. Wasting 8+ iterations on identical broken calls is wasteful even with OPEN_ENDED budget.

## Solution
Add an acceleration heuristic: if 3+ consecutive tool failures produce the IDENTICAL error message (not just same tool — same error text), reduce the stagnation threshold to 3 for that tool.

Implementation:
1. In `StopPolicy`, track consecutive errors with same tool + same error text
2. When `consecutive_identical_errors >= 3`, inject a forceful strategy-shift message AND reduce stagnation threshold from 8 to 3
3. The strategy-shift message should say: "This tool has failed 3+ times with the same error. Stop using it and try a completely different approach."

Implementation location: `src/stop_policy.rs`, add `consecutive_identical_error_count` tracking and modify stagnation detection.
