# Task 698: Long-Running Autonomy And Stop-Policy Recovery

## Type

Autonomy / Runtime Stability

## Severity

Critical

## Scope

Stop policy, continuation, stagnation recovery, approach retry

## Session Evidence

Round 2 showed frequent premature or weak stopping:

- Prompt 02: `iteration_limit_reached`, continuation, then `respond_abuse`.
- Prompt 03: repeated read stagnation, then `respond_abuse`.
- Prompt 04: `iteration_limit_reached`, continuation, repeated stagnation, then `respond_abuse`.
- Prompt 05: repeated read stagnation, then `respond_abuse`.
- Prompt 06: `iteration_limit_reached` despite meaningful progress before producing the report.

## Problem

Elma is intended to be a long-running autonomous agent by default. Current stop behavior still finalizes after shallow loops, repeated malformed calls, or response abuse instead of forking a new approach, narrowing scope, or switching tool strategy.

## Proposed Solution

Rework stop recovery around autonomy:

- Treat `iteration_limit_reached`, `respond_abuse`, and repeated same malformed tool calls as recoverable when the user requested a concrete deliverable.
- Fork a sibling approach after stagnation instead of continuing down the same failing branch.
- Increase smart continuation budgets for `MULTISTEP` and `OPEN_ENDED` tasks that still have missing deliverables.
- Require a deliverable checklist before finalization.
- Log why the run is continuing or stopping as transcript-native collapsible rows.

## Acceptance Criteria

- [ ] Deliverable tasks do not finalize while required artifacts are missing if recovery options remain.
- [ ] Repeated malformed tool calls trigger strategy change, not repeated stagnation.
- [ ] Prompt tests 02-06 reach either a created artifact or a clear incomplete status with no false completion.
- [ ] Trace clearly shows approach fork/recovery decisions.

## Verification Plan

Replay prompts 02-06 with debug traces and inspect stop reasons. Confirm no prompt ends solely due to `respond_abuse` without a recovery attempt.

