# Task 390: Approach Branch Retry And Prune Engine

**Status:** Pending
**Priority:** HIGH
**Estimated effort:** 3-4 days
**Dependencies:** Task 379, Task 389
**References:** user objective for failed approaches and retry until request is achieved

## Objective

Add approach-aware retry behavior: when an approach fails, that branch stops progressing down the work graph, and Elma creates a new approach for the same original objective.

## Problem

Current retry behavior can change strategy, but it does not model failed approaches as first-class branches. Without explicit branch state, Elma may keep trying the same failing path or lose semantic continuity.

## Implementation Plan

1. Add `ApproachAttempt` state:
   - `approach_id`
   - `objective_id`
   - `strategy`
   - `status`
   - `failure_class`
   - `evidence`
2. Define branch statuses: `active`, `succeeded`, `failed`, `pruned`, `superseded`.
3. Wire failure classes from Task 379 into branch pruning.
4. Add a new-approach generator that chooses a different strategy without keyword triggers.
5. Prevent failed branches from spawning lower-level goals, plans, or instructions.
6. Emit transcript rows for approach start, failure, prune, retry, and success.

## Verification

```bash
cargo test approach
cargo test retry
cargo test decomposition
cargo test orchestration
cargo build
```

Manual probe:

- Force a tool failure and verify Elma stops that branch, creates a new approach, and keeps solving the original user request.

## Done Criteria

- Repeated failures do not continue down the same approach chain.
- New approaches carry the same original objective.
- Branch state is persisted or reconstructable from session events.
- Retry behavior remains bounded and transcript-visible.

