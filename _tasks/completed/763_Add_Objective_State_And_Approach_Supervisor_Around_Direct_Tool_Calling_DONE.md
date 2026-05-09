# Task 763: Add Objective State And Approach Supervisor Around Direct Tool Calling

## Type

Architecture / Autonomy / Work Graph

## Severity

Critical

## Scope

Direct tool-calling pipeline, stop policy, approach engine, work graph bridge

## Problem

The live path now sends most work through direct model tool calling and returns a single `Respond` step. This bypasses much of Elma's intended architecture: objective state, approach branching, work graph status, and completion contracts. In the latest session, Elma kept pursuing `tasks/completed`, did not branch to `_tasks/completed`, then finalized stale artifact text after iteration limit.

## Root Cause

Direct tool calling has no supervisor that can compare the current objective to progress, detect failed strategies, and fork a sibling approach. The model is left to self-correct inside the same branch until stop policy fires.

## Proposed Solution

- Add an `ObjectiveState` for each turn with:
  - raw objective,
  - required outcomes,
  - active approach id,
  - completed evidence,
  - blockers,
  - unresolved requirements.
- Add an approach supervisor around direct tool calling.
- When repeated failures happen, the supervisor should create a sibling approach with a changed strategy rather than continuing the same failed branch.
- Stop policy should consult objective state before finalization.
- Persist objective/approach state in session artifacts.

## Acceptance Criteria

- [ ] Repeated failed path access forks to a path-resolution approach instead of repeating the same path.
- [ ] Iteration limit cannot finalize if objective state says required outcomes are unresolved.
- [ ] Session trace shows active approach, failed approach, and sibling approach creation.
- [ ] Direct tool-calling results still stream to the UI normally.

## Verification Plan

- Fixture: request `tasks/completed` when only `_tasks/completed` exists.
- Fixture: repeated invalid tool call should create a new approach with a different strategy.
- Replay latest session and confirm the completed-task verification turn does not end in stale artifact completion.

## Dependencies

Depends on Task 761. Coordinate with Tasks 764 and 765.

## Notes

This task should not revive the old DSL flow. Keep strict JSON/tool calling, but wrap it with objective-aware runtime state.

