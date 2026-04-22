# Task 110: Claude Code-Style Terminal UI

## Superseded

This active task is superseded by `_tasks/pending/166_Claude_Code_Terminal_Parity_Master_Plan.md`, especially Tasks 167, 170, and 171.

Do not implement this task as written. It is too narrow for the current Claude Code parity goal and still assumes older Elma thinking prefixes/flow. The useful streaming requirements are absorbed into Task 171.

## Problem

Elma's terminal UI doesn't match Claude Code's behavior:
1. No real-time thinking display during generation
2. Activity indicators show but thinking appears AFTER response
3. Missing the visual sequence: User → Analyzing → Thinking → Response

## Target: Claude Code Terminal Behavior

Claude Code (issue #30660) shows:
1. **Activity spinner** while model is "thinking" - labeled
2. **Thinking stream** appears IN REAL-TIME as tokens arrive
3. **Thinking block** with dim gray styling, collapsible
4. **Final response** appears after thinking completes

## Required Changes

### 1. Enable SSE Streaming in API calls
- Use `stream: true` in chat requests
- Parse SSE chunks incrementally
- Emit events as thinking/content arrives

### 2. Add Thinking Display to UI (real-time)
- Create/update thinking entry as chunks arrive
- Push to transcript during generation, not after
- Show with `~ ` prefix in dim gray

### 3. Activity + Thinking Flow
```
User: hi
→ [Spinner: Analyzing | Processing...]  ← current (correct)
→ [Spinner: Thinking | Reasoning...] ← ADD - show during API call
→ ~ thinking text...                 ← ADD - stream in real-time  
→ ● Final response                  ← after thinking done
```

### 4. Fix Activity Message
Current: "Processing your request..." 
Target: "Analyzing..." / "Thinking..." / "Responding..." per stage

## Implementation Plan

1. **Modify request_chat_final_text** (`src/orchestration_helpers/mod.rs`):
   - Set `stream: true` in request
   - Use SSE streaming with callback
   - Yield content/thinking chunks as they arrive

2. **Add streaming callback to TerminalUI**:
   - `on_content_chunk(&str)` - append to assistant entry
   - `on_thinking_chunk(&str)` - create/update thinking entry

3. **Update app_chat_loop** to show thinking during API call:
   - Start streaming request
   - Update UI as chunks arrive
   - Activity shows "Thinking..." during generation

4. **Clean up thinking extraction**:
   - Remove broken code from earlier attempts
   - Keep only `isolate_reasoning_fields` for non-streaming fallback

## Files to Change

- `src/orchestration_helpers/mod.rs` - streaming request
- `src/ui_terminal.rs` - callback handlers
- `src/ui_state.rs` - thinking entry updates
- `src/app_chat_loop.rs` - streaming flow

## Verification

- [ ] Build with `cargo build`
- [ ] Tests pass with `cargo test`
- [ ] Run with local model and verify:
  - Activity shows "Thinking..." during generation
  - Thinking text appears in real-time with ~ prefix
  - Final response appears after thinking

## Notes

- NO TypeScript migration needed - Rust can do SSE streaming
- Use reqwest's `stream` feature (already enabled)
- Keep fallback for non-streaming APIs
