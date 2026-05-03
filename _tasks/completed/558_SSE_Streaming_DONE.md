# 558 — Decouple SSE Streaming from TUI Event Emission

- **Priority**: High
- **Category**: Refactoring
- **Depends on**: 550 (removes legacy orchestration complexity)
- **Blocks**: 563, 584

## Problem Statement

The SSE streaming functions in `tool_loop.rs` tightly couple three responsibilities:

1. **Network I/O**: Reading SSE byte chunks from the HTTP response stream
2. **Parsing**: Splitting SSE frames, extracting JSON deltas, detecting thinking blocks
3. **UI Updates**: Calling `tui.handle_ui_event()` for every content/thinking delta

The worst offender is `request_tool_loop_model_turn_streaming()` (~170 lines, `tool_loop.rs:93-261`) which interleaves byte stream reading, SSE line parsing, JSON chunk parsing, thinking block detection, and TUI event emission in a single function with nested loops.

This makes it:
- **Untestable**: You cannot test SSE parsing without mocking the entire HTTP layer AND the TUI
- **Unreusable**: The SSE parsing logic cannot be reused for non-TUI contexts (headless mode, testing)
- **Hard to debug**: A TUI issue can look like a parsing issue and vice versa

## Why This Matters for Small Local LLMs

- Small models produce more malformed output — the parsing layer needs robust testing
- Streaming behavior with small models is more erratic — the parsing needs to handle edge cases (incomplete JSON fragments, mid-stream stop tokens)
- TUI responsiveness matters for user experience during long streaming waits with slow local models

## Current Behavior

```rust
// tool_loop.rs - single function does everything
async fn request_tool_loop_model_turn_streaming(
    tui: &mut TerminalUI,
    client: &reqwest::Client,
    chat_url: &Url,
    req: ChatCompletionRequest,
    timeout_s: u64,
    session: &SessionPaths,
) -> Result<ToolLoopModelTurn> {
    // HTTP request
    // SSE byte stream loop
    // Buffer management
    // JSON delta parsing
    // Thinking block detection
    // TUI event emission
    // Content accumulation
}
```

## Recommended Target Behavior

Split into three layers:

1. **SSE Stream Reader** (`sse_stream.rs`): Generic SSE byte stream → `Stream<Item = SseFrame>`
2. **Chat Completion Parser** (`chat_stream_parser.rs`): `Stream<SseFrame>` → `Stream<ChatStreamEvent>`
3. **TUI Bridge** (existing `tool_loop.rs`): Consumes `Stream<ChatStreamEvent>` and emits UI events

```rust
// Layer 1: Generic SSE
async fn read_sse_stream(response: Response) -> impl Stream<Item = Result<SseFrame>>

// Layer 2: Chat-specific parsing (testable without TUI or network)
fn parse_chat_stream_events(frames: impl Stream<Item = SseFrame>) -> impl Stream<Item = ChatStreamEvent>

#[derive(Debug, Clone)]
enum ChatStreamEvent {
    ContentDelta(String),
    ThinkingDelta(String),
    ToolCallDelta { index: usize, id: Option<String>, name: Option<String>, arguments: String },
    ThinkingStarted,
    ThinkingFinished,
    ContentFinished,
    Error(String),
}

// Layer 3: TUI integration
async fn stream_with_tui(
    events: impl Stream<Item = ChatStreamEvent>,
    tui: &mut TerminalUI,
) -> Result<ToolLoopModelTurn>
```

## Source Files That Need Modification

- `src/tool_loop.rs` — Extract SSE/chunk parsing into separate modules; keep only TUI integration
- `src/ui_chat.rs` — May contain similar coupling (audit `chat_once_with_timeout`, `chat_json_with_repair_timeout`)

## New Files/Modules

- `src/sse_stream.rs` — Generic SSE byte stream → frame stream
- `src/chat_stream_parser.rs` — Frame stream → chat event stream
- `src/stream_types.rs` — `SseFrame`, `ChatStreamEvent`, `StreamingToolCallPart` types

## Step-by-Step Implementation Plan

1. Create `src/stream_types.rs` with shared types:
   ```rust
   pub struct SseFrame {
       pub event: Option<String>,
       pub data: String,
   }
   
   pub enum ChatStreamEvent {
       ContentDelta(String),
       ReasoningDelta(String),
       ToolCallDelta(ToolCallDelta),
       ContentStarted,
       ThinkingStarted,
       ThinkingFinished,
       ContentFinished,
       Error(String),
   }
   ```

2. Create `src/sse_stream.rs`:
   - Extract byte stream reading and SSE line parsing from `tool_loop.rs`
   - Return `impl Stream<Item = Result<SseFrame>>`
   - Handle connection errors, incomplete frames, [DONE] markers

3. Create `src/chat_stream_parser.rs`:
   - Parse SSE frames into `ChatStreamEvent` variants
   - Handle reasoning_content, content, tool_calls deltas
   - Handle thinking block detection (think tags within content)
   - Pure function: no network, no TUI, no side effects
   - MUST be independently testable

4. Update `tool_loop.rs`:
   - Consume `ChatStreamEvent` stream
   - Emit TUI events for each event variant
   - Accumulate content and tool calls for final `ToolLoopModelTurn`

5. Update `final_answer` streaming function similarly

6. Run `cargo test` and scenario tests

## Recommended Crates

- `futures` — already a dependency, for `Stream` and `StreamExt`
- `tokio-stream` — already a dependency, for `StreamExt`

## Validation/Sanitization Strategy

- SSE parser must handle: empty data, multi-line data, comments, retry fields, missing newlines
- Chat parser must handle: incomplete JSON fragments, missing fields, unexpected event types
- Both parsers must recover gracefully from malformed input (small model output)

## Testing Plan

1. **SSE parser unit tests**: Feed known SSE byte sequences, verify correct frame extraction
2. **Chat parser unit tests**: Feed known JSON deltas, verify correct `ChatStreamEvent` sequence
3. **Integration test**: Full streaming pipeline with mock HTTP response
4. **Edge case tests**: Truncated frames, mid-stream disconnection, empty deltas, duplicate deltas, overlapping thinking tags
5. **Golden tests**: Save known model responses as SSE fixtures, replay and verify parsing

## Acceptance Criteria

- SSE parsing is a standalone module with no TUI or HTTP dependencies
- Chat stream parsing is a pure function testable without network
- TUI integration uses a clean `Stream<Item = ChatStreamEvent>` interface
- Existing streaming behavior is unchanged (scenario tests pass)
- New unit tests cover SSE parsing and chat parsing independently

## Risks and Migration Notes

- **Performance risk**: Adding intermediate stream layers may add latency. Use zero-copy parsing where possible (borrow from byte buffers).
- **Compatibility risk**: Different LLM providers emit slightly different SSE formats. The parser must handle OpenAI, Anthropic, and compatible formats.
- **Tool call streaming**: Tool call deltas are the most complex part (incremental argument accumulation). Keep `StreamingToolCallPart` logic but move it to the parser layer.
- Do this AFTER Task 550 to avoid refactoring both orchestration paths.
