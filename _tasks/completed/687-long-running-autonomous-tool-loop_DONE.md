# Task 687: Long Running Autonomous Tool Loop

## Type

Architecture

## Severity

Critical

## Scope

Architecture-wide

## Session Evidence

Across the testing prompt sessions, Elma repeatedly stopped at the fixed tool-loop budget instead of completing the requested work:

- `sessions/s_1778084330_810579000/session.md`: prompt 01 ended with `StopReason Budget limit: iteration_limit_reached` before creating the requested `project_tmp` report.
- `sessions/s_1778084542_908955000/session.md`: prompt 02 ended with `StopReason Budget limit: iteration_limit_reached`.
- `sessions/s_1778084708_633588000/session.md`: prompt 03 ended with `StopReason Budget limit: iteration_limit_reached`.
- `sessions/s_1778084857_555628000/session.md`: prompt 04 logged `[STAGNATION] stagnation run 3 (tool: read)` and then stopped at `iteration_limit_reached`.
- `sessions/s_1778085552_840248000/session.md`: prompt 08 stopped at `iteration_limit_reached` after a partial backup.

`trace_debug.log` in these sessions shows `tool_loop: starting max_iterations=12 stagnation_threshold=8 timeout=30m`.

## Problem

Elma is intended to be a continuing autonomous agent. A fixed 12-iteration budget is too shallow for multi-step local tasks and causes premature finalization even when the user explicitly requested concrete artifacts. The stop reason is visible, but the runtime still proceeds as if the turn can end with an answer.

## Root Cause Hypothesis

Confirmed: tool-loop control currently uses a static iteration ceiling for tasks that require discovery, analysis, file writing, and verification.

Likely: stop-policy logic treats iteration budget as a terminal condition instead of a planning signal that should trigger continuation, decomposition, or a new approach branch.

## Proposed Solution

Implement an autonomy-oriented runtime budget model:

- Inspect `src/tool_loop.rs`, `src/stop_policy.rs`, `src/budget_forecaster.rs`, `src/complexity_gate.rs`, and `src/approach_engine.rs`.
- Replace the fixed default iteration ceiling for non-direct tasks with a dynamic budget envelope derived from complexity, required deliverables, observed progress, and tool result size.
- Treat `iteration_limit_reached` as a recoverable planning event when outstanding user requirements remain.
- Add stagnation-aware branching: repeated malformed calls, repeated duplicate calls, or no new evidence should trigger a new strategy rather than finalization.
- Surface continuation decisions as transcript rows.

## Acceptance Criteria

- [ ] File-output and audit prompts do not stop only because 12 iterations elapsed.
- [ ] Stagnation causes strategy repair or explicit incomplete-state finalization with evidence, not silent budget exhaustion.
- [ ] Stop reasons distinguish completed, incomplete-but-finalized, model/transport failure, and true hang/stagnation.

## Verification Plan

Replay prompts 01, 04, and 08 from `_testing_prompts/`. Each session must either complete the requested artifact and verification or continue/replan until a concrete unrecoverable blocker is logged.

## Dependencies

None.

