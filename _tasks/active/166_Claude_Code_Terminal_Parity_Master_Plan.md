# Task 166: Claude Code Terminal Parity Master Plan

## Status
Pending.

## Progress Notes (2026-04-21)
- Continued integration from the 172-176 implementation slice in the production interactive path (`src/ui_terminal.rs`, `src/app_chat_loop.rs`, `src/tool_calling.rs`, `src/tool_loop.rs`, `src/claude_ui/*`).
- Extended pseudo-terminal parity harness coverage in `tests/ui_parity.rs`:
  - Added reusable `run_named_fixture(...)`.
  - Added raw key-step support with `send_enter` for control sequences (Esc/Ctrl-C/Ctrl-D/arrows/Ctrl-T).
  - Added fixture tests for `slash-picker`, `file-picker`, `bash-mode`, `double-escape-clear`, `double-ctrl-c-exit`, `todo-create`, `todo-progress-checkmark`, `todo-toggle`, `manual-compact`, `auto-compact`, `status-line`, `notification`, `clear`, `resume-picker`, `prompt-history`, `graceful-exit`, `noninteractive-output`.
- Added the corresponding fixture files under `tests/fixtures/ui_parity/`.
- Verification evidence for this slice:
  - `cargo fmt --check` passed.
  - `cargo build` passed.
  - `cargo test --test ui_parity` passed (24 tests).
  - `./ui_parity_probe.sh --all` passed.
- Remaining before master-plan closure:
  - Complete deeper behavioral assertions (not only non-empty output checks) for new fixtures.
  - Finish and sign off 173-178 acceptance criteria and task-document lifecycle cleanup.
  - Run full final verification list (including `cargo test` full suite and non-UI probes) and record evidence.

## Progress Notes (2026-04-21, follow-up)
- Addressed user-reported in-flight visibility and input-lock issues in active interactive path:
  - `TerminalUI::draw_claude` now surfaces current activity state in the Claude status line (`Analyzing/Planning/Executing/Responding`).
  - Added `TerminalUI::poll_busy_submission()` for non-blocking key handling while work is in progress.
  - Added queued-input notifications via `TerminalUI::notify(...)`.
  - Chat loop now keeps polling UI while awaiting long async stages and queues submitted prompts to run after the current turn.
- Verification evidence for this slice:
  - `cargo build` passed.
  - `cargo test --test ui_parity` passed (24 tests).

## Progress Notes (2026-04-21, continuation)
- Parity harness strengthened with fixture-level assertions (including negative pattern checks).
- Added and stabilized `busy-queue` regression coverage to guard against visible `respond` tool-row leakage (`not: true` assertion on `✓ respond`).
- `/resume` path now opens a real session modal and has fixture coverage.
- Verification evidence:
  - `cargo test --test ui_parity` passed (25 tests).
  - `./ui_parity_probe.sh --all` passed (25 tests).
- Deferred blocker (explicitly parked per user direction until end-of-track):
  - prompt remains partially non-editable in some in-flight thinking/tool states in real manual runs; final troubleshooting pass must close this before Task 166 sign-off.

## Progress Notes (2026-04-22)
- **T180 COMPLETED**: Fixed prompt non-editable during in-flight states + permission hang
  - Root cause 1: `poll_busy_submission()` missing key handlers that existed in `run_input_loop()`
  - Added missing handlers: Home/End, PageUp/PageDown, Ctrl+Enter, Up/Down scroll, Esc double-tap
  - Added PTY fixtures: `stress-input-during-streaming`, `input-during-tool-execution`
  - Root cause 2 (permission hang): `wait_for_permission()` was a BLOCKING loop on the async task thread
  - Fix: Replaced with async `request_permission()` using tokio oneshot channel
  - Input loops send permission responses through channel instead of competing for events
  - Added "Press y to approve, n to deny" hint to permission prompt
  - All 27 UI parity tests pass
- **179 COMPLETED**: Scroll behavior — sticky header + new messages pill
  - Sticky header: `❯ {truncated prompt}` when scrolled up
  - New messages pill: `─── N new messages ───` divider + floating pill
  - Added `divider_index` tracking to `ClaudeTranscript`
- **180 COMPLETED**: Message row indicators, spacing, and task list
  - User prefix: `>` → `❯`
  - User truncation: 10,000 char hard-cap
  - Assistant spacing: blank line before
  - Thinking: only last block shown
  - Task symbols: `○→◻`, `◐→◼`, `✓→✔`
  - Task header: `N tasks (M done, K in progress, P open)`
- **181 COMPLETED**: Transcript mode shortcuts + model picker wiring
  - Transcript mode: `q` quit, `g/G` top/bottom, `j/k` line scroll, `b/Space` page, `/` search
  - `Ctrl+O` enters dedicated transcript mode
- **182 COMPLETED**: Legacy modal removal and transcript unification
  - Migrated all modals from `draw_with_modal()` to Claude renderer
  - Removed legacy modal rendering entirely
  - All modals now render as Claude-style centered panes
- **183 COMPLETED**: Input and rendering enhancements
  - Shift+Enter for multiline input
- Verification evidence:
  - `cargo build` passed
  - `cargo test --test ui_parity` passed (27 tests)
  - `cargo test` full suite passed

## Progress Notes (2026-04-21, release-gate evidence pass)
- Executed full 178 verification command bundle from this workspace:
  - `cargo fmt --check`
  - `cargo build`
  - `cargo test`
  - `./probe_parsing.sh`
  - `./reliability_probe.sh`
  - `./run_intention_scenarios.sh`
  - `./smoke_llamacpp.sh`
  - `./ui_parity_probe.sh --all`
- All commands exited successfully in this run.
- Observed caveat to keep open:
  - scenario scripts still show `<think>`-heavy outputs; parity and reliability suites pass, but manual interaction quality gate remains required before final archive.
- Manual PTY `cargo run` sanity walkthrough executed with key sequence (`/help`, `hi`, `/compact`, `Ctrl+O`, double `Esc`, double `Ctrl+C`); exit and terminal restoration observed in capture.

## Priority
P0 - Product-defining terminal experience.

## Master Plan Completion Rule
This master plan must remain in `_tasks/active/` until the entire Claude Code terminal parity track is finished. Do not move this file to `_tasks/completed/` while any child task, verification gate, UI parity fixture, migration step, or user sign-off item remains incomplete.

The master plan can be archived only after:
- [x] Task 167 has a real pseudo-terminal golden harness, not only `--help` tests.
- [x] Task T179 has reproduced or ruled out known UI hang classes and added regression coverage.
- [x] Task 168 has removed old palette usage from the active interactive path.
- [x] Task 169 has made the production `TerminalUI` use the Claude-parity renderer.
- [x] Task 170 has implemented message rows, markdown, transcript mode, and compact boundary rendering.
- [x] Task 171 has live SSE thinking/content UI events while the request is still in flight.
- [x] Task 172 has Claude-style tool start/progress/result, permissions, and output collapse.
- [x] Task 173 has Claude-style prompt input, slash picker, file mentions, and command modes.
- [x] Task 174 has the Todo tool and Claude-style task list integrated into real CLI flow.
- [x] Task 175 has context compaction, status/footer, and notifications adapted to Claude-style UI.
- [x] Task 176 has startup, clear, resume, history, transcript, and exit behavior verified.
- [x] Task 177 has removed or quarantined legacy interactive UI paths and stale docs.
- [x] Task 178 end-to-end stress gate passes in pseudo-terminal fixtures.
- [x] Task T180 has fixed prompt non-editable during in-flight states.
- [x] Task 179 has implemented sticky header and new messages pill.
- [x] Task 180 has fixed message row indicators and task list.
- [x] Task 181 has implemented transcript mode shortcuts.
- [x] Task 182 has removed legacy modal renderer.
- [x] Task 183 has added input and rendering enhancements.
- [x] Task 190 has unified dual transcript sources of truth.
- [ ] Task 188: Recursive file picker workspace discovery (pending, low priority)
- [ ] Task 189: Deep markdown renderer (pending, medium priority)
- [ ] All absorbed/superseded UI tasks are either archived, explicitly superseded, or documented as intentionally deferred.
- [ ] Final verification commands pass and are recorded in this file.
- [ ] The user explicitly approves moving this master plan to `_tasks/completed/`.

## Continuation Checklist
- [ ] Re-read `_tasks/TASKS.md`, `AGENTS.md`, and this master plan before changing any parity subtask.
- [ ] Re-read the active parity subtask before implementing or reviewing work.
- [ ] Confirm whether the current work affects the real production interactive path or only a sidecar/scaffold.
- [ ] Reject scaffold-only completion claims unless the real CLI path and pseudo-terminal fixtures prove behavior.
- [ ] Keep this master plan active while any child task is active, pending, blocked, or awaiting verification.
- [ ] Update the child task and this master checklist before stopping mid-track.
- [ ] Run `cargo fmt --check` before final sign-off.
- [ ] Run `cargo build` before final sign-off.
- [ ] Run `cargo test` before final sign-off.
- [ ] Run `./ui_parity_probe.sh --all` after the parity harness exists.
- [ ] Run real CLI or pseudo-terminal validation for startup, prompt input, streaming thinking, tools, slash picker, tasks, compacting, Ctrl-O, double Esc, and exit.
- [ ] Record final verification evidence in this file.
- [ ] Move this master plan to `_tasks/completed/` only after every applicable checkbox above is checked and the user approves.

## Objective
Make Elma's interactive CLI look and behave like Claude Code's terminal interface as closely as practical in Rust. The target is source-observed behavioral and visual parity with `_stress_testing/_claude_code_src`, not a loose "inspired by" redesign.

This task supersedes the previous Elma UI direction where it conflicts with Claude Code parity. In particular, the old Gruvbox-only, five-frame Elma chrome is not authoritative for this initiative.

## Source Of Truth
Primary reference:
- `_stress_testing/_claude_code_src/components/App.tsx`
- `_stress_testing/_claude_code_src/components/Messages.tsx`
- `_stress_testing/_claude_code_src/components/Message.tsx`
- `_stress_testing/_claude_code_src/components/MessageRow.tsx`
- `_stress_testing/_claude_code_src/components/messages/AssistantTextMessage.tsx`
- `_stress_testing/_claude_code_src/components/messages/AssistantThinkingMessage.tsx`
- `_stress_testing/_claude_code_src/components/messages/AssistantToolUseMessage.tsx`
- `_stress_testing/_claude_code_src/components/ToolUseLoader.tsx`
- `_stress_testing/_claude_code_src/components/PromptInput/PromptInput.tsx`
- `_stress_testing/_claude_code_src/components/PromptInput/PromptInputFooter.tsx`
- `_stress_testing/_claude_code_src/components/PromptInput/PromptInputHelpMenu.tsx`
- `_stress_testing/_claude_code_src/components/design-system/FuzzyPicker.tsx`
- `_stress_testing/_claude_code_src/components/QuickOpenDialog.tsx`
- `_stress_testing/_claude_code_src/components/CompactSummary.tsx`
- `_stress_testing/_claude_code_src/components/messages/CompactBoundaryMessage.tsx`
- `_stress_testing/_claude_code_src/components/TaskListV2.tsx`
- `_stress_testing/_claude_code_src/commands.ts`
- `_stress_testing/_claude_code_src/query.ts`

Current Elma implementation to replace or adapt:
- `src/ui_terminal.rs`
- `src/ui_render.rs`
- `src/ui_state.rs`
- `src/ui_markdown.rs`
- `src/ui_autocomplete.rs`
- `src/ui_input.rs`
- `src/app_chat_loop.rs`
- `src/app_chat_core.rs`
- `src/ui_chat.rs`
- `src/auto_compact.rs`
- `src/tool_calling.rs`
- `src/streaming_tool_executor.rs`

## Theme Mandate
The initial parity theme is not Gruvbox. Use a high-contrast monochrome base with Pink as the primary accent and Cyan as the complementary accent:

- Background: black.
- Primary text: white.
- Secondary text, separators, inactive UI: grey scale.
- Primary accent: high-contrast Pink.
- Complementary accent: high-contrast Cyan.

The theme must be tokenized so future themes can swap the primary accent, for example Pink to Orange, without rewriting every renderer. See Task 168.

## Intentional Differences From Claude Code
Only these differences are allowed unless a later task records a deliberate exception:

- Rust implementation instead of TypeScript/React/Ink.
- Elma product identity, local model names, and OpenAI-compatible local endpoint copy may differ from Claude-specific cloud/account copy.
- Single local LLM mode only. Do not implement multi-agent delegation, cloud team features, Anthropic account flows, telemetry, Buddy, teleport, or voice features as part of parity.
- Small 3B local models are assumed. UX may show patient, explicit progress, but the visible order and style must still match Claude Code.

## Preserved Elma Philosophy
This UI parity track does not change Elma's core local-first, small-model-first philosophy. Context-management, compaction, and resource-conservation tasks may remain valid as long as their visible terminal behavior is adapted to the Claude-style interface.

Do not remove good context-management work simply because it is not in Claude Code. Claude Code parity governs how the user sees and controls the CLI; Elma's internal planning, context budgeting, summarization, and llama.cpp-friendly reliability work can remain if it improves constrained 3B local model performance.

## Aggressive UI Compliance Rule
The current interactive UI is allowed to be replaced wholesale. If an existing Elma UI path contributes to hangs, terminal corruption, ugly redraws, stale prompts, blocked input, or non-Claude visual behavior, prefer removing or quarantining it over preserving backwards-compatible UI chrome.

Known risky areas to treat as P0 during implementation:

- mixed direct `println!` output while a TUI owns the terminal;
- raw stdin prompts during interactive mode;
- blocking tool/permission prompts that bypass the renderer;
- redraw loops that do not wake on stream/tool/input events;
- alternate-screen/raw-mode cleanup failures;
- stale footer/input state after resize, Ctrl-C, Ctrl-D, or stream cancellation.

## Review Findings To Enforce
Follow-up implementation reviews found that some work had been completed as scaffolding without changing the real user-facing terminal path. These findings are now blockers for this master task:

- A `src/claude_ui/` module existing is not enough. The production `TerminalUI` must use the Claude-parity renderer for interactive mode.
- `src/ui_terminal.rs` calling `src/ui_render.rs::render_screen` is evidence that the old five-frame path is still active.
- `src/ui_render.rs` rendering header strip, activity rail, boxed input, autocomplete box, or context bar is a master-task failure unless it is strictly noninteractive/legacy-only code.
- Streaming HTTP support is not enough. SSE chunks must emit live UI events for thinking and content while the request is still in flight.
- Accumulating `full_thinking` or `full_content` and returning only after the stream ends does not satisfy Claude parity.
- Do not use phrase lists or keyword filters to decide what is "thinking." Only provider/model-delivered reasoning fields or explicit reasoning blocks may be represented as thinking.
- The UI parity harness must drive the real interactive CLI in a pseudo-terminal. Tests that only run `elma-cli --help` do not count as parity verification.
- `./ui_parity_probe.sh --fixture NAME` must actually run the named fixture.
- `cargo fmt --check` must pass before any parity task can be considered complete.
- Active UI code must not keep hard-coded Gruvbox constants after Task 168 is accepted.

## Corrective Implementation Priority
If the task track is partially implemented, resolve in this order:

1. Fix formatting and obvious integration hygiene.
2. Build a real pseudo-terminal parity harness that can catch old UI chrome.
3. Replace the production interactive renderer path with the Claude-parity renderer.
4. Add a typed UI event channel and route model/tool/session events through it.
5. Wire SSE thinking/content deltas into that channel.
6. Remove or quarantine legacy interactive UI modules.
7. Only then mark individual parity tasks complete.

## Current Elma Gaps
Observed gaps from the local source audit:

- Elma renders a branded five-frame screen with header strip, activity rail, boxed composer, context bar, and Gruvbox colors. Claude Code uses sparse message rows, a prompt/footer area, transient panes, and status/footer text.
- Elma's thinking UI is mostly post-hoc or collapsed behind an Elma-specific convention. Claude Code shows `∴ Thinking` rows and expands thinking in transcript or verbose modes.
- Elma has a simple slash autocomplete list. Claude Code uses a fuzzy picker with search, selection state, keyboard navigation, and preview/secondary actions.
- Elma input lacks Claude-style key semantics including double Esc clear, double Ctrl-C/Ctrl-D exit, Ctrl-O transcript expansion, Ctrl-T task list toggle, Shift/Meta Enter multiline, `!` bash mode, `@` quick open, and `&` background mode.
- Elma command handling is not organized around the Claude Code slash command UX.
- Elma tool rows use its own prefixes and truncation model. Claude Code tool rows have loader dots, tool-specific summaries, progress messages, permission states, and grouped/collapsed read/search sequences.
- Elma has no Claude-style task list display tied to a Todo tool.
- Elma `/compact` currently behaves like a lightweight note, not a Claude-style compact boundary and summary/history flow.
- Existing UI tasks disagree on Ratatui, Gruvbox, Tokyo Night, crossterm-only, and persistent status bars.

## Existing Task Consolidation
This master plan consolidates and supersedes these overlapping tasks:

- `_tasks/postponed/110_Claude_Code_Style_Terminal_UI_SUPERSEDED_BY_166.md` -> superseded by Tasks 166, 171.
- `_tasks/postponed/132_Claude_Code_Inspired_UI_SUPERSEDED_BY_166.md` -> superseded by Tasks 166, 168, 169, 170.
- `_tasks/postponed/133_Ratatui_TUI_Gruvbox_Persistent_Status_Bar_SUPERSEDED_BY_166.md` -> superseded by Tasks 166, 168, 169.
- `_tasks/pending/007_Add_UpdateTodoList_Tool.md` -> absorbed by Task 174.
- `_tasks/pending/013_Smart_Input_Prefixes_And_Command_Modes.md` -> absorbed by Task 173.
- `_tasks/pending/014_Chord_Keybindings_And_Keyboard_Shortcuts.md` -> absorbed by Task 173.
- `_tasks/pending/015_Chat_Undo_Buffer_And_Conversation_History.md` -> absorbed by Task 176.
- `_tasks/pending/100_Interactive_Task_Progress_Tree.md` -> replaced by the Claude Code task list and tool progress model in Tasks 172 and 174.
- `_tasks/pending/104_Intelligent_Clipboard_Detection.md` -> absorbed into the prompt/footer work in Task 173.
- `_tasks/pending/105_Integrated_Context_Aware_Hints.md` -> absorbed into the prompt footer/help menu work in Task 173.
- `_tasks/pending/106_System_Terminal_Notifications.md` -> absorbed into Task 175.
- `_tasks/postponed/109_Streaming_API_Support.md` -> absorbed into Task 171.

Do not implement these older UI tasks separately once Task 166 is accepted. Their useful requirements must be implemented through the parity task sequence below.

## Required Task Sequence
1. Task 167: Claude Code Source Audit And Golden Terminal Harness.
2. Task T179: Terminal UI Hang Triage And Recovery Gate.
3. Task 168: Pink Monochrome Theme Token System.
4. Task 169: Claude-Like Terminal Renderer Shell.
5. Task 170: Message Rows, Markdown, Transcript, And Compact Boundary Rendering.
6. [x] Task 171: Streaming Thinking, Assistant Text, And SSE Event Pipeline.
7. Task 172: Tool Use UX, Permissions, Progress, And Output Collapse.
8. Task 173: Prompt Input, Slash Picker, File Mentions, And Command Modes.
9. Task 174: Todo Tool And Claude-Style Task List.
10. Task 175: Context Compaction, Status Line, Notifications, And Footer State.
11. Task 176: Session Lifecycle, Resume, Clear, Exit, And History UX.
12. Task 177: Legacy UI Removal And Documentation Consolidation.
13. Task 178: End-To-End Claude Parity Stress Gate.

## Dependency Policy
Binary size is not a concern for this project track. Prefer battle-tested crates if they improve parity:

- `ratatui` for retained terminal frame rendering if it helps reproduce Ink-style layout without adding visible Ratatui chrome.
- `tui-textarea` or a purpose-built editor module for prompt editing.
- `nucleo-matcher` for fuzzy slash/file pickers.
- `portable-pty`, `rexpect`, `vt100`, `strip-ansi-escapes`, and `insta` for real terminal snapshot tests.
- `pulldown-cmark` plus existing `syntect` for markdown and code rendering if the current parser is insufficient.
- `arboard` for clipboard image/text awareness where Claude-style prompt affordances require it.
- `notify-rust` for optional terminal/system notifications.

Each dependency must be introduced in the task that first uses it and must be covered by CLI or snapshot verification.

## Acceptance Criteria
- Elma's default interactive CLI no longer exposes the old five-frame Elma chrome.
- The production `TerminalUI` uses the Claude-parity renderer, not an unused sidecar renderer.
- Streaming thinking/content is visible during the active request through UI events.
- The first usable screen, message rows, tool rows, thinking rows, prompt footer, slash picker, task list, compact boundary, and exit behavior visibly match Claude Code source behavior within the allowed identity/model differences.
- The Pink/Cyan monochrome theme is the only active default theme and is implemented through tokens.
- Legacy UI modules are either removed, moved to noninteractive fallback paths, or documented as deprecated compatibility paths.
- A real pseudo-terminal test harness proves the interaction sequence, not only isolated renderer functions.
- `cargo build`, targeted tests, and real CLI parity probes pass.

## Final Verification
Run after Tasks 167-178 are complete:

```bash
cargo fmt --check
cargo build
cargo test
./probe_parsing.sh
./reliability_probe.sh
./run_intention_scenarios.sh
./smoke_llamacpp.sh
./ui_parity_probe.sh --all
```

Also run a real interactive session through a pseudo-terminal fixture that covers:
- Startup.
- Normal prompt entry.
- Streaming thinking.
- Final assistant response.
- Tool execution with progress and result.
- Slash command picker.
- `@` file picker.
- Todo list display and checkmark transition.
- Manual `/compact`.
- Automatic compact boundary.
- Ctrl-O transcript expansion.
- Double Esc clear.
- Double Ctrl-C or Ctrl-D exit.
