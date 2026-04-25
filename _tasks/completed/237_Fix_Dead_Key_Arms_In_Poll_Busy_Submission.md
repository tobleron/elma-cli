# 237: Fix Dead Key Arms In `poll_busy_submission`

## Status
`completed`

## Priority
P0 — Scroll and picker navigation silently broken during busy state.

## Source
Code review finding C-2. `poll_busy_submission` in `ui_terminal.rs` has four match arms (`PageUp`, `PageDown`, `Up`, `Down`) duplicated verbatim. In Rust, the second set of arms is unreachable dead code — they never execute. As a result, scrolling and picker navigation are silently non-functional while Elma is processing a request.

## Objective
Remove the duplicate dead match arms. Verify scroll and picker navigation work correctly during the busy state.

## Scope

### `src/ui/ui_terminal.rs`
- Identify the duplicate block: lines ~1890–1918 (the second set of `PageUp`, `PageDown`, `Up`, `Down` arms inside `poll_busy_submission`).
- Delete those four duplicate arms entirely.
- The first occurrence of each arm (lines ~1860–1888) must remain intact and unchanged.

### Lint guard
- Add to `Cargo.toml` or a `#![deny(...)]` attribute:
  ```toml
  [lints.rust]
  unreachable_patterns = "warn"
  ```
  Or add `#![warn(unreachable_patterns)]` to `src/main.rs` to catch future regressions.

## Verification
- `cargo build` passes.
- `cargo clippy -- -W unreachable-patterns` reports zero unreachable-pattern warnings in `ui_terminal.rs`.
- Manual test: run Elma, send a long-running prompt, and verify that Up/Down/PageUp/PageDown scroll the transcript while the response is streaming.
- Manual test: type `/` to open the slash picker while busy — verify Up/Down navigates the picker.

## References
- `src/ui/ui_terminal.rs:1860–1918` (the duplicate region)
