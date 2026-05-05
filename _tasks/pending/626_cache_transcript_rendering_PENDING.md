# 626 Cache Transcript Rendering Output

## Summary
Biggest single CPU win. `ClaudeTranscript::render_ratatui` re-parses markdown and re-wraps every message on every frame. For 200 messages at 25fps during streaming, that's full pulldown-cmark + syntect highlighting per message per frame.

## Affected Files
- `src/claude_ui/claude_state.rs:1155` — `render_ratatui` iterates all messages, calls `to_ratatui_lines` per message
- `src/claude_ui/claude_state.rs:192` — `to_ratatui_lines` for Assistant calls `render_assistant_content`
- `src/claude_ui/claude_markdown.rs:832` — `render_markdown_ratatui_with_width` full pulldown-cmark parse + syntect per call
- `src/claude_ui/claude_render.rs:1173` — transcript wraps entire output per frame, clones `last_line_mapping`
- `src/claude_ui/claude_render.rs:1619` — `wrap_lines_with_mapping` char-by-char wrapping per frame

## Current Behavior
Every `render_ratatui` call:
1. Iterates all `self.messages` (unbounded growth)
2. Calls `to_ratatui_lines` per message → markdown parse → span building → word wrapping
3. Re-wraps all lines through `wrap_lines_with_mapping` (O(chars) per frame)
4. Clones `Vec<usize>` line mapping per frame

## Proposed Fix
Cache rendered output per message in `ClaudeTranscript`:
- Add `cached_lines: Vec<Vec<Line<'static>>>` — one entry per message
- Add `cached_mapping: Vec<usize>` — message index per line
- Add `dirty: bool` field, set on message push / expand toggle / width change
- In `render_ratatui`: if not dirty and width unchanged, return cached clone
- `AssistantContent` could store pre-rendered `Vec<RenderBlock>` to skip pulldown-cmark re-parse

## Estimated CPU Savings
~50% of per-frame render CPU

## Status
PENDING
