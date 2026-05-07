# Task 746: Harden tool_loop.rs Stream Parsing Against Malformed SSE Chunks

## Type

Reliability / Resilience / Error Handling

## Severity

High

## Scope

Tool loop, SSE streaming, JSON parsing

## Problem

`src/tool_loop.rs` contains complex SSE stream parsing logic (lines 94-200+) that processes streaming tool call deltas. The parsing is brittle:

1. **Silent JSON parse failures**:
   ```rust
   let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) else {
       continue;
   };
   ```
   If a chunk is malformed JSON, it's silently skipped. The model may have sent a critical tool call delta that is lost.

2. **No chunk boundary validation**: the buffer accumulation (`buffer.push_str(&String::from_utf8_lossy(&chunk_bytes))`) assumes chunks are complete lines, but SSE chunks can split across TCP packets.

3. **No max buffer size**: `buffer` can grow unbounded if the stream sends malformed data without newlines.

4. **Silent stream errors**:
   ```rust
   Err(error) => {
       append_trace_log_line(&format!("[TOOL_LOOP_STREAM_ERROR] {}", error));
       break;
   }
   ```
   A stream error breaks the loop but doesn't inform the user or model.

5. **Assumes `choices[0]`**: `chunk.get("choices").and_then(|c| c.as_array())` gets the first choice without checking if choices exist.

6. **Reasoning content extraction is fragile**:
   ```rust
   let reasoning = delta
       .get("reasoning_content")
       .or_else(|| delta.get("reasoning"))
       .or_else(|| delta.get("thought"))
       .and_then(|v| v.as_str())
       .map(crate::claude_ui::strip_thinking_tags_preserve_spacing)
       .unwrap_or_default();
   ```
   This silently falls back if the provider uses a different field name.

This violates:
- **Rule 2**: reliability over speed — silently dropping chunks is not reliable
- **Rule 11**: eliminate crashes and parse failures
- **Rule 13**: add real timeout mechanisms and sanitize inputs

## Root Cause

SSE parsing was written for ideal conditions and hardened incrementally with `continue` and `break` rather than structured error recovery.

## Proposed Solution

### Phase 1 — Structured chunk parsing

Replace inline parsing with a dedicated `SseChunkParser`:

```rust
struct SseChunkParser {
    buffer: String,
    max_buffer_len: usize,
    chunks_parsed: usize,
    chunks_failed: usize,
}

impl SseChunkParser {
    fn push_bytes(&mut self, bytes: &[u8]) -> Vec<Result<SseChunk, SseParseError>>;
    fn flush(&mut self) -> Vec<Result<SseChunk, SseParseError>>;
}
```

### Phase 2 — Fail loudly on critical errors

1. If `chunks_failed > chunks_parsed / 2`, abort the turn and report: "Stream parsing failed: most chunks were malformed"
2. If `buffer.len() > max_buffer_len`, abort and report: "Stream buffer overflow — provider sent unparseable data"
3. Stream errors should produce a `ToolLoopError::StreamFailure` that propagates to the user

### Phase 3 — Validate chunk structure

1. After JSON parse, validate the chunk has the expected structure:
   - `choices` is a non-empty array
   - `choices[0].delta` exists
   - `delta` has at least one of: `content`, `tool_calls`, `reasoning_content`
2. If a chunk lacks all expected fields, trace it but don't treat it as a failure unless it happens repeatedly

### Phase 4 — Provider adapter

1. Move reasoning field extraction to `src/llm_provider.rs`:
   - OpenAI: `delta.content` + `delta.tool_calls`
   - Anthropic: `delta.content` + `delta.thinking` (or provider-specific fields)
   - Generic: configurable field mapping
2. `tool_loop.rs` should not hardcode field names; it should use the provider's delta format

### Phase 5 — Tests

1. Add unit tests for `SseChunkParser`:
   - Split chunks across multiple byte arrays
   - Malformed JSON chunks
   - Buffer overflow
   - Empty chunks
   - Missing `choices` array

## Acceptance Criteria

- [ ] `SseChunkParser` exists with structured error handling
- [ ] Malformed chunks are not silently skipped; they're logged and counted
- [ ] If > 50% of chunks fail, the turn aborts with a clear error
- [ ] Buffer has a max size and overflows are handled
- [ ] Stream errors propagate to the user, not just trace logs
- [ ] Provider-specific field extraction lives in `llm_provider.rs`
- [ ] `cargo build && cargo test` passes
- [ ] Unit tests cover split chunks, malformed JSON, and buffer overflow

## Verification Plan

- Unit test: chunk split across 3 byte arrays → correctly reassembled
- Unit test: 3 malformed chunks + 2 valid → valid parsed, malformed counted
- Unit test: 5 malformed + 1 valid → turn aborts
- Unit test: buffer exceeds 1MB → overflow error
- Integration test: mock provider sends malformed SSE → user sees error message

## Dependencies

- `src/tool_loop.rs` (primary site)
- `src/llm_provider.rs` (provider adapter)
- `src/sse_stream.rs` (Task 558 — SSE parsing)

## Notes

This is about **resilience**, not perfection. The goal is to detect when the stream is broken and stop gracefully, not to parse every possible malformed chunk.

The `continue` on JSON parse failure is especially dangerous because it can cause the model to "lose" a tool call. If the model sent `{"tool_calls": [...]}` but the JSON was malformed, the tool call is silently dropped, and the model waits for a result that never comes.

If `sse_stream.rs` (Task 558) already has a parser, use it. Don't create a second parser. The issue may be that `tool_loop.rs` doesn't use `sse_stream.rs` — if so, this task is about integration.
