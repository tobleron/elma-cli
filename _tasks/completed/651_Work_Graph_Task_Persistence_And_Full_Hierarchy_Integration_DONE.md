# Task 651: Work Graph Task Persistence And Full Hierarchy Integration

**Status:** pending
**Priority:** CRITICAL
**Type:** Architecture / Task Persistence
**Scope:** `src/work_graph.rs`, `src/work_graph_bridge.rs`, `src/task_persistence.rs`, `src/runtime_task.rs`, `_elma-tasks/`
**Source:** deferred task 494, AGENTS.md task persistence rule

## Summary

Ensure every work graph instruction that resolves to a step becomes a persisted task in both session state and `_elma-tasks/` when policy requires it.

## Evidence And Gap

- AGENTS.md requires per-session `sessions/<id>/runtime_tasks/tasks.json` and per-workspace `_elma-tasks/NNN_{auto|user}_...md`.
- `src/task_persistence.rs` exists, but integration with live work graph and approach execution needs verification.
- Deferred Task 494 had the right direction but included outdated or overly broad wording.

## Implementation Plan

1. Audit live call sites that build `WorkGraph`, `ApproachEngine`, and runtime tasks.
2. Persist task creation, status transitions, approach id, graph node id, stop reason, and evidence references.
3. Add status flow enforcement: pending -> in_progress -> completed|failed.
4. Add resume tests that rebuild active task state after restart.
5. Surface task status changes as transcript rows when relevant.

## Acceptance Criteria

- [ ] Auto and user tasks are not memory-only.
- [ ] Task ledgers survive session resume.
- [ ] Approach/task/evidence links are present and traceable.
- [ ] Duplicate task creation is prevented on retries/resume.

## Verification Plan

Run task persistence tests plus a synthetic multi-step session that creates, updates, completes, fails, and resumes tasks.

