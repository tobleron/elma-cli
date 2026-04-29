# Task 283: Session Transcript Flush On Tool Results And PTY Transcripts

**Status**: Pending  
**Priority**: High  
**Depends on**: T281 (transcript persistence infrastructure)  
**Elma Philosophy**: Incremental reliability, crash-safe, preserve all evidence

## Goal

Ensure tool outputs, PTY transcripts, and shell command results are flushed to the session's `display/terminal_transcript.txt` and `artifacts/` immediately when available—not just at session exit.

This reduces data loss risk on crashes and enables real-time log inspection during long-running operations.

## Requirements

### Part 1: Tool Result Flush

When a tool completes (in `src/tool_loop.rs`):
- Append the full ToolResult to `display/terminal_transcript.txt` with timestamp
- Also write the output to `session/artifacts/tool_<name>_<tool_id>.txt` for quick access
- Use atomic writes (temp file + rename) to avoid corruption on crash

**Implementation**:
```rust
// In tool_loop.rs after tool completes:
append_to_session_transcript(&session_root, &tool_name, &output)?;
write_tool_artifact(&session_root, &tool_name, &tool_id, &output)?;
```

### Part 2: PTY Transcript Flushing

In `src/program_utils.rs` (pty/shell capture):
- After each `CaptureOutput` completes, write the sanitized PTY transcript to `session/artifacts/pty_<timestamp>.txt`
- Append a summary line to `display/terminal_transcript.txt`
- Use buffered writes for efficiency

**Implementation**:
```rust
// In capture_pty_output() or similar:
flush_pty_transcript(&session_root, &sanitized_bytes, &duration_ms)?;
```

### Part 3: Streaming Writes

For long-running commands (e.g., `find`, `du`, large file processing):
- Every 5 seconds or on newline boundaries, write incremental output to `artifacts/`
- Keep a rolling buffer in memory to batch writes
- Use non-blocking I/O to avoid blocking the TUI

**Implementation**:
- Add a `StreamingArtifactWriter` struct that:
  - Buffers output lines
  - Flushes every 5 seconds or on buffer full (16KB)
  - Appends to artifact file atomically

### Part 4: Wire Into `src/ui/ui_terminal.rs` Cleanup

Update the cleanup path to:
- Verify all tool results have been flushed
- Write a session summary to `session/session_summary.txt` (timestamp, tool count, total lines, etc.)
- Checksum the transcript to detect corruption

## Non-Requirements (Out of Scope)

- Real-time tail of artifacts (handled by user directly via `tail -f`)
- Log rotation (handled by task 282 garbage collector)
- Streaming to remote server

## Testing

- [ ] Write a tool result and verify it appears in `display/terminal_transcript.txt` within 1 second
- [ ] Run a long `find` command and verify intermediate results appear in `artifacts/` every 5 seconds
- [ ] Simulate crash during tool execution and verify output is recoverable from `artifacts/`
- [ ] Verify no duplicate lines when transcript is appended on retry

## Acceptance Criteria

1. Tool output is flushed to `display/terminal_transcript.txt` within 1 second of tool completion
2. PTY transcripts are written to `artifacts/pty_*.txt` incrementally
3. Streaming writes happen every 5 seconds without blocking the TUI
4. Crash during tool execution results in ≥99% of output being recoverable
5. No data corruption from concurrent writes

## Notes

- Atomic writes (rename) are critical for crash safety
- Buffering reduces I/O cost and keeps TUI responsive
- This task complements task 282 (garbage collection) by ensuring transcripts are always available
- Follows Elma's incremental reliability principle
