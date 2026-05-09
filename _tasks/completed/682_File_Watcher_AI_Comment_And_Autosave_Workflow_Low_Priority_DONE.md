# Task 682: File Watcher AI Comment And Autosave Workflow Low Priority

**Status:** pending
**Priority:** LOW
**Type:** Optional Offline Feature
**Scope:** `src/snapshot.rs`, `src/session_store.rs`, future watcher modules
**Source:** deferred task 493, postponed autosave/file tracker tasks 006/022

## Summary

Add optional file watching, autosave checkpoints, and AI-comment workflows only after core persistence, snapshot, and UI event architecture are stable.

## Evidence And Gap

- File watching and autosave are useful, but they can create hidden background activity if not transcript-native.
- Core snapshot and session tasks must land first.

## Implementation Plan

1. Build on Task 660 and Task 664 snapshot/checkpoint semantics.
2. Watch only workspace paths allowed by policy.
3. Surface watcher events as transcript/session events.
4. Keep AI-generated comments opt-in and reversible.

## Acceptance Criteria

- [ ] Watcher never scans ignored/protected paths.
- [ ] Autosave checkpoints are bounded and garbage collected.
- [ ] User can inspect and disable watcher actions.
- [ ] No hidden background mutation occurs.

## Verification Plan

Run local watcher fixtures with edits, ignored files, protected files, and rollback.

