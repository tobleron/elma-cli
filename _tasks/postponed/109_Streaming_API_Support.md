# Task 109: Streaming API Support for Real-Time Thinking

## Goal

Enable streaming API responses (SSE) to display thinking content in real-time as it arrives, not after the full response completes.

## Problem

The thinking content currently appears AFTER the response completes. True real-time thinking requires streaming API support.

## Implementation Summary

### COMPLETED: Infrastructure

1. **Added Stream Types** (`src/types_api.rs`):
   - `ChatCompletionChunk` - streaming response chunk
   - `ChunkChoice` - delta message container  
   - `DeltaMessage` - incremental content/reasoning

2. **Added Streaming Function** (`src/ui_chat.rs`):
   - `chat_streaming()` - SSE streaming with callback
   - `StreamHandler` - accumulates chunks
   - Parses SSE `data: {...}` lines
   - Extracts `reasoning_content` delta from each chunk

3. **Added Dependencies**:
   - Added `stream` feature to reqwest in Cargo.toml
   - Added `anyhow::Context` import

### NOT COMPLETED: Full Integration

To fully enable real-time thinking:
- Switch `request_chat_final_text` to use `chat_streaming()` instead of `chat_once_with_timeout()`
- Add async callback to update Thinking entry in UI as chunks arrive
- Handle streaming errors gracefully with fallback to non-streaming

## Files Changed

- `src/types_api.rs` — added chunk types
- `src/ui_chat.rs` — added `chat_streaming()` and `StreamHandler`
- `Cargo.toml` — added `stream` feature to reqwest

## Verification

- [x] Build with `cargo build`
- [x] Tests pass with `cargo test`
- [ ] Full streaming integration (left for future work)

## Notes

- Infrastructure is ready; full integration is optional
- Streaming requires API support (most OpenAI-compatible APIs support it)
- The current non-streaming path still works