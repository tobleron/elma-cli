# Task 739: Persist Exact Left Chat Window Render In Session Debug Artifacts

## Type

Observability / UI Debugging / Session Trace

## Severity

High

## Scope

`src/claude_ui/claude_render.rs`, `src/claude_ui/claude_state.rs`, `src/ui/ui_terminal.rs`, `src/session_write.rs`, session artifacts, UI snapshot tests

## Problem

The session folder has `terminal_transcript.txt`, but that file is an event-style semantic transcript. It is not an exact record of what the user saw in the left chat pane after wrapping, collapsing, viewport slicing, scroll offset, sticky header behavior, and terminal dimensions are applied.

For troubleshooting Elma's behavior, we need at least one artifact that shows the left chat window exactly as rendered to the user. The right thinking sidebar is already functioning well and should not be the focus of this artifact.

## Proposed Solution

Persist a plain-text render artifact generated from the same left-pane render path used by the TUI.

Create two artifacts:

1. `sessions/<id>/left_chat_render.txt`
   - Final visible left chat pane at session cleanup or after each completed turn.
   - Plain text, ANSI-stripped.
   - Includes terminal width, height, transcript pane rectangle, content width, scroll offset, timestamp, and active turn number.
   - Excludes the right thinking sidebar.

2. `sessions/<id>/left_chat_frames.jsonl`
   - Optional rolling frame log for debugging turn-by-turn changes.
   - Each record includes timestamp, turn, viewport dimensions, scroll offset, event kind, and rendered left-pane lines.
   - Throttle writes so normal streaming does not cause excessive disk I/O.

The artifact must be generated after wrapping and viewport slicing, not from raw messages. If the renderer currently cannot expose this cleanly, add a small pure function that returns the exact left-pane visible lines from the current transcript state and layout inputs.

## Acceptance Criteria

- [ ] Every interactive session writes `left_chat_render.txt`.
- [ ] The file shows the final visible left chat pane exactly as the user saw it, excluding the right thinking sidebar.
- [ ] The file includes viewport metadata sufficient to reproduce wrapping decisions.
- [ ] A throttled `left_chat_frames.jsonl` is available when debug/trace mode is enabled.
- [ ] Existing `terminal_transcript.txt` remains available as an event transcript.
- [ ] The artifact writer is non-blocking or buffered enough not to affect TUI responsiveness.
- [ ] No model-context messages are generated from this artifact unless a later troubleshooting task explicitly reads it.

## Verification Plan

- UI snapshot test: render a transcript with wrapped text and collapsed tool trace; `left_chat_render.txt` lines match the visible left-pane lines.
- Manual test: run one prompt in an 80x24 terminal and compare the file with the terminal's left chat pane.
- Regression test: right thinking content does not appear in `left_chat_render.txt`.
- Stress test: streaming output does not write a frame for every token unless debug mode explicitly requests it.

## Notes

This task directly supports the current debugging cycle. It lets future assessment distinguish between "Elma did the wrong thing" and "Elma did the right thing but the left chat transcript displayed it poorly or hid critical operational rows."
