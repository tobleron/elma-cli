# Task 667: Replayable Trace Reducer And Raw Payload Bundle

**Status:** pending
**Priority:** HIGH
**Type:** Observability / Architecture
**Scope:** `src/event_log.rs`, `src/session_display.rs`, `src/session_write.rs`, `src/tool_loop.rs`
**Source:** `_knowledge_base/_source_code_agents/codex-cli/codex-rs/rollout-trace/`

## Summary

Create a trace bundle with append-only raw events, raw payload references, and a deterministic reducer that produces a semantic timeline for debugging and UI replay.

## Evidence And Gap

- Elma has `event_log.rs`, `trace_debug.log`, `reasoning_audit.jsonl`, `session.json`, and transcript files, but no reducer that validates lifecycle completeness.
- Codex rollout-trace separates raw events from reduced graph state and validates replay.
- Session forensics becomes much more reliable if the timeline is mechanically reducible.

## Implementation Plan

1. Define a raw trace event schema with sequence, turn id, wall time, event kind, and payload refs.
2. Store model requests/responses, tool args/results, compaction checkpoints, and transcript notices as raw payloads when large.
3. Implement a reducer that builds a typed turn timeline and detects missing terminal events.
4. Cache reduced timeline next to the session for fast browsing.

## Acceptance Criteria

- [ ] Raw events are append-only and ordered.
- [ ] Reduced timeline can be rebuilt from raw events.
- [ ] Reducer fails on unmatched tool start/end or incomplete finalization unless explicitly aborted.
- [ ] UI/session browser can display reduced timeline summaries.

## Verification Plan

Run reducer tests on synthetic traces and a real session produced by a complex prompt.

