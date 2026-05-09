# Task 769: Add Last Session Replay Regression Harness

## Type

Testing / Session Forensics / Regression Gate

## Severity

High

## Scope

Scenario tests, session replay, trace assertions, prompt fixtures

## Problem

The latest failure classes were only discovered by manually inspecting session artifacts. There is no replay harness that turns bad sessions into durable regression tests, so completed tasks can appear successful while the same behavioral failures recur.

## Root Cause

Current tests cover many units, but not end-to-end session narratives with current-turn context, stale evidence, artifact tracking, path recovery, and finalization all interacting.

## Proposed Solution

- Create a replay fixture from the latest session prompts:
  1. `hi`
  2. `find AGENTS.md and tell me where it is located.`
  3. `root path has no AGENTS.md ?`
  4. `Read all testing prompts and tell me if they are enough to test elma_cli`
  5. `Verify how many of the completed tasks under tasks/completed are actually completed.`
- Add assertions over final answers and trace/session artifacts.
- Support deterministic stub model/tool sequences where needed so CI does not depend on a live endpoint.
- Add a session-forensics check that can fail on stale artifact finalization, missing tool summaries, incomplete coverage, or unresolved objective completion.

## Acceptance Criteria

- [ ] The latest failure sequence is represented as a regression fixture.
- [ ] Tests fail if AGENTS root location is omitted.
- [ ] Tests fail if an existence question becomes artifact completion.
- [ ] Tests fail if completed-task verification resolves to stale `_testing_prompts` output.
- [ ] Tests fail if tool summaries claim no tools ran when tool artifacts exist.

## Verification Plan

- Run the new replay test locally.
- Confirm the current failing behavior is caught before fixes.
- Confirm Tasks 761-768 make the replay pass.

## Dependencies

Should be implemented after Tasks 761-768 define the new contracts, but initial failing fixtures can be added earlier.

## Notes

This is the guard against infinite improvement loops. Every serious bad session should become a replayable test.

