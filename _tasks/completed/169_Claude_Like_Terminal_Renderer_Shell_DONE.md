# Task 169: Claude-Like Terminal Renderer Shell

## Status
Completed.

## Objective
Replace Elma's current five-frame terminal UI with a Claude Code-like retained terminal renderer: sparse transcript rows, prompt/footer region, transient picker panes, and no persistent Elma header/activity/context chrome.

## Completed Work
- [x] Integrated `ClaudeRenderer` as the primary interactive renderer in `TerminalUI`.
- [x] Implemented `UiEvent` boundary for typed UI updates.
- [x] Quarantined legacy renderer to `src/ui_render_legacy.rs` and marked as `@legacy-only`.
- [x] Verified that the production interactive path no longer uses the old five-frame chrome.
- [x] Added `Ctrl+O` transcript expansion toggle.
- [x] Updated `ClaudeRenderer` for multi-line input support.
- [x] Made `ui_modal.rs` theme-aware using canonical tokens.
- [x] Passed `cargo build`, unit tests, and `ui_parity_probe.sh --fixture startup`.

## Required Visual Direction
Remove or demote from the default interactive path:

- Header strip.
- Faint full-width Elma separator system.
- Persistent activity rail.
- Boxed composer/input rectangle.
- Token context progress bar.
- Decorative Elma-specific streaming icons or brand glyphs.
- Gruvbox-driven frame layout.

The default screen should feel like Claude Code: content-first rows, minimal persistent chrome, prompt at the bottom, footer/status/hints only when useful, and modal panes only for pickers/help/permissions.

## Stack Decision
Use Rust crates aggressively if they improve parity. Ratatui is now the recommended production path unless an implementation proves the custom renderer can meet the same resize, modal, footer, snapshot, and no-flicker requirements. Do not expose Ratatui's stock visual style. Crossterm remains acceptable as the event backend.

Recommended implementation:

- `ratatui` for frame composition and deterministic drawing.
- `crossterm` for raw mode/events/alternate screen.
- `unicode-width` and `unicode-segmentation` for cursor and wrapping correctness.
- `vt100` plus `insta` through Task 167 for snapshots.

## Architecture
Create a UI boundary that separates:

- State model: transcript, footer, input, pickers, task list, compact state, status line.
- Layout calculation: terminal size, row allocation, overflow, scroll, transcript mode.
- Rendering: styled spans to terminal backend.
- Event handling: key/mouse/focus/paste/resize events.

Suggested modules:

- `src/ui/claude_state.rs`
- `src/ui/claude_renderer.rs`
- `src/ui/claude_layout.rs`
- `src/ui/claude_input.rs`
- `src/ui/claude_theme.rs` or reuse Task 168 canonical theme.

Keep names consistent with the final module organization chosen by the implementer.

## Renderer Requirements
- Full redraw on meaningful state changes is acceptable, but rendering must not flicker during streaming.
- Transcript rows must preserve Claude spacing and prefix hierarchy.
- Input cursor must be rendered with inverse or terminal-native cursor behavior matching Claude as closely as possible.
- Prompt/footer must remain stable while assistant/tool streams update above it.
- Resize must rewrap content and keep the cursor visible.
- Scrolling must support normal transcript, expanded transcript, and modal focus modes.
- Alternate screen cleanup must be reliable on panic, Ctrl-C, Ctrl-D, and normal exit.

## Migration Requirements
- `TerminalUI` may be replaced or internally delegated to the new renderer.
- Existing noninteractive output should remain available for scripts or non-TTY mode.
- `app_chat_loop` must stop printing interactive messages directly once the renderer owns the screen.
- The renderer must expose a small event/update API for Tasks 170-176 instead of every caller mutating visual strings directly.
- A `src/claude_ui` module that is not used by `TerminalUI` does not count.
- The active interactive path must not call `src/ui_render.rs::render_screen` after this task is complete.
- Old `UIState`/`TranscriptItem` can remain only as a migration adapter or noninteractive compatibility layer; it must not own interactive rendering.
- The old five-frame `src/ui_render.rs` implementation must be deleted, renamed as legacy, or gated behind an explicit non-TTY/legacy flag.
- The renderer API must accept typed UI events rather than pre-rendered strings so streaming, tools, tasks, compacting, and input can update the screen in place.

## Required UI Event Boundary
Define a small typed event channel before wiring callers:

- `TurnStarted`
- `UserSubmitted`
- `ThinkingStarted`
- `ThinkingDelta`
- `ThinkingFinished`
- `AssistantContentDelta`
- `AssistantFinished`
- `ToolStarted`
- `ToolProgress`
- `ToolFinished`
- `PermissionRequested`
- `PermissionResolved`
- `TasksUpdated`
- `CompactBoundary`
- `StatusUpdated`
- `Notification`
- `InputChanged`
- `ModeChanged`
- `Resize`
- `ExitRequested`

The event channel may be synchronous or async, but the renderer must be the single place that turns these events into terminal state.

## Acceptance Criteria
- The first interactive screen no longer shows the old Elma header strip, activity rail, boxed input, or context bar.
- The prompt and transcript remain stable while text streams.
- A pseudo-terminal fixture can type a prompt, receive a response, resize the terminal, and exit without corrupting the terminal.
- There is one authoritative interactive renderer path.
- `rg -n "ui_render::render_screen|render_screen\\(" src/ui_terminal.rs src/app_chat_loop.rs` does not show the production interactive path using the old renderer.
- A real pseudo-terminal snapshot fails if `Elma WORKFLOW`, the old header strip, boxed composer borders, or the context progress bar appears.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test claude_renderer -- --nocapture
cargo test ui_parity_renderer -- --nocapture
./ui_parity_probe.sh --fixture startup
./ui_parity_probe.sh --fixture resize
./ui_parity_probe.sh --fixture prompt-entry
```

The final verification must include a real `cargo run` or `target/debug/elma` pseudo-terminal session that proves the old five-frame chrome is absent.
