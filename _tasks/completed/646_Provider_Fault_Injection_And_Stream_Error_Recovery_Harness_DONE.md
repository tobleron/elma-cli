# Task 646: Provider Fault Injection And Stream Error Recovery Harness

**Status:** pending
**Priority:** HIGH
**Type:** Test Coverage / Model Robustness
**Scope:** `src/llm_provider.rs`, `src/tool_loop.rs`, `src/sse_stream.rs`, `src/stream_types.rs`
**Source:** deferred task 473, `_knowledge_base` OpenHands retry and Codex stream handling patterns

## Summary

Add deterministic provider fault-injection tests for streaming failures, invalid JSON chunks, context overflow, timeouts, provider error bodies, and stop-sequence edge cases.

## Evidence And Gap

- `request_tool_loop_model_turn_streaming` manually parses SSE chunks and handles fallback to non-streaming.
- Current reliability depends on live model behavior, making regressions hard to reproduce.
- Deferred Task 473 already identified provider fault injection as necessary.

## Implementation Plan

1. Add fake provider fixtures for stream interruption, malformed SSE frames, malformed tool-call deltas, timeout, 429/500 error bodies, context overflow, and empty choices.
2. Assert retry/fallback decisions and visible notices.
3. Capture partial assistant/reasoning/tool-call content safely without corrupting conversation state.
4. Verify stop reasons and failed provider calls are persisted in session artifacts.

## Acceptance Criteria

- [ ] Fault fixtures run without network.
- [ ] Streaming fallback never duplicates assistant messages.
- [ ] Provider errors surface in transcript and session diagnostics.
- [ ] Context overflow suggests compaction or shorter scope, not a false completed answer.

## Verification Plan

Run `cargo test provider_fault streaming_error tool_loop_fallback -- --nocapture`.

