# Task 684: Remote Daemon Channel And Notification Integrations Low Priority

**Status:** pending
**Priority:** LOW
**Type:** Optional Integration
**Scope:** future daemon/channel modules, `src/session_store.rs`, `src/event_log.rs`
**Source:** postponed tasks 126-131, 106, 128

## Summary

Defer daemon mode, HTTP gateway, Telegram, always-on services, and terminal/system notifications until core local CLI/session architecture is stable.

## Evidence And Gap

- These features require network or background service behavior, which is lower priority than offline reliability.
- If implemented prematurely, they can bypass transcript-native visibility and permission policy.

## Implementation Plan

1. Require Task 679 headless API and Task 641 transcript visibility before any remote channel.
2. Add channel abstraction only after permissions, session state, and event logs can represent non-TTY actions.
3. Keep remote channels opt-in and disabled by default.
4. Persist channel-origin metadata in every message/tool event.

## Acceptance Criteria

- [ ] Remote channels cannot bypass local permission gates.
- [ ] Always-on mode has explicit lifecycle and shutdown records.
- [ ] Notifications are optional and do not become footer/status noise.
- [ ] Session resume distinguishes local TTY vs remote-origin messages.

## Verification Plan

Use local fake channel fixtures; no external service dependency.

