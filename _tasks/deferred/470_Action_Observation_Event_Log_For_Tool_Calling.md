# Task 470: Action Observation Event Log For Tool Calling

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 4-6 days
**Depends on:** completed Task 381, completed Task 430, completed Task 446 (Phase 4 transcript visibility), Task 469
**References:** completed Task 338, `src/tool_loop.rs`, `src/tool_calling.rs`, `src/permission_gate.rs`, `src/evidence_ledger.rs`, `src/claude_ui/`

## Problem

Elma currently has several overlapping runtime records:

- chat messages sent to the model
- transcript rows shown in the TUI
- trace/debug lines
- evidence ledger entries
- tool execution results
- permission prompts
- stop-policy outcomes
- session JSON/markdown persistence

These records are useful, but fragmented. When behavior regresses, it is hard to reconstruct the exact sequence:

1. what the model attempted
2. what policy allowed or blocked
3. what tool actually ran
4. what observation came back
5. what evidence became available
6. why the loop stopped
7. what final answer was produced

Task 338 documented this need, but the event-log implementation is not present in the current rollback state. Reintroduce it for the current JSON/tool-calling architecture.

## Objective

Create a typed action-observation event log as the canonical runtime timeline for each session turn.

The event log should drive or cross-check:

- transcript-native operational visibility
- evidence grounding
- stop reason visibility
- permission/audit history
- session replay/debugging
- certification suites in Task 471

## Non-Goals

- Do not reintroduce DSL `AgentAction` events.
- Do not replace the user-facing transcript with raw event dumps.
- Do not add another default root-level session file if Task 430 has not landed.
- Do not store large raw outputs in the event log.
- Do not make event logging depend on UI structs.
- Do not edit `src/prompt_core.rs`.

## Event Model

Add a focused module such as `src/event_log.rs`.

Suggested event families:

```rust
pub enum AgentEvent {
    Lifecycle(LifecycleEvent),
    Model(ModelEvent),
    Tool(ToolEvent),
    Policy(PolicyEvent),
    Evidence(EvidenceEvent),
    Transcript(TranscriptEvent),
    Finalization(FinalizationEvent),
}
```

Suggested concrete events:

- `SessionStarted`
- `TurnStarted`
- `ModelRequestStarted`
- `ModelToolCallProposed`
- `ToolStarted`
- `ToolFinished`
- `PermissionRequested`
- `PermissionGranted`
- `PermissionDenied`
- `PolicyBlocked`
- `EvidenceRecorded`
- `CompactionStarted`
- `CompactionFinished`
- `StopPolicyTriggered`
- `FinalAnswerPrepared`
- `TurnFinished`
- `SessionEnded`

Each event should include:

- stable event id
- session id
- turn id
- monotonic sequence number
- timestamp
- compact summary
- optional structured payload
- optional artifact references

## Storage Contract

After Task 430:

- Store compact events in `session.json.events` or `session.json.event_log`.
- Store large payloads only as artifact references under `artifacts/`.
- `session.md` receives human-readable rows generated from selected events.

Before Task 430:

- Keep event persistence behind one helper so it can be moved into `session.json` cleanly.
- Avoid adding long-term scattered files.

The event log is the runtime timeline, not the full raw transcript. It should be compact and replayable.

## Implementation Plan

### Phase 1: Types And In-Memory Log

1. Define event types independent of UI structs.
2. Add `EventLog` with append-only sequence semantics.
3. Add helpers:
   - `record_event(event)`
   - `events_for_turn(turn_id)`
   - `latest_stop_event()`
   - `tool_events_for_call(tool_call_id)`
4. Add tests for ordering, serialization, and stable event ids.

### Phase 2: Tool Loop Integration

Emit events from:

- `src/tool_loop.rs`
  - model turn start/end
  - proposed tool calls
  - stop-policy outcomes
  - final answer preparation
- `src/tool_calling.rs`
  - tool execution start/end
  - tool failures
  - artifact creation
- `src/permission_gate.rs`
  - request/grant/deny
- `src/evidence_ledger.rs`
  - evidence entries and claim support
- `src/auto_compact.rs`
  - compaction boundaries and summaries

### Phase 3: Transcript Projection

Add a small renderer that projects selected events into user-visible transcript rows.

Rules:

- Operational decisions should be visible but compact.
- Raw event payloads stay collapsed or omitted from normal transcript view.
- Stop reasons and permission decisions must never be trace-only.
- Tool events should preserve current TUI behavior.

### Phase 4: Persistence And Replay

Add persistence through the session writer layer:

- append events atomically
- preserve event order
- avoid duplicate events on retry/re-render
- load events for old session replay where available

Add replay utilities for tests and debugging:

- reconstruct tool timeline
- reconstruct visible operational rows
- verify every `ToolStarted` has a matching `ToolFinished`
- verify every final answer for evidence-required turns has at least one supporting evidence event or an honest insufficient-evidence outcome

### Phase 5: Certification Hooks

Expose helpers for Task 471:

- `assert_tool_events_balanced(events)`
- `assert_permission_resolution(events)`
- `assert_final_answer_grounded(events)`
- `assert_no_hidden_stop(events, transcript)`
- `assert_session_projection_consistent(events, session.md)`

## Files To Audit

| File | Reason |
|------|--------|
| `src/tool_loop.rs` | Model/tool/finalization event emission |
| `src/tool_calling.rs` | Tool execution event emission |
| `src/streaming_tool_executor.rs` | Parallel event ordering |
| `src/permission_gate.rs` | Permission event emission |
| `src/safe_mode.rs` | Policy reason metadata |
| `src/stop_policy.rs` | Stop outcome events |
| `src/evidence_ledger.rs` | Evidence event bridge |
| `src/auto_compact.rs` | Compaction events |
| `src/session_write.rs` | Event persistence |
| `src/claude_ui/claude_state.rs` | Transcript row projection |
| `src/claude_ui/claude_render.rs` | Collapsible event rendering |

## Success Criteria

- [ ] Every model-proposed tool call has a logged proposal event.
- [ ] Every executed tool has balanced start/finish events.
- [ ] Permission prompts have request and resolution events.
- [ ] Stop-policy outcomes are evented and transcript-visible.
- [ ] Evidence ledger additions have corresponding evidence events.
- [ ] Event log serialization round-trips without UI dependencies.
- [ ] Event replay can reconstruct a compact turn timeline.
- [ ] Large raw outputs are artifact references, not event payloads.
- [ ] Certification helpers can detect missing tool finishes, hidden stops, and unsupported finals.

## Verification

```bash
cargo build
cargo test event_log
cargo test tool_loop
cargo test tool_calling
cargo test permission_gate
cargo test evidence_ledger
cargo test session
```

Manual smoke:

1. Run a read/search request and verify tool proposal/start/finish/evidence/final events.
2. Deny a permission prompt and verify request/denial/final answer events.
3. Trigger stop-policy behavior and verify the stop event is visible in transcript projection.
4. Replay one turn from events and compare it to the visible transcript rows.

## Anti-Patterns To Avoid

- Do not create a hidden trace replacement.
- Do not duplicate full transcript text in every event.
- Do not couple event serialization to ratatui/Claude UI types.
- Do not make event replay responsible for executing tools.
- Do not emit policy or routing details only to debug logs.
