# Task 618: Budget iterations too restrictive for INVESTIGATE tasks with model recovery

## Type

Bug

## Severity

Medium

## Scope

System-wide (budget/complexity)

## Session Evidence

**Session:** `s_1777843822_776972000`, multiple turns
**Model:** Huihui-Qwen3.5-4B

Multiple turns hit `iteration_limit_reached` before completing the task:

1. Turn "show me the first 5 lines of GEMINI.md": 7 iterations (6 budget), stopped by limit — the task COMPLETED (shell `head` found the file) but the final answer wasn't properly finalized because the budget was exhausted mid-recovery

2. Turn "did you do it?": 6 iterations, ALL wasted on read failures — the model never even tried shell because it ran out of budget before recovering

3. Turn "How about Minimalistic without TUI": 7 iterations, stopped by limit — the model was actively searching but ran out of budget

The current INVESTIGATE budget (6 iterations) assumes the model will use tools efficiently. But the 4B model:
- Wastes 1-2 iterations on invalid tool calls
- Needs 1-2 iterations to self-correct after failures
- Needs 1-2 iterations to gather evidence
- Needs 1-2 iterations for finalization

6 iterations is barely enough for an IDEAL case, and nowhere near enough when the model needs recovery cycles.

Looking at the budget at `src/stop_policy.rs:87-93`:
```rust
"INVESTIGATE" => 6,
"MULTISTEP" => 12,
"OPEN_ENDED" => 20,
```

## Problem

The INVESTIGATE tier budget (6 iterations) is too low for small models that need recovery cycles. The model frequently exhausts the budget before completing the task, even when it WAS making progress (e.g., found the file with glob, just needed one more iteration to read it).

The budget exhaustion creates a false failure — the model COULD have completed the task given 2-3 more iterations.

## Root Cause Hypothesis

**Confirmed:** Static 6-iteration budget doesn't account for model self-correction overhead. Small models need more iterations to recover from validation errors, retry with corrected arguments, and finalize with evidence.

## Proposed Solution

Increase INVESTIGATE budget from 6 to 9 iterations. This gives the model ~3 additional recovery cycles for self-correction.

Alternative (more sophisticated): dynamically extend the budget when the model shows evidence of making progress (e.g., last tool call succeeded, evidence was gathered). If the model is failing repeatedly, don't extend — force finalization. If making progress, allow 2-3 extra iterations.

Minimal change: `src/stop_policy.rs:89` — change `6` to `9`.

Files to change:
- `src/stop_policy.rs` — INVESTIGATE budget increase

## Acceptance Criteria

- [ ] INVESTIGATE tasks have 9 max_iterations (not 6)
- [ ] Tasks that currently exhaust at 6 iterations can complete in 7-9
- [ ] DIRECT tasks remain at 3 (no change for simple tasks)
- [ ] MULTISTEP and OPEN_ENDED budgets unchanged

## Verification Plan

- Unit test: verify `StageBudget::from_complexity("INVESTIGATE")` returns max_iterations=9
- Replay session: verify "show me the first 5 lines of GEMINI.md" turn completes without budget exhaustion
- Check that stagnating tasks still finalize (stagnation threshold catches non-progress)

## Dependencies

- Task 616 (read→shell fallback) — reduces the need for this change somewhat, but doesn't eliminate it
- Task 607 (duplicate suppression) — also reduces wasted iterations

## Notes

If the model reaches iteration 9 without completing, it's truly stuck and SHOULD finalize. The 3 extra iterations are for recovery, not for meandering. Combined with Task 616 (auto-read-fallback), most tasks should complete within the extended budget.
