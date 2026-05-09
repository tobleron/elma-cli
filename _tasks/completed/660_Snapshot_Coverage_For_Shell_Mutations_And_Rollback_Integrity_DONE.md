# Task 660: Snapshot Coverage For Shell Mutations And Rollback Integrity

**Status:** pending
**Priority:** HIGH
**Type:** Reliability / Safety
**Scope:** `src/snapshot.rs`, `src/tool_calling.rs`, `src/patch_executor.rs`, `src/persistent_shell.rs`
**Source:** postponed task 094, user priority on best error handling

## Summary

Make every mutating path snapshot-aware and verify rollback integrity for shell, write/edit/patch, and code interpreter mutations.

## Evidence And Gap

- `tool_calling.rs` has best-effort snapshot helpers, but some failures only trace snapshot errors.
- Shell commands can mutate files through arbitrary commands after permission approval.
- Snapshot coverage needs tests that prove recovery works, not only that snapshots are attempted.

## Implementation Plan

1. Define mutating operation classes and their snapshot requirements.
2. Require successful pre-mutation snapshot or explicit user-approved no-snapshot execution for high-risk operations.
3. Add rollback verification tests for write/edit/patch/shell mutation.
4. Persist snapshot ids in tool events and transcript rows.

## Acceptance Criteria

- [ ] Mutating tools record snapshot id or explicit no-snapshot reason.
- [ ] Rollback restores file contents and existence state.
- [ ] Snapshot failures do not silently proceed on high-risk mutations.
- [ ] Session artifacts link tool calls to snapshots.

## Verification Plan

Run mutation fixtures that edit, delete, fail midway, rollback, and compare file hashes.

