# T180: Fix Prompt Non-Editable During In-Flight States

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Status
In Progress

## Progress Notes (2026-04-22)
- Identified root cause: `poll_busy_submission()` was missing key input handlers that exist in `run_input_loop()`
- Missing keys that made prompt feel "non-editable":
  - `Home` / `End` - cursor navigation
  - `PageUp` / `PageDown` - transcript scrolling
  - `Ctrl+Enter` - submit with autocomplete
  - `Up` / `Down` - transcript scroll when not in history mode
  - `Esc` - double-esc detection for clearing prompt
- Added comprehensive key handling to `poll_busy_submission()` to match `run_input_loop()`
- Added 2 PTY fixtures:
  - `stress-input-during-streaming.yaml` - verifies keystrokes during model streaming
  - `input-during-tool-execution.yaml` - verifies input during long-running operations
- All 27 UI parity tests pass
- Full `cargo test` passes
- Verification evidence:
  - `cargo build` passed
  - `cargo test --test ui_parity` passed (27 tests)
  - `cargo test` full suite passed

## Verification
- [x] PTY fixture: rapid keystrokes during streaming are all captured
- [x] PTY fixture: input works during long tool execution
- [x] PTY fixture: no dropped keystrokes under stress
- [x] `cargo test --test ui_parity` passes (27 tests)
- [x] All existing UI tests pass

## Additional Fix (2026-04-22)
- **Root cause for permission hang**: `wait_for_permission()` was a BLOCKING loop that ran on the async task thread, preventing ALL async work (model streaming, UI updates, input handling)
- **Fix**: Replaced blocking `wait_for_permission()` with async `request_permission()` using tokio oneshot channel:
  - `request_permission()` creates a channel, sets up the permission prompt, and awaits the response
  - Input loops (`run_input_loop` and `poll_busy_submission`) detect active permission requests and send responses through the channel
  - 30-second timeout prevents infinite hangs
  - Added clear "Press y to approve, n to deny" hint to permission prompt rendering
- **CRITICAL SECOND FIX**: `request_permission()` must pump the UI while waiting because `execute_program` is NOT wrapped in `await_with_busy_queue` like model streaming is. Added UI pump loop inside `request_permission()` that calls `draw()` and `poll_busy_submission()` every 50ms while waiting for the user's y/n response.
- **Files changed**: `ui_terminal.rs`, `permission_gate.rs`, `tool_calling.rs`, `execution_steps_shell.rs`, `claude_state.rs`
- **Verification**: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests), `cargo test` full suite passed

## Trackpad Scroll Fix (2026-04-22)
- **Issue**: Trackpad 2-finger pinch/scroll was interpreted as Up/Down arrow keys, causing prompt history navigation instead of transcript scrolling
- **Fix**: Enabled crossterm mouse capture (`EnableMouseCapture`) to receive actual scroll wheel events
- **Implementation**: Handle `MouseEventKind::ScrollDown` / `ScrollUp` in both `run_input_loop` and `poll_busy_submission`
- **Scroll amount**: 3 lines per wheel tick (typical terminal behavior)
- **Cleanup**: Disable mouse capture on exit to restore terminal state

## Priority
**P0** — Explicit blocker for Task 166 sign-off

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Fix the issue where the prompt becomes partially non-editable during in-flight thinking and tool execution states.

## Background
This is an **explicit blocker** noted in Task 166 master plan:
> "prompt remains partially non-editable in some in-flight thinking/tool states in real manual runs; final troubleshooting pass must close this before Task 166 sign-off"

## Symptoms to Reproduce
- During model streaming (thinking or content), the prompt input may not accept keystrokes
- During tool execution (especially long-running shell commands), the input area may freeze
- The cursor may not respond to arrow keys, backspace, or text input
- Intermittent — happens during async operations

## Root Cause Hypothesis

### Hypothesis 1: Event Loop Blocking
- `crossterm::event::read()` blocks while async work is pending
- `poll_busy_submission()` has different input handling than idle state

### Hypothesis 2: Terminal State Corruption
- Raw mode or cursor position corrupted during rapid redraws
- `println!` or `eprintln!` calls corrupt TUI state

### Hypothesis 3: Race Condition
- `draw_claude()` called from multiple threads without synchronization
- `ClaudeRenderer` state mutated concurrently

## Investigation Steps

### 1. Create PTY Fixture for Stress Input
- Send a prompt, then rapidly send keystrokes during fake server's delay
- Assert all keystrokes are captured in the output
- File: `tests/fixtures/ui_parity/stress-input-during-streaming.yaml`

### 2. Audit Event Loop in `ui_terminal.rs`
- Check if `crossterm::event::read()` is called while async work is pending
- Verify `poll_busy_submission()` handles all key events correctly
- Check if `draw_claude()` is called from async contexts without proper synchronization
- Focus on: `run_input_loop()`, `poll_busy_submission()`, `draw_claude()`

### 3. Audit for Blocking Output
- Grep for `println!`, `eprintln!` in TUI path
- Verify cursor is restored after every draw
- Check for any direct stdout/stderr writes that could corrupt terminal state

### 4. Add Diagnostics (Temporary)
- Debug mode logging for key events and draw calls
- Timing measurements for draw calls
- Thread/async context tracking

## Potential Fixes

| Hypothesis | Fix |
|------------|-----|
| Blocking read | Use `crossterm::event::poll()` with timeout instead of blocking `read()` |
| Terminal corruption | Add terminal state validation before each draw; atomic draw calls |
| Race condition | Protect `ClaudeRenderer` state with a mutex; queue state updates instead of direct mutation |

## Files to Audit
- `src/ui/ui_terminal.rs` — event loop, input handling (PRIMARY)
- `src/claude_ui/claude_render.rs` — renderer state, concurrent access
- `src/app_chat_loop.rs` — async interaction with UI

## Files to Create
- `tests/fixtures/ui_parity/stress-input-during-streaming.yaml` — PTY fixture

## Verification Checklist
- [ ] PTY fixture: rapid keystrokes during streaming are all captured
- [ ] PTY fixture: input works during long tool execution
- [ ] PTY fixture: no dropped keystrokes under stress
- [ ] Manual: 5-minute interactive session with no input lockups
- [ ] `cargo test --test ui_parity` passes
- [ ] All existing UI tests pass

## Related Tasks
- Task 166 (master plan)
- Task T179 (hang triage — related but focused on deadlock/hang)
- Task 171 (streaming — may cause rapid redraws)

## Success Criteria
The prompt must remain fully editable during ALL states:
- While model is thinking (streaming)
- While model is generating content
- While tools are executing (any duration)
- While permission prompts are displayed
- No intermittent lockups during normal operation

---
*Created: 2026-04-22*
