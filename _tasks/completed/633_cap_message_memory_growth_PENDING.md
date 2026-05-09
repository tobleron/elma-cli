# 633 Cap Message Memory Growth

## Summary
`ClaudeTranscript.messages` Vec grows unbounded. In long sessions it can reach thousands of entries. Every `render_ratatui` call iterates the entire vector, and helper methods (`last_user_message`, `count_unseen`) do linear scans.

## Affected Files
- `src/claude_ui/claude_state.rs:913` — `messages: Vec<ClaudeMessage>` grows without bound
- `src/claude_ui/claude_state.rs:1128` — `last_user_message` scans entire vec from end (clones full content)
- `src/claude_ui/claude_state.rs:1139` — `count_unseen_assistant_turns` linear scan from divider
- `src/claude_ui/claude_state.rs:1155` — `render_ratatui` iterates all messages every frame

## Current Behavior
- Every user/assistant/tool/thinking message pushed to unbounded Vec
- `render_ratatui` iterates all messages (O(n)) every frame
- `last_user_message()` reverse linear scan + clones full content (could be 10K+ chars)
- `count_unseen_assistant_turns()` linear scan from divider to end
- Memory grows linearly with session length; no cleanup mechanism

## Proposed Fix
- Implement ring buffer or windowed view for visible transcript (keep last N messages in memory)
- Offload old entries to session storage (compact summary or full archive)
- Cache `last_user_message` as a truncated preview string, update on push
- Cache `unseen_assistant_turns` counter, increment on push while scrolled up, reset on scroll-to-bottom
- Maintain a `divider_index` for the compaction boundary rather than keeping everything

## Estimated CPU Savings
Prevents O(n) growth in per-frame rendering + memory; ~15% render CPU reduction over long sessions

## Status
PENDING
