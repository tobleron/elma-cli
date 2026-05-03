# Task 472: Session Rewind And Checkpoint Restore UX

**Status:** pending
**Source patterns:** Roo-Code rewind/checkpoint restore, OpenHands state recovery, Hermes checkpoint manager
**Depends on:** completed Task 338 (event log), Task 456 (file context tracker)

## Summary

Add user-facing session rewind and checkpoint restore capabilities for transcript state, tool state, and file edits where snapshots are available.

## Why

Elma has session storage and snapshot-related code, but reference agents expose clearer recovery workflows. A robust rewind path helps users recover from bad edits, failed tool loops, and wrong turns without manually reconstructing state.

## Implementation Plan

1. Define checkpoint boundaries using the action-observation event log.
2. Link file snapshots to edit/write/patch events.
3. Add a UI or command path to inspect checkpoints and rewind to one.
4. Make restore behavior explicit about files, transcript, and session metadata affected.
5. Add tests for rewind after edit, after failed tool, and after compaction.

## Success Criteria

- [ ] User can list checkpoints for a session.
- [ ] Rewind clearly states what will be reverted.
- [ ] File restore uses snapshots and refuses unsafe missing snapshots.
- [ ] Transcript and event log remain coherent after rewind.
- [ ] Tests cover partial restore failures.

## Anti-Patterns To Avoid

- Do not use destructive git commands for internal rewind.
- Do not silently discard user edits outside Elma snapshots.
- Do not confuse undoing a UI row with reverting filesystem changes.
