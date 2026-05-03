# Task 438: Provider Connectivity And Endpoint Hardening

**Status:** completed
**Priority:** CRITICAL
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 473, pending Task 448, completed Task 278, completed Task 303

## Summary

Harden LLM provider endpoint construction, connectivity checks, auth behavior, and provider-specific request paths.

## Implementation Completed

1. **Skip active connectivity checks at startup** — Removed the blocking connectivity check from bootstrap. Defer to first request (offline-first).
2. **Fixed endpoint path duplication** — Fixed `check_endpoint_connectivity` which was appending `/v1/chat/completions` to a URL that already contained it.
3. **Added /provider command** — Interactive dialog to configure endpoint IP/port and model. Uses `inquire` crate for prompting.
4. **Persistence** — Config saved to `_elma.config` in model config directory for next-session persistence.

## Files Changed

- `src/app_bootstrap_core.rs`: Removed startup connectivity check, fixed path bug
- `src/app_chat_loop.rs`: Added `/provider` command routing
- `src/app_chat_handlers.rs`: Added `handle_provider_config` with interactive dialog + persistence

## Verification

- `cargo check --all-targets` ✓
- `cargo clippy --all-targets` ✓
- 820 tests pass (1 pre-existing flaky session_paths test unrelated)
