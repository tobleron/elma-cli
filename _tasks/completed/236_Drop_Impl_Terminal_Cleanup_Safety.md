# 236: Drop Impl — Terminal Cleanup Safety

## Status
`completed`

## Priority
P0 — Critical terminal safety.

## Source
Code review finding C-1. `TerminalUI` has no `Drop` impl. A panic, early `?` propagation, or any error between TUI construction and the explicit `cleanup()` call leaves the terminal in raw mode with the alternate screen active, corrupting the user's shell session.

## Objective
Implement `Drop for TerminalUI` so the terminal is unconditionally restored on any exit path — panic or clean.

## Scope

### `src/ui/ui_terminal.rs`
- Add `impl Drop for TerminalUI` immediately after the `impl TerminalUI` block:

  ```rust
  impl Drop for TerminalUI {
      fn drop(&mut self) {
          // Best-effort cleanup — errors are not propagable from Drop.
          // The explicit cleanup() call in the chat loop handles the clean-exit case;
          // this Drop handles panics and propagated errors.
          let _ = self.cleanup();
      }
  }
  ```

- The existing `cleanup()` call at `app_chat_loop.rs:982` must remain — it provides the normal-exit flush + history save path. `Drop` is the safety net only.

### `src/app_chat_loop.rs`
- No changes needed — the existing explicit `tui.cleanup()?` is correct for the clean path. After this task, the `?` propagation case is also covered.

## Verification
- `cargo build` passes.
- `cargo test` passes.
- Manual test: introduce a deliberate `panic!()` inside the chat loop after TUI initialization; verify the terminal returns to normal (cooked) mode after the process exits.
- Optional: add a PTY-backed integration test that verifies raw mode is cleared after a forced panic.

## References
- `src/ui/ui_terminal.rs:41–65` (struct definition)
- `src/ui/ui_terminal.rs:1953–1973` (cleanup method)
- `src/app_chat_loop.rs:619, 982` (TUI construction and explicit cleanup)
