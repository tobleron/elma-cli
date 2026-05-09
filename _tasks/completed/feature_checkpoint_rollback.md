# Implement Checkpoint and Rollback System

## Description
Periodically saves the state of the workspace (files) and the chat history. This allows users to "roll back" to a previous state, reverting file changes and restoring the conversation context to a specific point in time.

## Reference Implementation (Dirac)
- `_knowledge_base/dirac/proto/dirac/checkpoints.proto`
- `_knowledge_base/dirac/src/checkpoints/`
- `_knowledge_base/dirac/webview-ui/src/shared/ui/CheckmarkControl.tsx`

## Implementation Plan for elma-cli
1. Integrate with Git to create lightweight branches or snapshots for every major agent action.
2. Implement a `session-checkpoint` command to allow users to view and restore previous states.

## Status
PENDING
