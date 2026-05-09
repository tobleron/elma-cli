# Task 664: Session Rewind Checkpoint Restore And Compaction Recovery

**Status:** pending
**Priority:** HIGH
**Type:** Reliability / UX Internals
**Scope:** `src/session_store.rs`, `src/session_browser.rs`, `src/auto_compact.rs`, `src/snapshot.rs`, `src/claude_ui/`
**Source:** deferred task 472, old checkpoint tasks, `_knowledge_base` Roo rewind-after-condense tests

## Summary

Add reliable session rewind and checkpoint restore that works across compaction boundaries and workspace snapshots.

## Evidence And Gap

- Auto-compaction and snapshots exist but are not tied to a user-facing rewind/restore workflow.
- Roo-Code has explicit rewind behavior after condense in the knowledge base.
- Enterprise-grade local CLI sessions need recovery from bad turns, accidental edits, and compaction drift.

## Implementation Plan

1. Define checkpoint records containing conversation item ids, event sequence, workspace snapshot id, runtime tasks, and compaction state.
2. Add commands/UI events to list, preview, and restore checkpoints.
3. Ensure restoring after compaction reinstalls the correct effective history and summary state.
4. Add safety prompts for workspace restore operations.

## Acceptance Criteria

- [ ] Rewind removes later conversation/tool/runtime task state consistently.
- [ ] Workspace rollback can be tied to a checkpoint.
- [ ] Compaction summaries remain coherent after restore.
- [ ] Restore operations are visible in transcript and session events.

## Verification Plan

Run synthetic sessions with compact -> edit -> rewind -> resume and compare state.

