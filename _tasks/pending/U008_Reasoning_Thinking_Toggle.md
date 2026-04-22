# Task U008: Reasoning (Thinking) Toggle

## Objective
Expose an explicit UI toggle for collapsing/expanding reasoning blocks in the chat history.

## Strategy
- Update `ui_chat.rs` or `ui_markdown.rs`.
- Detect reasoning blocks in message content.
- Implement an interactive state for folding/unfolding these blocks.
- Ensure the state persists during scroll/resize events.
