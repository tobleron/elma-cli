# Task 638: Tool Thinking Notice State Machines For UI

**Status:** pending
**Priority:** HIGH
**Type:** Architecture / UI Internals
**Scope:** `src/claude_ui/claude_state.rs`, `src/claude_ui/claude_stream.rs`, `src/claude_ui/claude_render.rs`, `src/tool_loop.rs`
**Source:** old UI tasks, `_knowledge_base` Codex trace and Crush event models

## Summary

Replace ad hoc tool/thinking/notice row mutation with explicit state machines that are replayable from runtime events.

## Evidence And Gap

- `ClaudeMessage` mixes persistent transcript rows, ephemeral prompt hints, tool traces, thinking blocks, notices, and compact summaries in one enum.
- `tool_loop.rs` emits model/tool/finalization events, but UI state is updated through multiple direct calls and partial assumptions.
- AGENTS.md requires transcript-native operational visibility for hidden processes.

## Implementation Plan

1. Define lifecycle states for tool traces: proposed, running, succeeded, failed, denied, timed out, cancelled.
2. Define thinking states: streaming, summarized, collapsed, expanded, redacted.
3. Define notice states: transcript-persistent, collapsible, ephemeral, superseded.
4. Persist enough metadata to reconstruct the UI state from session events.
5. Render stop reasons, compaction, route/complexity, and budget as collapsible rows instead of footer/status-only data.

## Acceptance Criteria

- [ ] Every tool row has a stable lifecycle transition and terminal state.
- [ ] Thinking summaries do not overwrite raw evidence artifacts.
- [ ] Notices include type, created time, collapse state, and persistence policy.
- [ ] A replay test can rebuild UI rows from a synthetic event sequence.

## Verification Plan

Use reducer tests plus a manual prompt that triggers a tool call, compaction/budget notice, and final stop reason.

