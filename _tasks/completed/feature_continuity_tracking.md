# Implement Continuity Tracking (Alignment Scoring)

## Description
A mechanism that tracks whether the agent's execution remains aligned with the user's goal by verifying state at various "checkpoints" during a task. It calculates an alignment score.

## Reference Implementation (Dirac)
- `_knowledge_base/dirac/src/continuity.rs`
- `_knowledge_base/dirac/src/app_chat_loop.rs`

## Implementation Plan for elma-cli
1. Implement a background monitoring loop that evaluates the agent's tool outputs against the original task description and provides a "confidence" or "alignment" metric in the terminal output.

## Status
PENDING
