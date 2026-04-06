# Task 127: Telegram Bot Integration

## Priority
**Postponed (Tier B — Future Capability)**
**Created:** 2026-04-05
**Status:** Postponed
**Depends on:** Task 126 (Daemon Mode HTTP Gateway)

## Overview

Elma accessible via Telegram DMs. User messages on Telegram route to the daemon, which processes them through the tool-calling pipeline and sends back the reply.

## Scope

### 1. Telegram Bot Setup
- Uses `teloxide` crate for Telegram Bot API
- Polling mode (no webhook needed for personal use)
- Bot token configured via `ELMA_TELEGRAM_TOKEN` env var or config file
- `/start` command initializes session, sends welcome message

### 2. Message Routing
- Each Telegram user → unique session (keyed by Telegram user ID)
- Text messages → daemon `/chat` endpoint
- Bot replies with Elma's response text
- Long responses split into Telegram message chunks (4096 char limit)

### 3. Conversation State
- Session persists across Telegram bot restarts
- User's session ID stored in SQLite, keyed by Telegram user ID
- `/reset` command clears user's session history
- `/status` shows current session info

### 4. Tool Approval Policy
- Telegram sessions: **auto-approve** all tool calls (no interactive prompts available)
- Safety enforced by pre-flight hooks and destructive command detection (Tasks 116-120)
- Dangerous commands require explicit `/confirm` follow-up message

### 5. Integration Points
- `src/telegram_bot.rs` (new) — teloxide bot setup + message handlers
- `src/app.rs` — `elma telegram` subcommand or integrated into daemon
- Calls daemon HTTP endpoint, not direct tool-calling pipeline

### 6. Design Constraints
- **Text-only** — no image/voice/PDF handling in this task
- **DM only** — no group chat support initially
- **Single bot** — one Elma instance, one Telegram bot token
- **No browser automation** — shell, read, search, respond tools only

## Estimated Effort
~400 lines of Rust. 1-2 days focused work.

## Verification
1. `cargo build` clean
2. Bot starts, responds to `/start` with welcome message
3. User sends "list files" → Elma replies with file listing
4. User sends "who are you?" → Elma replies correctly
5. Bot survives disconnect and reconnects automatically
6. Multiple Telegram users get independent sessions
