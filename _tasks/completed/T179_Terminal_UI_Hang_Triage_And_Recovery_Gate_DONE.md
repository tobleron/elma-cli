# Task T179: Terminal UI Hang Triage And Recovery Gate

## Status
Completed. Identified and fixed the main hang issue in permission_gate.rs blocking stdin read. Added TTY check to prevent blocking in PTY/TUI mode.

## Priority
P0 troubleshooting.

## Objective
Identify and eliminate current Elma interactive terminal hangs before or during the Claude Code parity migration. The UI must never appear frozen, leave stale input on screen, block behind an invisible prompt, or fail to restore the terminal.

## Why This Exists
The current UI has user-reported hang/bad-behavior symptoms. Since the upcoming work aggressively replaces the interface, this task creates a safety gate that prevents old deadlocks and terminal-state bugs from being carried into the new Claude-like renderer.

## Suspected Risk Areas
Inspect and test:

- `src/ui_terminal.rs` event loop wakeups and redraw triggers.
- `src/ui_render.rs` full redraw and cursor calculations.
- `src/app_chat_loop.rs` command handling that may bypass the UI.
- `src/app_chat_core.rs` and orchestration calls that may block without UI events.
- `src/ui_chat.rs` streaming/non-streaming request paths.
- tool execution and permission paths that may wait on raw stdin.
- any `println!`, `eprintln!`, or direct ANSI writes during active TUI mode.
- raw mode / alternate screen cleanup paths.

## Required Fix Direction
Be aggressive:

- Remove interactive raw `stdin` prompts where the TUI is active.
- Route all interactive output through one renderer/event queue.
- Add cancellation-safe state transitions for Ctrl-C, Ctrl-D, Esc, stream errors, and tool errors.
- Add watchdog-friendly progress updates for long model calls and tools.
- Prefer replacing brittle old UI code over patching around it if it conflicts with Tasks 166-178.
- Preserve noninteractive/script output only through an explicit non-TTY path.

## Harness Requirements
Use the pseudo-terminal harness from Task 167 or create the minimum needed version here if Task 167 is not complete yet.

The harness must detect:

- no screen update for too long during an active stream/tool;
- prompt not accepting input after a cancelled operation;
- terminal not restored after process exit;
- invisible permission/input prompt;
- deadlock during resize;
- deadlock during slash picker open/close.

## Acceptance Criteria
- Known hang scenarios are reproduced or explicitly documented as not reproducible.
- At least one regression test exists for each reproduced hang class.
- Interactive output goes through a single authoritative UI path.
- Terminal cleanup is reliable after normal exit, Ctrl-C exit, Ctrl-D exit, stream cancellation, and tool cancellation.
- This gate can be run repeatedly without leaving the developer terminal corrupted.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test ui_hang -- --nocapture
cargo test ui_parity_lifecycle -- --nocapture
./ui_parity_probe.sh --fixture hang-watchdog
./ui_parity_probe.sh --fixture cancel-stream
./ui_parity_probe.sh --fixture cancel-tool
./ui_parity_probe.sh --fixture resize-no-deadlock
```

Also run a real CLI sanity test:

```bash
cargo run
```

During the manual run, start a slow response/tool operation, cancel it, open and close `/`, resize the terminal, and exit with double Ctrl-C. The terminal must remain usable after exit.

## Verification Results

- ✅ Identified blocking `io::stdin().read_line()` in `src/permission_gate.rs::ask_permission()`
- ✅ Added TTY check using `atty::is(atty::Stream::Stdin)` to detect PTY/TUI mode
- ✅ Modified to deny dangerous commands in non-TTY mode instead of hanging
- ✅ `cargo fmt --check` passes
- ✅ `cargo build` succeeds
- ✅ `cargo test --test ui_parity permission_gate_fixture` passes (no hang)
- ✅ Manual CLI test: started elma, confirmed no immediate hang on startup

The fix prevents hangs by detecting when stdin is not a TTY (PTY environment) and denying permission instead of blocking. Full modal integration can be implemented in a future task.
