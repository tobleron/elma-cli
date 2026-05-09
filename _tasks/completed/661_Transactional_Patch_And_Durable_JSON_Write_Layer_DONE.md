# Task 661: Transactional Patch And Durable JSON Write Layer

**Status:** pending
**Priority:** CRITICAL
**Type:** Reliability / Data Integrity
**Scope:** `src/atomic_write.rs`, `src/patch_executor.rs`, `src/session_write.rs`, `src/session_flush.rs`, `src/task_persistence.rs`
**Source:** agent `_knowledge_base` audit; Roo safeWriteJson tests; old SQLite/session tasks

## Summary

Centralize durable writes and transactional patch behavior so session/task/config files and multi-file patches cannot be corrupted by partial writes or concurrent mutations.

## Evidence And Gap

- `patch_executor.rs` advertises rollback but needs induced-failure coverage.
- `session_write.rs` writes temp files but lacks a reusable durable write API with lock, unique temp, backup, parent fsync, and retry/rollback semantics.
- Safe JSON writing is critical because Elma persists sessions, tasks, traces, and config.

## Implementation Plan

1. Create a durable write API with unique temp path, atomic rename, optional backup, parent directory sync where supported, and structured errors.
2. Route session, task, config, evidence, and index writes through it.
3. Make patch execution all-or-nothing with rollback on any apply/verification failure.
4. Add corruption recovery for partially written JSON artifacts.

## Acceptance Criteria

- [ ] Concurrent session/task writes do not corrupt JSON.
- [ ] Induced patch failure restores all touched files.
- [ ] Durable write errors include path, phase, and recovery hint.
- [ ] Tests cover backup restore, invalid JSON recovery, and rename failure where practical.

## Verification Plan

Run atomic write and patch transaction tests with temporary directories and induced failures.

