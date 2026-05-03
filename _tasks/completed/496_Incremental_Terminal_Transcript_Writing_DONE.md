# Task 496: Incremental Terminal Transcript Writing

**Status:** pending
**Priority:** high
**Primary surfaces:** `src/ui/ui_terminal.rs`, `src/session_write.rs`, `src/claude_ui/claude_render.rs`
**Depends on:** Task 495 (truncation prevents memory spike per-event)
**Related issues:** Terminal emulator crashes during cleanup memory spike

## Objective

Eliminate the cleanup-time memory spike entirely by appending each event to `terminal_transcript.txt` as it occurs, instead of building a single giant `String` during `cleanup()`.

## Problem

`cleanup()` at `ui_terminal.rs:2260-2338` loops over ALL `ClaudeMessage` entries and concatenates them into one `String` via `out.push_str()`, then calls `std::fs::write(tpath, out)`. This:

1. Doubles memory: transcript exists in `self.claude.transcript.messages` AND in the `out` String
2. Spikes at cleanup time (when macOS may already be under pressure from the active TUI session)
3. Loses crash resilience: if cleanup is interrupted, the transcript is lost

By contrast, `session.md` already writes incrementally via `append_session_markdown()` at `session_write.rs:311`, called from `claude_render.rs:216` on every `push_message()`.

## Implementation Plan

1. Add `append_terminal_transcript(session_root: &Path, msg: &ClaudeMessage)` in `session_write.rs`, following the same pattern as `append_session_markdown`:
   - User messages → `"> {content}\n\n"`
   - Assistant messages → `"● {content}\n\n"`
   - Thinking → `"∴ Thinking: {content}\n\n"` (truncated to 200 chars)
   - ToolTrace → truncated output preview (relies on Task 495 helpers)
   - Other variants → same format as current `cleanup()` loop
   - Use atomic append: `OpenOptions::new().create(true).append(true)`

2. Call `append_terminal_transcript()` from `ClaudeRenderer::push_message()` in `claude_render.rs` (which already calls `append_session_markdown`).

3. In `cleanup()`, replace the `for msg in &self.claude.transcript.messages` loop with just writing the header:
   ```rust
   let header = format!("=== Terminal Transcript ({}) ===\n\n", chrono::Local::now().to_rfc3339());
   // Prepend header to existing file, or write new file with header
   ```

4. Also call `append_terminal_transcript()` from `handle_event()` for events that bypass `push_message()` (ToolTrace updates, etc.).

## Success Criteria

- `cleanup()` produces no memory allocation proportional to transcript size
- `terminal_transcript.txt` content is identical to current batch output format
- Session restart or crash mid-session preserves partial transcript
