# Task 495: Truncate Tool Output In Terminal Transcript

**Status:** pending
**Priority:** high
**Primary surfaces:** `src/ui/ui_terminal.rs:2260-2338`
**Related issues:** Sessions s_1777743795_254752000 and s_1777744940_287543000 terminated the terminal emulator during cleanup

## Objective

Prevent terminal emulator crashes by capping tool output written to `terminal_transcript.txt` at 1024 characters instead of dumping the full tool output (which can be 128KB+ per `read` call).

## Problem

`cleanup()` at `ui_terminal.rs:2260-2338` iterates all `ClaudeMessage` entries and writes them into a single `String` before atomically writing to `terminal_transcript.txt`. Two `ClaudeMessage` variants write the **untruncated** `output` field:

**`ToolResult` (legacy)** at lines 2281-2290:
```rust
out.push_str(&format!(
    "✓ Tool result ({}): success={} duration_ms={:?}\n{}\n\n",
    name, success, duration_ms, output   // ← full 128KB
));
```

**`ToolTrace` (current)** at lines 2303-2311:
```rust
out.push_str(&format!(
    "status: completed success={} duration_ms={:?}\n{}\n\n",
    success, duration_ms, output   // ← full 128KB
));
```

With 6 `read` tool calls returning 50–128KB each, the transcript string reaches **357KB+**. This memory spike during `cleanup()` pushes macOS memory pressure, killing the terminal emulator.

The full tool output already persists to `artifacts/tool-results/<call_id>.txt` — there is no need to duplicate it in the transcript.

By contrast, `session.md` writing in `session_write.rs:336-338` already caps at **200 chars**:
```rust
let preview: String = output.chars().take(200).collect();
```

## Implementation Plan

1. Add a helper function `transcript_tool_output_preview(output: &str) -> String` that returns `output.chars().take(1024)` plus a `"…"` suffix if truncated.

2. Replace the `{output}` usage in `ToolResult` (lines 2287-2290) and `ToolTrace::Completed` (lines 2308-2311) with the preview.

3. Verify: `cargo build && cargo test -- ui_terminal`.

## Success Criteria

- Re-run the "read all docs and compare with source code" prompt
- `terminal_transcript.txt` file size is **< 50KB** (was 357KB)
- No terminal emulator crash
