# Task 657: Tool Execution Event Ledger With Raw Payload References

**Status:** pending
**Priority:** HIGH
**Type:** Observability / Tooling
**Scope:** `src/event_log.rs`, `src/tool_loop.rs`, `src/tool_calling.rs`, `src/tool_result_storage.rs`
**Source:** deferred task 470, `_knowledge_base` Codex rollout-trace design

## Summary

Upgrade tool execution logging into a replayable ledger that stores compact semantic events plus raw payload/artifact references.

## Evidence And Gap

- `event_log.rs` records tool events but only stores tool name/id and optional artifact fields.
- Codex rollout trace separates raw trace bundles from reduced replay state.
- Session forensics needs exact tool args, outputs, status, duration, policy decisions, and evidence references.

## Implementation Plan

1. Add raw payload storage for tool input/output when content is large or sensitive.
2. Store reduced event fields: tool name, call id, status, duration, exit code, timeout, policy decision, artifact refs.
3. Ensure early returns, validation failures, permission denials, and stop-policy outcomes all end their events.
4. Add a replay reducer that reconstructs per-turn tool timeline.

## Acceptance Criteria

- [ ] Every tool call has start and terminal event or an explicit skipped/rejected event.
- [ ] Large raw payloads are referenced, not duplicated into every artifact.
- [ ] Replay detects unmatched starts/ends.
- [ ] Session forensics can answer what tool failed and why without reading debug-only logs.

## Verification Plan

Run tool-loop fixtures with success, validation failure, permission denial, timeout, and stop-policy termination.

