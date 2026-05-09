# Task 766: Strengthen Finalization Against Stale Artifacts And Objective Drift

## Type

Finalization / Truthfulness / Semantic Continuity

## Severity

Critical

## Scope

Finalization verifier, continuity tracker, artifact verifier, objective state

## Problem

The latest session ended the completed-task verification prompt with:

`Completed the requested artifact work. Created or updated: _testing_prompts/01_prompt.txt, _testing_prompts/06_prompt.txt`

This answer was unrelated to the user's current request. The continuity tracker still logged `continuity_score=1.00`.

## Root Cause

Finalization checks are too shallow. They look at non-empty output, some evidence presence, length ratios, and artifact existence, but they do not compare the final answer against the current objective contract, required scope, stop outcome, and current-turn deliverables.

## Proposed Solution

- Make finalization verification consume `CurrentTurnContext`, `ObjectiveState`, `DeliverableContract`, and `ScopeCoverageLedger`.
- Reject or relabel answers that mention artifacts not requested in the current turn.
- Reject completion language when objective requirements remain unresolved.
- Replace heuristic answer-length checks with objective contract checks.
- If finalization fails, continue with a changed approach or produce a precise partial-progress report.

## Acceptance Criteria

- [ ] Stale deliverable names from prior turns fail finalization.
- [ ] `continuity_score=1.00` cannot occur when final answer solves a different request.
- [ ] Iteration-limit answers are labeled partial unless objective state proves completion.
- [ ] Tests cover stale artifact finalization, wrong objective answer, clean complete answer, and partial blocked answer.

## Verification Plan

- Replay latest session and assert the final turn cannot return `_testing_prompts` artifact completion.
- Unit test finalization verifier with mismatched objective and answer.
- Inspect transcript rows for rejected finalization reason.

## Dependencies

Depends on Tasks 761, 762, 763, and 764.

## Notes

Do not add more prompt-only finalization instructions. This must be a runtime verifier with typed inputs.

