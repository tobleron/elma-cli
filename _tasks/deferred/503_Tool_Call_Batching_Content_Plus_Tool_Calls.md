# Task 503: Tool Call Batching — Preserve Content Alongside Tool Calls

**Status:** completed
**Priority:** HIGH
**Primary surfaces:** `src/tool_loop.rs`

## Objective (IMPLEMENTED at `src/tool_loop.rs:1073-1086`)

Model narrative text alongside tool calls is now preserved in the conversation history. Before the per-tool-call loop, a content-only assistant message is pushed if `turn.content` is non-empty. This allows the model to narrate progress and execute tools in the same turn, eliminating the need for separate `respond` calls.

## Objective

Currently, when the model emits both text content AND tool calls in a single response, the content is discarded. Only the tool calls are kept. This forces the model to emit a separate `respond` tool call (consuming an extra iteration) whenever it wants to narrate progress.

Fix: preserve the `content` field alongside `tool_calls` so the model can think aloud AND execute tools in the same turn. This collapses the 50+ respond-separate iterations seen in the session into ~5-8 combined turns.

## Root Cause

At `tool_loop.rs:903-970`, the `request_tool_loop_model_turn_streaming()` function parses the model response. The `ToolLoopModelTurn` struct stores only `content` OR `tool_calls`, but not both. When `tool_calls` is non-empty, `content` is dropped.

## Implementation

1. **Add `content` field to `ToolLoopModelTurn`** alongside `tool_calls`:
   ```rust
   pub(crate) struct ToolLoopModelTurn {
       pub content: String,
       pub tool_calls: Vec<ToolCall>,
       pub reasoning_content: Option<String>,
   }
   ```
   (This field already exists — ensure it's preserved when `tool_calls` is non-empty.)

2. **In the model response parser** (`request_tool_loop_model_turn_streaming`): when the stream yields a turn with both `content` and `tool_calls`, set both fields on the result. Currently the code checks `tool_calls.is_empty()` and discards `content` if tool calls exist.

3. **In the main tool loop** (around line 972-1100): when appending the model's turn to the `messages` vector, include the `content` as a regular assistant text message even when tool calls are present. The current code at `tool_loop.rs:1166-1173` appends the message with content only if no tool calls exist. Change to append content AND tool calls as a single assistant message.

4. **Update `ChatMessage` construction**: The OpenAI-compatible `ChatMessage` already supports both `content` and `tool_calls` in the same message. Set both.

## Files to Modify

- `src/tool_loop.rs:903-970` — parser: preserve content when tool_calls present
- `src/tool_loop.rs:1166-1173` — message appending: add content even when tool_calls exist

## Verification

- Run the "read all docs" prompt. First tool loop should show <20 iterations instead of 59.
- Each turn should show content text (narrative reasoning) plus tool calls in the same turn.
- No regression in existing tests.
- `cargo build && cargo test && cargo clippy` passes.
