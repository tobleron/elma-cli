# Task 131: Telegram Session Persistence Across Restarts

## Priority
**Postponed (Tier B — Future Capability)**
**Created:** 2026-04-05
**Status:** Postponed
**Depends on:** Tasks 126 (Daemon Mode), 127 (Telegram Bot), 129 (Background Sessions)

## Overview

When the Elma daemon restarts (or crashes and recovers), Telegram users should find their conversations intact — same session, same history, same context. No "I lost our conversation" moments.

## Scope

### 1. Telegram User → Session ID Mapping
- SQLite table: `telegram_sessions (telegram_user_id TEXT PRIMARY KEY, elma_session_id TEXT)`
- On bot startup: load all mappings, restore sessions
- If session no longer exists (cleaned up): create new one, update mapping

### 2. Session Restoration
- On daemon restart: reload sessions from SQLite
- Elma's conversation history persists via existing session storage
- Tool call history, context, and state all survive restart

### 3. Stale Session Cleanup
- Sessions older than 30 days with no activity: archive
- `/forget` command on Telegram: delete user's session and mapping
- Graceful: warns user before deletion, confirms

### 4. Integration Points
- `src/telegram_bot.rs` — session mapping on startup
- `src/session.rs` — session restoration logic
- SQLite schema migration for `telegram_sessions` table

## Estimated Effort
~200 lines. Half-day focused work.

## Verification
1. `cargo build` clean
2. Send messages on Telegram, restart daemon → conversation continues seamlessly
3. `/forget` → session deleted, next message starts fresh
4. 30-day stale sessions archived automatically
