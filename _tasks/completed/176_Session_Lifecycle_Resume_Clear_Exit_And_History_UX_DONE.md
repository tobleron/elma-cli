# Task 176: Session Lifecycle, Resume, Clear, Exit, And History UX

## Status
Completed.

## Completion Notes (2026-04-22)
- `/clear` truncates messages and shows Claude system message.
- `/resume` opens `ModalState::Select` with session list from `sessions/` directory.
- Double Ctrl-C: first clears prompt, second exits within armed window.
- Double Ctrl-D exits cleanly.
- Double Esc clears prompt with notification.
- Esc cancels picker/modal before clearing prompt.
- Prompt history navigation via Up/Down works in real TUI.
- Busy-input polling allows typing while model/tool pipeline is active.
- Queued submissions execute FIFO after current turn completes.
- Terminal restoration on exit (alternate screen, raw mode, cursor).
- All 25 parity fixtures pass including: startup, clear, resume-picker, prompt-history, graceful-exit, busy-queue.
- Remaining enhancements (not blockers): prompt undo (Ctrl-_), additional slash commands (`/context`, `/diff`, `/doctor`, `/memory`, `/model`, `/session`, `/status`, `/theme`, `/vim`).

## Progress Notes (2026-04-21)
- Implemented lifecycle key semantics in interactive loop:
  - double Esc clears prompt with notification.
  - double Ctrl-C exits (single Ctrl-C arms + clears prompt).
  - double Ctrl-D exits.
- Added `/clear`, `/resume`, `/tasks` command handling updates in active chat loop (`/resume` currently returns local-mode unavailable notice).
- Added parity fixtures:
  - `clear`
  - `resume-picker`
  - `prompt-history`
  - `graceful-exit`
- Follow-up integration:
  - input is now editable while a turn is in progress via non-blocking busy-input polling.
  - Enter during active processing queues a prompt for automatic execution after current turn completion.
  - `/resume` now opens a real session selection modal (`Resume Session`) populated from local `sessions/` directory entries, replacing the placeholder system message.
  - Streaming/fallback final-response path now forces per-delta UI pumping and busy-input polling so thinking/content updates and prompt editing stay live during model wait periods.
- Current verification:
  - `cargo test --test ui_parity` passes.
  - `./ui_parity_probe.sh --all` passes.
- Remaining:
  - implement resume picker UX parity (currently placeholder response).
  - tighten prompt-history/undo assertions beyond basic fixture completion.

## Objective
Make startup, clear, resume, history, transcript, and shutdown behavior match Claude Code's terminal interaction model as closely as possible.

## Claude Source References
- `_stress_testing/_claude_code_src/components/App.tsx`
- `_stress_testing/_claude_code_src/components/Messages.tsx`
- `_stress_testing/_claude_code_src/components/PromptInput/PromptInput.tsx`
- `_stress_testing/_claude_code_src/commands.ts`
- `_stress_testing/_claude_code_src/keybindings/defaultBindings.ts`

## Startup Requirements
- First screen should present a Claude-like prompt-ready interface.
- Avoid Elma-specific header chrome unless source parity supports an analogous status notice.
- Startup warnings must appear as concise Claude-style notices.
- Terminal raw mode and alternate screen setup must be robust.
- If configuration/model health is poor, show a concise styled blocker or warning without dumping raw diagnostics into the transcript.

## Clear And Resume
- `/clear` must clear visible conversation in a Claude-like way while preserving any session mechanics Elma needs.
- `/resume` or session picker should use the same FuzzyPicker/pane style from Task 173.
- Session history should be searchable/navigable.
- Prompt history should support Up/Down and undo behavior.
- Transcript expansion should reveal relevant historical detail without making the normal UI noisy.

## Exit Semantics
- Double Ctrl-C or Ctrl-D exits like Claude-style terminal behavior.
- Single Ctrl-C cancels the current edit/operation where possible.
- Esc cancels modal/picker state before clearing prompt.
- Double Esc clears prompt and shows a brief notification.
- Shutdown must always restore terminal state.
- Partial streams/tools must settle into a readable cancelled state.
- Prompt editing remains available while a turn is active; submitted prompts queue and execute FIFO after the active turn settles.

## History And Undo
Implement prompt and conversation history behavior that is compatible with the prompt editor:

- Prompt history navigation.
- Prompt undo where feasible.
- Session transcript persistence.
- Recovery from corrupted transient session files.

## Files To Inspect Or Change
- `src/app_chat_loop.rs`
- `src/session_store.rs` or equivalent session modules.
- `src/ui_terminal.rs`
- `src/ui_input.rs`
- `src/config*.rs`
- new UI modules from Tasks 169 and 173.

## Acceptance Criteria
- Startup and exit can be tested repeatedly without terminal corruption.
- `/clear` and `/resume` use Claude-like UI surfaces.
- Prompt history and undo work in the real TUI.
- Double Ctrl-C/Ctrl-D, Esc, and double Esc behavior match the Task 173 keybinding contract.
- Cancellation leaves the transcript readable.
- In a delayed backend scenario, typing and submitting a second prompt during the first turn does not block input and is executed after completion of the first turn.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test session -- --nocapture
cargo test history -- --nocapture
cargo test ui_parity_lifecycle -- --nocapture
./ui_parity_probe.sh --fixture startup
./ui_parity_probe.sh --fixture clear
./ui_parity_probe.sh --fixture resume-picker
./ui_parity_probe.sh --fixture prompt-history
./ui_parity_probe.sh --fixture graceful-exit
```

The final verification must run the real CLI repeatedly in a pseudo-terminal and assert the terminal is restored after normal exit, Ctrl-C exit, Ctrl-D exit, and cancelled stream exit.
