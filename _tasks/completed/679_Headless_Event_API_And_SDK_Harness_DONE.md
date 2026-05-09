# Task 679: Headless Event API And SDK Harness

**Status:** pending
**Priority:** MEDIUM
**Type:** Architecture / Test Coverage
**Scope:** `src/app_chat_loop.rs`, `src/tool_loop.rs`, `src/event_log.rs`, `src/session_store.rs`
**Source:** deferred task 478, user priority on trace/session debugging

## Summary

Expose a local headless runner/API that executes sessions and emits JSONL events for tests, replay, and automation without a TTY.

## Evidence And Gap

- Interactive UI and real CLI are the authority, but tests need deterministic headless access to the same runtime path.
- Deferred Task 478 proposed JSONL/SSE event APIs; JSONL should come first to stay local/simple.

## Implementation Plan

1. Add a headless session runner that uses the same tool loop, permissions, event log, and session persistence as interactive mode.
2. Emit JSONL events for input, route/complexity, model request, tool lifecycle, notices, final answer, and stop reason.
3. Support permission callbacks and scripted approvals/denials.
4. Add SDK-style tests for multi-turn, tool-control, cancellation, and resume flows.

## Acceptance Criteria

- [ ] Headless runs generate the same session artifacts as TTY runs.
- [ ] JSONL events are stable and versioned.
- [ ] Tests can run complex prompts without screen scraping.
- [ ] Permission decisions are auditable.

## Verification Plan

Run headless scenarios and compare reduced timelines with interactive-session artifacts where practical.

