# Task 760: Enforce Task Governance Guidelines In Task Creation Workflows

## Revision Note — 2026-05-07

The runtime/source-code interpretation of this task was reverted after review. Do not treat this task as approval to add task-governance analyzers, checklist enforcement, or task-quality runtime code to `src/task_steward.rs` or related Elma runtime modules.

The intended governance boundary is agent/developer workflow guidance: coder agents working in this repository must read and follow `AGENTS.md`, `_tasks/_tasks.md`, `_tasks/_guidelines.md`, and `_tasks/_masterplan.md` before creating or modifying task files. Runtime Elma task-steward behavior should only be changed by a new, explicit task with separate approval and verification.

## Type

Governance / Reliability / Task Quality

## Severity

High

## Scope

Task steward, task creation helpers, project guidance, analyzer checks

## Problem

Elma has strong objectives in `AGENTS.md` and `_tasks/_guidelines.md`, but task creation can still produce work items that repeat failed repair patterns, introduce deterministic request keyword checks, or do not explicitly protect semantic continuity. This risks another loop of completed tasks that compile but do not improve real behavior.

## Root Cause

Task governance has been mostly documentation-driven. The runtime and task-steward guidance do not yet require every new pending task to prove objective alignment, non-keyword behavior selection, non-circularity, and verification coverage before the task is accepted.

## Proposed Solution

- Update task-steward logic so task creation reads `AGENTS.md`, `_tasks/_tasks.md`, and `_tasks/_guidelines.md`.
- Add a task-quality checklist section to new pending tasks created by Elma.
- Add a local analyzer or test helper that flags pending tasks missing guideline alignment, verification plan, anti-keyword policy, or anti-circularity notes.
- Do not reject tasks by brittle title words; validate the task structure and explicit commitments.

## Acceptance Criteria

- [ ] New Elma-created pending tasks include objective alignment, anti-keyword, anti-circularity, and verification commitments.
- [ ] Task steward guidance uses `_tasks/_tasks.md` and `_tasks/_guidelines.md`.
- [ ] A local check can report pending task files that do not satisfy the governance checklist.
- [ ] Existing completed tasks are not rewritten unless a new regression is confirmed.

## Verification Plan

- Unit test the task-quality checker with valid and invalid fixture tasks.
- Run the checker against `_tasks/pending/`.
- Start a task-steward prompt and confirm guidance includes `_tasks/_guidelines.md`.

## Dependencies

None.

## Notes

This task should not add more bureaucracy to implementation tasks. It should prevent low-quality task creation that sends Elma into repeated repair cycles.
