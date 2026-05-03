# Task 460: Background Job Tool And Notify-On-Complete

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Dependencies:** Task 387, Task 459
**References:** source-agent parity: Crush job tools, Hermes background process notifications

## Objective

Add a structured background job tool so Elma can start, inspect, stream, and stop long-running local processes without blocking the agent loop.

## Implementation Plan

1. Reuse or extend `src/background_task.rs` and `src/persistent_shell.rs`.
2. Add tool declarations for:
   - `job_start`
   - `job_status`
   - `job_output`
   - `job_stop`
3. Route execution through sandbox/profile policy from Task 459.
4. Emit transcript notifications when jobs complete, fail, or exceed output budgets.
5. Store job output as session artifacts with concise summaries in context.

## Verification

```bash
cargo test background
cargo test tool_calling
cargo test transcript
cargo build
```

## Done Criteria

- Long-running commands do not block the session.
- Completion is visible without polling by the model.
- Output is bounded, persisted, and summarized.
- Job tools obey permissions and execution profiles.

