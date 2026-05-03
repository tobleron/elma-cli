# Task 338: Formal Action-Observation Event Log

**Status:** active — types, emissions, global init, event_log table wired. Persist/replay + tests added.
**Source patterns:** OpenHands event stream, Hermes trajectory logs, Crush pubsub architecture
**Depends on:** Task 304 (transcript operational visibility), Task 345 (versioned extension state)

## Summary

Introduce a typed action-observation event log as the canonical runtime record for model actions, tool calls, tool observations, policy decisions, compaction, finalization, and failures.

## Why

Elma currently has session storage, traces, transcript rows, evidence entries, and tool execution state. These are useful but fragmented. Reference agents with strong recovery and debugging use a single typed event stream that can be replayed, inspected, and persisted.

## Implementation Plan

1. Define `AgentEvent`, `ActionEvent`, `ObservationEvent`, `PolicyEvent`, and `LifecycleEvent` types.
2. Emit events from the tool loop, shell preflight, permission gate, compaction, finalizer, and session store.
3. Render visible collapsible rows from events instead of ad hoc UI state where feasible.
4. Persist events to SQLite with a schema version.
5. Provide replay utilities for tests and debugging.

## Success Criteria

- [x] Every tool call has a matching observation event.
- [x] Compaction, routing/formula choice, stop reason, and permission decisions are represented as events.
- [~] Session resume can reconstruct transcript-visible operational rows. — persist/replay functions exist; needs integration wiring.
- [x] Existing transcript UX remains stable.
- [x] Tests cover event ordering and replay of a small session.

## Anti-Patterns To Avoid

- Do not create another hidden trace-only channel.
- Do not put execution mode or routing detail in the bottom status bar.
- Do not make event serialization depend on UI structs.
