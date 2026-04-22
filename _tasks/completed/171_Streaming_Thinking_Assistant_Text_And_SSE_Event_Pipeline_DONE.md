# Task 171: Streaming Thinking, Assistant Text, And SSE Event Pipeline

## Status
Pending.

## Objective
Make Elma stream model-visible thinking and assistant text into the Claude-style UI in real time, using an OpenAI-compatible SSE pipeline that works with small local models.

## Existing Work To Absorb
This task absorbs:

- `_tasks/postponed/109_Streaming_API_Support.md`
- `_tasks/active/110_Claude_Code_Style_Terminal_UI.md`

The existing `src/ui_chat.rs` streaming foundation is useful but incomplete because the interactive UI is not updated as chunks arrive.

## Claude Source References
- `components/messages/AssistantThinkingMessage.tsx`
- `components/messages/AssistantTextMessage.tsx`
- `query.ts`
- `components/Messages.tsx`
- `components/MessageRow.tsx`

## Requirements
- Use `stream: true` for compatible providers.
- Parse OpenAI-compatible SSE incrementally.
- `request_chat_final_text_streaming` must not only accumulate chunks and return at the end; it must emit UI events as chunks arrive.
- Emit UI events for:
  - turn start,
  - thinking delta,
  - assistant text delta,
  - tool call delta,
  - usage update,
  - finish reason,
  - stream error,
  - fallback non-streaming response.
- Support common reasoning field variants from local OpenAI-compatible servers:
  - `delta.reasoning_content`,
  - `delta.reasoning`,
  - provider-specific visible reasoning fields already modeled in `src/types_api.rs`.
- Do not synthesize hidden reasoning. Only display reasoning text actually returned by the model/provider.
- Do not classify thinking by phrase lists, hardcoded sentence starts, or keyword filters.
- Show thinking while it streams, then preserve/collapse it according to Task 170.
- Show final assistant text after or alongside thinking according to event order.
- If streaming is unsupported, fall back to non-streaming and still render the same final transcript shape.

## Required Integration Shape
Introduce a stream event sink used by the final-answer request path. The exact Rust type can vary, but it must support this behavior:

```rust
enum UiEvent {
    ThinkingStarted,
    ThinkingDelta(String),
    ThinkingFinished,
    AssistantContentDelta(String),
    AssistantFinished,
    StreamError(String),
}
```

The final-answer pipeline must call the sink while the HTTP response is still streaming. It may also return the final accumulated content for conversation history, but the UI must not wait for that return value before updating the screen.

If the provider emits `delta.reasoning_content`, `delta.reasoning`, or supported provider-specific reasoning fields, emit `ThinkingDelta`. If it emits `delta.content`, emit `AssistantContentDelta`. If only non-streaming output is available, emit a complete assistant event sequence after fallback.

## UI Sequence
The default visible sequence should match Claude-style behavior:

```text
> user request

∴ Thinking

● final assistant response
```

In expanded/transcript mode, thinking body is visible and dim/italic where terminal support allows.

## Fake Provider Harness
Add a deterministic fake OpenAI-compatible SSE server for tests. It must be able to stream:

- reasoning chunks only,
- content chunks only,
- interleaved reasoning and content,
- tool call chunks,
- malformed chunks followed by recovery,
- premature disconnect.

## Files To Inspect Or Change
- `Cargo.toml`
- `src/types_api.rs`
- `src/ui_chat.rs`
- `src/orchestration_helpers/mod.rs`
- `src/app_chat_loop.rs`
- `src/app_chat_core.rs`
- `src/ui_state.rs`
- new UI event modules introduced by Task 169.

## Acceptance Criteria
- Thinking appears while the request is still in flight.
- Assistant content appears incrementally without waiting for the full response.
- The active chat loop no longer waits for `resolve_final_text(...).await` to finish before any assistant content becomes visible.
- Thinking text is preserved for Ctrl-O/transcript expansion when the provider supplies reasoning fields.
- `full_thinking` or equivalent accumulated reasoning is either emitted/persisted or removed; it must not be silently collected and discarded.
- Stream errors do not leave the terminal in a broken state.
- Non-streaming providers still work.
- Snapshot and real CLI tests prove the event order.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test streaming -- --nocapture
cargo test ui_parity_streaming -- --nocapture
./ui_parity_probe.sh --fixture thinking-stream
./ui_parity_probe.sh --fixture content-stream
./ui_parity_probe.sh --fixture stream-error-fallback
```

Also run a real CLI pseudo-terminal session against the fake SSE server and verify the thinking row updates before the final assistant row is complete.
