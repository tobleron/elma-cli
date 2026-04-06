# Task 126: Daemon Mode — HTTP Gateway

## Priority
**Postponed (Tier B — Future Capability)**
**Created:** 2026-04-05
**Status:** Postponed
**Blocks:** Tasks 127 (Telegram), 128 (Channel Abstraction)

## Overview

Run Elma as a background process that listens on a local HTTP port and processes messages from any caller (scripts, cron, other apps, future Telegram bot). This is the foundation for "always-on" Elma.

## Scope

### 1. `elma daemon` Command
- Starts axum HTTP server on configurable port (default `127.0.0.1:8765`)
- Accepts POST `/chat` with `{ "message": "...", "session_id": "optional" }`
- Returns streaming or blocking response with Elma's reply
- Daemon logs to file, not terminal
- PID file for management

### 2. Session Routing
- Each `session_id` maps to an Elma session (creates if new)
- No session_id → auto-create
- Concurrent sessions supported (different callers, different contexts)
- Session state persists across daemon restarts (SQLite-backed)

### 3. Management Endpoints
- `GET /health` — daemon status, uptime, active sessions
- `GET /sessions` — list active sessions
- `DELETE /sessions/{id}` — terminate session
- `POST /reset` — reset all sessions

### 4. Integration Points
- `src/daemon_server.rs` (new) — axum HTTP server + routes
- `src/daemon_session.rs` (new) — session management for daemon context
- `src/app.rs` — add `daemon` subcommand
- Reuses existing tool-calling pipeline (`run_tool_calling_pipeline`)

### 5. Design Constraints
- **No browser automation** — daemon is text-only (shell, read, search, respond tools)
- **Local-only by default** — binds to `127.0.0.1`, configurable but warned
- **No auth by default** — localhost is trusted; optional Bearer token for remote access
- **Single binary** — no external services, no Docker required

## Estimated Effort
~500 lines of Rust. 1-3 days focused work.

## Verification
1. `cargo build` clean
2. `cargo test` — session routing, concurrent requests, health endpoint
3. Real test: `curl -X POST http://127.0.0.1:8765/chat -d '{"message": "list files"}'` → returns Elma's reply
4. Real test: two concurrent curl calls with different session_ids → both process independently
5. Daemon survives session crash and restarts cleanly
