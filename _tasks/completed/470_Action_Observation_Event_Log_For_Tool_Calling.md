# Task 470: Action Observation Event Log For Tool Calling

**Status:** in_progress
**Priority:** HIGH
**Started:** 2026-05-02

## Summary

Creating typed action-observation event log as the canonical runtime timeline for each session turn.

## Progress

### Phase 1: Types And In-Memory Log (Completed)

Created `src/event_log.rs` with:

- **Event types** (7 families):
  - `LifecycleEvent`: SessionStarted, TurnStarted, SessionEnded, TurnFinished
  - `ModelEvent`: ModelRequestStarted, ModelToolCallProposed, ModelResponseReceived
  - `ToolEvent`: ToolStarted, ToolFinished, ToolFailed
  - `PolicyEvent`: PermissionRequested, PermissionGranted, PermissionDenied, PolicyBlocked
  - `EvidenceEvent`: EvidenceRecorded
  - `TranscriptEvent`: TranscriptRowAppended
  - `FinalizationEvent`: StopPolicyTriggered, FinalAnswerPrepared

- **EventLog struct** with:
  - `record()` - append event with sequential IDs
  - `events_for_turn()` - filter by turn
  - `latest_stop_event()` - get most recent finalization
  - `tool_events_for_call()` - get tool start/finish pair

- **Tests**: Sequential IDs, turn filtering, stop event lookup

## Remaining Work

- Phase 2: Tool Loop Integration (emit events from tool_loop.rs, tool_calling.rs, permission_gate.rs, evidence_ledger.rs, auto_compact.rs)
- Phase 3: Transcript Projection (project events to user-visible rows)
- Phase 4: Persistence (save to session.json.events)
- Phase 5: Certification Helpers (assert_tool_events_balanced, assert_final_answer_grounded, etc.)