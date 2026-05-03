# Task 478: Headless Event API And SDK Harness

**Status:** pending
**Source patterns:** Qwen-code SDK tests, Crush server/proto, LocalAGI SSE API
**Depends on:** completed Task 338 (event log), completed Task 339 (tool metadata policy)

## Summary

Expose a local headless API for running Elma sessions with JSONL or SSE events, permission callbacks, tool results, and final answers. Add SDK-style tests that exercise multi-turn and tool-control flows.

## Why

Reference agents test their core logic outside the terminal UI. Elma's UI is important, but a headless event API would make automation, integration tests, and future interfaces more reliable.

## Implementation Plan

1. Add a headless session runner that emits typed events.
2. Support permission callbacks and deterministic test providers.
3. Provide JSONL as the first transport; consider SSE after the core runner is stable.
4. Add integration tests for multi-turn, denied permission, tool failure, and finalization.
5. Keep network servers disabled by default.

## Success Criteria

- [ ] A session can run without TUI initialization.
- [ ] Events include tool calls, observations, policy decisions, and final answer.
- [ ] Permission prompts can be handled programmatically.
- [ ] Integration tests use the headless runner.
- [ ] The API does not require a daemon for local use.

## Anti-Patterns To Avoid

- Do not make a remote daemon the default architecture.
- Do not duplicate core agent logic in the API layer.
- Do not leak secrets through event payloads.
