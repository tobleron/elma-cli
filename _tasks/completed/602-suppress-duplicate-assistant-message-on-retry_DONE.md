# Task 602: Suppress Duplicate Assistant Message When Continuity Retry Replaces Answer

## Type

Bug

## Severity

High

## Scope

Session-specific

## Session Evidence

Session `s_1777831852_914805000` trace_debug.log:

- `continuity_score=0.78 needs_fallback=false last_stage=finalization`
- Two HTTP calls: (1) continuity retry 120s/2896 bytes, (2) turn summary 15s/1703 bytes

The `app_chat_loop.rs` flow:
1. Tool loop streams the model's wrong answer to TUI via `AssistantContentDelta` + `AssistantFinished`
2. Continuity retry at line 1022 fires (score 0.78 < 0.85)
3. Retry produces a corrected answer at line 1064
4. Line 1091: `tui.add_message(MessageRole::Assistant, display_text)` adds the retried answer as a **new** message

The user sees two separate assistant messages: the streamed original and the retry result. This is the "second response about the workspace" the user reported.

## Problem

When the continuity retry fires, two assistant messages appear in the TUI:
1. The original wrong answer (streamed live from the tool loop)
2. The continuity retry result (added as a separate message at line 1091)

This is confusing: the user sees the original answer, then a new "corrected" answer appears below it. If the retry answer is similar, the user sees an unexplained duplicate. If different, they wonder which to trust.

Additionally, the streaming of the original answer wastes bandwidth — the user reads a wrong answer that gets discarded by the retry.

## Root Cause Hypothesis

**Confirmed**: The tool loop streams the model's answer as it's generated (`tool_loop.rs:230`). After the tool loop completes (`AssistantFinished` at line 249), the continuity retry may fire and produce a *different* final answer. At line 1091, this new answer is pushed as a separate TUI message, but the streamed answer is already visible and can't be removed.

**Confirmed**: The continuity retry replaces `final_text` at line 1064 for programmatic purposes (runtime.messages, artifact files), but the TUI already displayed the original streamed answer.

## Proposed Solution

### Option A: Suppress streaming when continuity retry may fire

Defer `AssistantContentDelta` and `AssistantFinished` until after the continuity check. Instead of streaming content to TUI during the tool loop, buffer it. After continuity check:

- If no retry needed: flush the buffered content to TUI
- If retry needed: discard the buffered content, show the retry result only

Implementation sketch:

```rust
// In tool_loop.rs: Instead of tui.handle_ui_event(AssistantContentDelta(delta)),
// buffer delta into a tui_stream_buffer
// Return both final_answer and tui_stream_buffer from the tool loop

// In app_chat_loop.rs, after continuity check:
if !tool_loop_result.tui_stream_buffer.is_empty() {
    for delta in tool_loop_result.tui_stream_buffer {
        tui.handle_ui_event(AssistantContentDelta(delta));
    }
}
tui.handle_ui_event(AssistantFinished);
```

### Option B: Overwrite last message on retry

When continuity retry fires, remove the already-streamed message and show only the retried answer. Requires the TUI to support message removal or replacement.

```rust
// In app_chat_loop.rs, line 1091:
if continuity_retry_fired {
    tui.replace_last_message(MessageRole::Assistant, display_text);
} else {
    tui.add_message(MessageRole::Assistant, display_text);
}
```

This requires adding a `replace_last_message` method to `TerminalUI`.

### Option C: Suppress continuity retry entirely when the answer was streamed

If the tool loop already produced a complete answer (the model stopped calling tools voluntarily, not by stop policy), skip the continuity retry entirely. The continuity check would still fire, but the retry would be skipped.

This is fragile — we want the continuity retry to improve bad answers, just not at the cost of duplication.

## Acceptance Criteria

- [ ] When continuity retry fires, only ONE assistant message is visible in the TUI
- [ ] The visible message is the continuity retry result, not the original streamed answer
- [ ] When continuity retry does NOT fire, streaming works as before
- [ ] No regression in artifact files (session.md, final_answer artifact) — the correct (retried) answer is saved

## Verification Plan

1. Create a test fixture where the model produces a low-scoring answer (score < 0.85)
2. Verify the TUI shows exactly one assistant message after continuity retry
3. Verify the session artifact contains the retried answer, not the original
4. Create a test fixture where the model produces a high-scoring answer (no retry)
5. Verify streaming works normally in this case

## Dependencies

Task 601 (evidence consistency) or Option C — if continuity retry is suppressed when the model had sufficient evidence, this task may not be needed.

## Notes

The `TerminalUI` does not currently support message replacement. Option B would require either:
- Adding a `replace_last_message` method
- Or reworking the message list to support undo/pop operations

The simpler approach is Option A: buffer tool loop streaming output and flush it after continuity check. This avoids TUI API changes.
