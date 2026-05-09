# Task 663: Session Store Typed Message Parts And Runtime State Ownership

**Status:** pending
**Priority:** HIGH
**Type:** Architecture / Persistence
**Scope:** `src/session_store.rs`, `src/session_state.rs`, `src/session_index.rs`, `src/session_paths.rs`, `src/event_log.rs`
**Source:** deferred tasks 469/489, old SQLite typed query task, `_knowledge_base` Crush/Goose session stores

## Summary

Unify session runtime state, typed message parts, event references, usage, stop reasons, and extension state into a durable structured store.

## Evidence And Gap

- Session data is split across `session.json`, markdown transcript, event log, SQLite store, summaries, runtime task files, and indexes.
- `session_paths.rs` comments note duplicate legacy artifacts.
- `_knowledge_base` Crush/Goose session models include typed messages and session services.

## Implementation Plan

1. Define typed message parts for user, assistant text, reasoning summary, tool call, tool result, notice, compact boundary, and finalization.
2. Add migrations for stop reason, token usage, model/provider, event refs, and extension state.
3. Decide ownership between `session.json`, SQLite, markdown transcript, and JSONL events.
4. Add consistency checks that detect drift between stores.

## Acceptance Criteria

- [ ] Resume can reconstruct a turn without relying on markdown parsing.
- [ ] Stop reasons and token usage survive restart.
- [ ] Extension/optional feature state is namespaced and versioned.
- [ ] Duplicate/legacy artifacts are documented or removed.

## Verification Plan

Run session migration/resume tests and inspect a real session after a multi-tool turn.

