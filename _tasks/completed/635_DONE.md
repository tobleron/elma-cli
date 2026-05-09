# Task 635: UI Runtime Event Reducer And Service Boundaries

**Status:** pending
**Priority:** CRITICAL
**Type:** Architecture / UI Internals
**Scope:** `src/claude_ui/`, `src/ui/`, `src/ui_terminal.rs`, `src/pubsub.rs`
**Source:** pending/deferred/postponed task audit; `_knowledge_base` Crush/OpenHands/Codex event architecture scan

## Summary

Split the terminal UI into a typed event reducer, an input service, a transcript service, a tool/notice service, and thin renderers. This is an internal architecture task, not a visual refresh.

## Evidence And Gap

- `src/claude_ui/claude_render.rs` owns transcript mutation, markdown/session writes, modal state, thinking panel state, token counters, layout, hit testing, and rendering.
- `src/ui/ui_terminal.rs` owns terminal I/O, event reading, task state, permission channels, queued submissions, token counts, and renderer calls.
- `_knowledge_base/_source_code_agents/crush/internal/pubsub/broker.go` and Crush session UI notes show cleaner event/service separation.
- `_knowledge_base/_source_code_agents/OpenHands` architecture notes emphasize an `EventStream` and controller separation.

## Implementation Plan

1. Add a canonical `UiRuntimeEvent` enum for user input, model deltas, tool lifecycle, notices, permissions, resize, and session lifecycle.
2. Add a pure reducer module that maps events into a `UiViewState` without terminal I/O.
3. Keep crossterm/raw-mode handling in `TerminalUI`; it should emit events and render view state, not own domain state.
4. Move transcript persistence out of `ClaudeRenderer::push_message` into a UI/session adapter.
5. Add reducer snapshot tests for common turn flows and regression cases.

## Acceptance Criteria

- [ ] `claude_render.rs` no longer mutates session files directly.
- [ ] Input, transcript, tool trace, notice, and thinking state have distinct owners.
- [ ] Renderer methods are mostly pure view functions over `UiViewState`.
- [ ] UI event reducer tests cover user message, streaming assistant, tool start/result, stop notice, permission, and resize.

## Verification Plan

Run `cargo test claude_ui ui_terminal ui_runtime_event -- --nocapture`, `cargo check --all-targets`, and a manual interactive prompt that streams text plus tool calls.

