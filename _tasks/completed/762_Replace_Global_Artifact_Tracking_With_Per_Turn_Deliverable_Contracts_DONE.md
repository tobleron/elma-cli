# Task 762: Replace Global Artifact Tracking With Per-Turn Deliverable Contracts

## Type

Architecture / Artifact Verification / Truthfulness

## Severity

Critical

## Scope

Artifact verifier, tool loop, finalization, session state

## Problem

Artifact tracking currently uses global state in `src/artifact_verifier.rs` through `OnceLock<RwLock<HashSet<String>>>`. In the latest session, a pre-existing `AGENTS.md` was reported as "Created or updated" even though the user asked whether it existed. A later unrelated turn also finalized as if `_testing_prompts/01_prompt.txt` and `_testing_prompts/06_prompt.txt` were requested deliverables.

## Root Cause

The artifact verifier tracks only path existence and a global required-artifact set. It does not distinguish:

- requested this turn vs stale from another turn,
- existed before turn vs written during turn,
- user-requested artifact vs artifact-like text in context,
- verified substantive deliverable vs incidental file.

## Proposed Solution

- Replace global required artifacts with a per-turn `DeliverableContract`.
- Record for each deliverable:
  - requested path,
  - source of request,
  - whether it existed before the turn,
  - whether a write/edit/backup tool touched it during the turn,
  - verification status,
  - whether finalization may claim creation/update.
- Persist the contract in the session.
- Deterministic artifact-completion finalization must only trigger when current-turn deliverables exist and were actually completed this turn.
- Keep compatibility wrappers only if needed, but remove load-bearing global state from runtime behavior.

## Acceptance Criteria

- [ ] Pre-existing files are not reported as created or updated without current-turn mutation evidence.
- [ ] Artifact requirements cannot leak across user turns.
- [ ] Finalization cannot use stale deliverables to answer an unrelated request.
- [ ] Deliverable state is persisted and inspectable in the session folder.
- [ ] Tests cover pre-existing file, stale prior artifact, missing artifact, and successful current-turn write.

## Verification Plan

- Replay the latest session sequence.
- Assert the AGENTS existence question does not create a deliverable contract.
- Assert the completed-task verification turn does not mention `_testing_prompts` artifacts.

## Dependencies

Depends on Task 761.

## Notes

Do not use file extension or artifact-like words as a sufficient signal. The contract must be derived from the current user objective and tool evidence.

