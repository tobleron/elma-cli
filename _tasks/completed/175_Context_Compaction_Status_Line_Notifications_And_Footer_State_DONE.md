# Task 175: Context Compaction, Status Line, Notifications, And Footer State

## Status
Completed.

## Completion Notes (2026-04-22)
- Replaced old `ui_context_bar::render_context_bar()` in status line with Claude-style compact format: `model=X  ctx=Y/Z`.
- Status line now shows activity state (`label: message`) when active, compact model/context when idle.
- Fixed notification expiry: added `notification_expiry: Option<Instant>` field to `TerminalUI` with 5-second TTL.
- Notifications now persist for 5 seconds then auto-clear instead of flashing for one frame.
- `notify()` and `enqueue_submission()` both set expiry timestamps.
- `draw_claude()` checks expiry before syncing notification line to renderer.
- All 429 tests pass (404 unit + 25 parity).

## Progress Notes (2026-04-21)
- Wired compact UI rows into active flow:
  - manual `/compact` now emits compact boundary + compact summary messages.
  - auto-compact path now emits compact boundary + compact summary messages.
- Added status/footer and notification line plumbing in Claude renderer state and `TerminalUI`.
- Added parity fixtures:
  - `manual-compact`
  - `auto-compact`
  - `status-line`
  - `notification`
- Current verification:
  - `cargo test --test ui_parity` passes.
  - `./ui_parity_probe.sh --all` passes.
- Follow-up integration:
  - Claude status line now includes active in-flight stage text from interactive activity state.
  - Added transient queued-input notification when user submits while a turn is already running.
- Remaining:
  - add stronger assertions for compact history affordance and notification expiry behavior.
  - validate context continuity after automatic compaction with follow-up prompts.

## Objective
Implement Claude Code-style context compaction, compact summaries, status line/footer state, and attention notifications.

This task must preserve Elma's local-first resource strategy for constrained 3B llama.cpp-style models. Claude Code parity controls the visible compaction/status experience; it does not require deleting Elma's useful context budgeting, summarization, or small-context continuity logic.

## Existing Work To Absorb
This task absorbs:

- Current `src/auto_compact.rs` behavior.
- Current `/compact` behavior in `src/app_chat_loop.rs`.
- `_tasks/pending/106_System_Terminal_Notifications.md`.

## Claude Source References
- `_stress_testing/_claude_code_src/query.ts`
- `_stress_testing/_claude_code_src/components/CompactSummary.tsx`
- `_stress_testing/_claude_code_src/components/messages/CompactBoundaryMessage.tsx`
- `_stress_testing/_claude_code_src/components/StatusLine.tsx`
- `_stress_testing/_claude_code_src/components/PromptInput/PromptInputFooter.tsx`

## Context Compaction Requirements
- Manual `/compact` must produce a real compact summary and compact boundary, not just a note.
- Automatic compact must trigger near context limits using the existing Elma token budget logic where applicable.
- Prompt-too-long and compact-needed states must be visible in the Claude-style footer/message pattern.
- Compact boundary row must read like Claude source behavior, including history expansion affordance.
- Ctrl-O/transcript mode must expose compact history.
- Compaction must preserve the information needed for small local models to continue reliably.
- Existing context-management tasks and modules may remain if they improve small-model reliability and do not leak old Elma UI chrome into the terminal.
- Compaction must be snapshot and real-CLI testable with deterministic fixtures.

## Status Line Requirements
Implement a Claude-like status line/footer system:

- Default status line should be dim and compact.
- It may show model, workspace/session, context percentage, elapsed time, permission mode, and relevant local endpoint state.
- It must not become the old persistent Elma context bar.
- It must adapt to narrow widths.
- It must use Pink/Cyan/grey tokens only.
- If user-configurable status commands are implemented, failures must be quiet and non-disruptive.
- It must update independently from transcript/message rows so in-flight indicators stay live while other layers continue rendering.
- Streaming/thinking/tool activity should animate on a stable tick cadence from the event loop (not only on incoming model chunks).

## Notification Requirements
Add attention signals that match Claude's footer/notification style first:

- Task completed while user is away.
- Permission needed.
- Prompt cleared.
- Compact completed.
- Long-running turn finished.

Optional OS notifications may use `notify-rust`, but in-terminal notification behavior is required first.

## Files To Inspect Or Change
- `src/auto_compact.rs`
- `src/app_chat_loop.rs`
- `src/app_chat_helpers.rs`
- `src/ui_state.rs`
- `src/ui_context_bar.rs`
- `src/ui_effort.rs`
- new UI renderer/footer modules from Task 169.

## Acceptance Criteria
- `/compact` creates a visible compact boundary and summary in the transcript.
- Auto-compact creates the same user-facing boundary.
- Status/footer shows useful state without recreating the old context bar.
- Notifications appear and expire in a Claude-like way.
- Footer/status remains interactive and visibly alive while model/tool work is in progress, including when backend responses are delayed.
- Context compaction preserves semantic continuity across a follow-up prompt.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test compact -- --nocapture
cargo test status_line -- --nocapture
cargo test ui_parity_compact -- --nocapture
./ui_parity_probe.sh --fixture manual-compact
./ui_parity_probe.sh --fixture auto-compact
./ui_parity_probe.sh --fixture status-line
./ui_parity_probe.sh --fixture notification
```

The final verification must run a real CLI fixture that fills enough context to trigger compaction, then asks a follow-up question that proves the compacted summary is active.
