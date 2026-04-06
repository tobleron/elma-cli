# Task 128: Channel Abstraction Trait

## Priority
**Postponed (Tier B — Future Capability)**
**Created:** 2026-04-05
**Status:** Postponed
**Depends on:** Tasks 126 (Daemon Mode), 127 (Telegram Bot)

## Overview

A generic `Channel` trait that decouples Elma's agent brain from the messaging layer. Adding Discord, Slack, or any future channel becomes implementing a trait — not modifying core Elma logic.

## Scope

### 1. Channel Trait
```rust
pub(crate) trait Channel {
    fn name(&self) -> &str;
    /// Start polling/listening, route messages to the daemon
    async fn run(self, daemon_url: &str) -> Result<()>;
    /// Extract user ID from channel-specific message
    fn user_id(message: &Self::Message) -> String;
    /// Send reply back through the channel
    async fn reply(&self, user_id: &str, text: &str) -> Result<()>;
}
```

### 2. Telegram Implementation
- `TelegramChannel` implements `Channel`
- Wraps teloxide bot setup from Task 127
- `user_id` = Telegram user ID
- `reply` = `bot.send_message(chat_id, text)`

### 3. Channel Registry
- Multiple channels can run simultaneously
- Each channel gets its own session namespace
- Daemon routes messages regardless of source channel
- `elma run --channels telegram,cli` — start with specific channels

### 4. Integration Points
- `src/channel.rs` (new) — trait definition + registry
- `src/telegram_bot.rs` — refactor to implement `Channel`
- `src/app.rs` — channel selection CLI flags

## Estimated Effort
~300 lines. 1 day focused work.

## Verification
1. `cargo build` clean
2. Telegram channel starts and routes messages
3. Adding a stub Discord channel compiles without modifying Elma core
4. Channel registry correctly isolates sessions per channel
