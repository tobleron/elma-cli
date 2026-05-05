# 632 Async Transcript Disk Writes

## Summary
`push_message` writes to `session.md` and `terminal_transcript.txt` synchronously in the hot event-processing path, blocking the UI thread on disk I/O.

## Affected Files
- `src/claude_ui/claude_render.rs:161` — `push_message` clones msg, acquires Mutex, writes to two files
- `src/claude_ui/claude_render.rs:178` — `msg.clone()` for all messages (tool outputs, assistant content)

## Current Behavior
- Every message pushed (thinking deltas, tool traces, assistant content) triggers:
  1. `msg.clone()` of potentially large string data
  2. `trace_log_state().lock()` — Mutex acquisition
  3. `append_session_markdown()` — file open/write/close
  4. `append_terminal_transcript()` — file open/write/close
- All inline in the UI event processing path

## Proposed Fix
- Buffer transcript entries into a `Vec<String>` per session
- Flush to disk on a debounced schedule (every 500ms) or at turn boundaries
- Or spawn writes to a background task via `tokio::spawn`
- Avoid `msg.clone()` by consuming moved values or using references
- Use `append` mode file handle kept open for the session instead of open/close per line

## Estimated CPU Savings
Eliminates UI thread blocking on disk I/O; negligible sustained CPU change

## Status
PENDING
