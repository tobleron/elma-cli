# Task 802: Duplicate Artifact Reading Prevention

## Status
- **Priority:** Medium
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Prevent the model from re-reading the same large artifact files multiple times within a turn. In the session, the agent read the 50KB `E3MZQbsx...` artifact 3 times and the 50KB `FX8wSS6N...` artifact twice, adding 150KB+ of redundant context.

## Root Cause
The model doesn't track what it has already read. Large artifact outputs are persisted to disk, and the model re-reads them as if they contain new information each time.

## Requirements
- Maintain a set of "read artifact paths" per turn.
- Before executing a `read` tool call on a path that matches an artifact pattern, check if it was already read this turn.
- If already read, inject a context note: "You already read this file this turn (path: X, read at iteration Y). The content hasn't changed."
- Track artifact read count and warn if the same file is read 3+ times: "You've read X 3 times. Move on."

## Failure Mode Fixed
- Redundant large reads bloating context window
- Wasted tool calls on already-seen artifacts
