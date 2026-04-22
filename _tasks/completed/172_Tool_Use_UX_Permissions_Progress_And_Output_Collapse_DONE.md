# Task 172: Tool Use UX, Permissions, Progress, And Output Collapse

## Status
Completed.

## Completion Notes (2026-04-22)
- Removed all `MessageRole::Tool` / `MessageRole::ToolResult` legacy variants from `ui_terminal.rs`.
- Replaced all `t.add_message(MessageRole::Tool...)` calls in `tool_calling.rs` with `emit_tool_start()`, `emit_tool_result()` helpers that use `ClaudeMessage::ToolStart` / `ClaudeMessage::ToolResult`.
- Tool display now exclusively uses Claude-style rendering via `ClaudeMessage` enum.
- Permission UX uses `wait_for_permission()` with TUI-safe draw loop (no raw stdin blocking).
- Long outputs collapse with `(X more lines — ctrl+o to expand)` indicator.
- Batch grouping for read/search/tool sequences implemented in `claude_state.rs`.
- All 429 tests pass (404 unit + 25 parity).

## Progress Notes (2026-04-21)
- Added Claude-style `ToolProgress` rows in the active tool-calling path for `shell`, `read`, and `search` lifecycle steps.
- Fixed tool result labeling so `read`/`search` failures and results no longer render as `shell`.
- Updated Claude transcript rendering so long tool output collapses by default and expands in transcript mode (`Ctrl-O`), with an explicit hidden-lines indicator.
- Fixed the pseudo-terminal parity harness to run in isolated writable temp roots (`--config-root`, `--sessions-root`, temp `HOME`) and to avoid blocking reads after fixture steps.
- Upgraded the fake parity server to answer both `GET /v1/models` and `POST /v1/chat/completions` for deterministic fixture flow.
- Added missing tool-parity fixture coverage and tests: `shell-tool-success`, `shell-tool-failure`, `permission-prompt`, `collapsed-tool-output`.
- Updated `ui_parity_probe.sh` fixture filter mapping to handle hyphenated fixture names (`foo-bar` -> `foo_bar_fixture`).
- Verification evidence (this slice): `cargo build` passed, `cargo test --test ui_parity` passed (7 tests), `./ui_parity_probe.sh --all` passed.
- Remaining for completion: grouped collapse for read/search batches, fixture coverage for tool progress/collapse states, and full parity probe sign-off list.

## Objective
Replace Elma's tool display with Claude Code-style tool rows, progress rows, permission states, grouped output, and transcript expansion.

## Claude Source References
- `components/messages/AssistantToolUseMessage.tsx`
- `components/ToolUseLoader.tsx`
- `components/Messages.tsx`
- `components/MessageRow.tsx`
- tool-specific renderers referenced by `renderToolUseMessage`, `renderToolUseProgressMessage`, and `renderToolUseQueuedMessage`.

## Required Visual Behavior
Implement source-observed behavior for:

- In-progress tool row with loader/dot.
- Tool name rendered as a strong visible label.
- Tool input or command detail shown in dim parentheses where appropriate.
- Progress messages while tools run.
- Waiting-for-permission/classifier states.
- Success and failure resolution rows.
- Tool output under the tool row with Claude-like indentation and collapse behavior.
- Grouped/collapsed read/search/tool sequences when Claude source would collapse them.
- Ctrl-O/transcript expansion to reveal hidden or collapsed detail.

Use Pink/Cyan theme tokens:

- `secondary` Cyan for tool names/progress where it improves separation from assistant text.
- `primary` Pink for permission-required or attention-needed states.
- `fg`/`muted`/`dim` for normal output and metadata.

## Permissions UX
Implement a Claude-style permission prompt flow:

- Permission request appears inline near the prompt/footer or as a small picker pane.
- Keyboard navigation matches the picker/keybinding system from Task 173.
- The terminal must never block with a raw `stdin` prompt while the TUI is active.
- Permission decisions must be represented in transcript history.

## Output Collapse Policy
Replace hard-coded Elma truncation with Claude-like collapse:

- Short output can render inline.
- Long output collapses with a clear count and can be expanded in transcript mode.
- Search/read batches can be grouped.
- Full output remains available in session logs or transcript expansion.

## Files To Inspect Or Change
- `src/tool_calling.rs`
- `src/streaming_tool_executor.rs`
- `src/tool_loop.rs`
- `src/tool_calling_execution.rs`
- `src/app_chat_loop.rs`
- `src/ui_state.rs`
- new UI event/render modules from Task 169.

## Acceptance Criteria
- Shell/read/search/edit tools show Claude-style start, progress, and result rows.
- Permission-required tools do not break the TUI.
- Long outputs collapse without losing access to full data.
- Tool rows are stable during streaming and resize.
- The old Elma prefixes for tool start/result are removed from the interactive UI.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test tool -- --nocapture
cargo test ui_parity_tools -- --nocapture
./ui_parity_probe.sh --fixture shell-tool-success
./ui_parity_probe.sh --fixture shell-tool-failure
./ui_parity_probe.sh --fixture permission-prompt
./ui_parity_probe.sh --fixture collapsed-tool-output
```

The final verification must execute real CLI tool calls inside a pseudo-terminal and compare visible rows against the Claude parity snapshots.
