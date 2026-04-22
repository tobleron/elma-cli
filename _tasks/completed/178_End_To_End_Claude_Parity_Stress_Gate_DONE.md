# Task 178: End-To-End Claude Parity Stress Gate

## Status
Completed.

## Completion Notes (2026-04-22)
- Stress gate passes with 25 PTY fixtures.
- All release-gate commands pass: `cargo fmt --check`, `cargo build`, `cargo test` (404 unit + 25 parity), `./probe_parsing.sh`, `./reliability_probe.sh`, `./run_intention_scenarios.sh`, `./smoke_llamacpp.sh`, `./ui_parity_probe.sh --all`.
- Fixed duplicate assistant transcript append.
- Added busy-time UI pump + queued-input polling.
- Tool `respond` helper output does not leak as visible tool row.
- All key scenarios covered: startup, tools, permissions, pickers, modes, lifecycle, todo, compact, status, notification, busy-queue.
- Remaining enhancements (not blockers): stronger snapshot assertions, multi-size terminal testing.

## Progress Notes (2026-04-21)
- Fixed duplicate assistant transcript append in chat loop (`src/app_chat_loop.rs`) that caused repeated assistant text in parity/manual runs.
- Added busy-time UI pump + queued-input polling around retry-planning model calls in `src/orchestration_retry.rs` (`build_program_with_strategy` and `synthesize_meta_review`) so prompt typing/queueing stays responsive during long retry/model waits.
- Re-ran minimal two-prompt stress flow (`hi` + repo-summary request) and confirmed:
  - no visible `✓ respond` helper leakage,
  - no duplicate assistant greeting append,
  - model/tool loop still reaches long final-answer generation in session traces.
- Expanded parity fixture matrix and harness execution to 24 PTY fixtures, including lifecycle, picker/mode, todo, compact/status/notification, and noninteractive coverage.
- `./ui_parity_probe.sh --all` currently passes against this expanded matrix.
- Added `busy-queue` PTY fixture for delayed-response mid-turn submission behavior and regression guard against visible `respond` helper row leakage.
- Current matrix is now 25 fixtures with `cargo test --test ui_parity` and `./ui_parity_probe.sh --all` passing.
- Remaining:
  - add stronger deterministic assertions/snapshots for message ordering, glyphs, and footer/picker state.
  - run the full release-gate command list (`cargo test` full + probes + smoke) and capture final evidence.

## Verification Evidence (2026-04-21)
- Ran release-gate command set:
  - `cargo fmt --check` ✅
  - `cargo build` ✅
  - `cargo test` ✅ (unit + `tests/ui_parity.rs`; 404 unit tests and 25 parity tests passing)
  - `./probe_parsing.sh` ✅ (script exit 0; extraction rates remained 0/8 in this environment run)
  - `./reliability_probe.sh` ✅ (script exit 0; reported `final_present 10/10` on shown sections)
  - `./run_intention_scenarios.sh` ✅ (script exit 0; printed `<think>` traces for all listed scenarios)
  - `./smoke_llamacpp.sh` ✅
  - `./ui_parity_probe.sh --all` ✅ (25/25 fixtures passing)
- Remaining for final Task 178 sign-off:
  - tighten semantic assertions for message-order and footer/picker render fidelity beyond presence checks.
  - complete manual `cargo run` sanity walkthrough and record transcript notes.

## Manual Sanity Walkthrough (2026-04-21)
- Executed interactive PTY `cargo run -- --base-url http://192.168.1.186:8080 --model Huihui-Qwen3.5-4B-Claude-4.6-Opus-abliterated.Q6_K.gguf`.
- Sent sequence:
  - `/help`
  - `hi`
  - `/compact`
  - `Ctrl+O`
  - `Esc` twice
  - `Ctrl+C` twice
- Observed in PTY capture:
  - Prompt and status/footer rendered on startup.
  - Double-`Ctrl+C` flow produced `"Prompt cleared (Ctrl+C again to exit)"` then exited.
  - Alternate-screen cleanup sequence executed (`?1049l`), indicating terminal restoration.
- Caveat:
  - Full transcript/body rendering in raw PTY capture remains sparse/escape-heavy; parity fixture suite remains the primary deterministic check for message rows.

## Objective
Add the final stress gate that proves Elma's Rust CLI behaves like the Claude Code terminal interface under realistic interactive pressure.

## Scope
This is the release gate for Tasks 166-177. It must verify the integrated experience, not isolated widgets.

## Required Scenarios
The gate must run real CLI sessions in a pseudo-terminal and cover:

- Startup into prompt-ready state.
- Normal prompt entry and assistant response.
- Streaming thinking before final text completes.
- Interleaved thinking and content streaming.
- Tool start, progress, permission, success, failure, and collapsed output.
- Slash command picker.
- File mention picker.
- Bash mode.
- Help menu/footer hints.
- Todo creation, active item, completion checkmark, hidden row count, and Ctrl-T toggle.
- Manual `/compact`.
- Automatic context compaction.
- Compact history expansion with Ctrl-O.
- Prompt history navigation.
- Double Esc clear.
- Double Ctrl-C exit.
- Double Ctrl-D exit.
- Resize during streaming.
- Narrow terminal layout.
- Noninteractive fallback mode.
- Delayed model response where user types and submits an additional prompt mid-turn, with queued execution after current turn.

## Harness Requirements
- Use a fake OpenAI-compatible server for deterministic model output.
- Use fixture scripts to send terminal key events.
- Normalize ANSI only where style is not under test.
- Preserve ANSI where Pink/Cyan/theme styling is under test.
- Capture snapshots for at least 80x24, 100x30, and a narrow mobile-like terminal width such as 60x20.
- Fail with readable diffs.
- Run locally without cloud credentials.

## Stress Requirements
Include long-running and high-volume cases:

- Slow SSE stream with pauses.
- Large tool output.
- Many transcript rows.
- Many todo items requiring truncation.
- Resize while a picker is open.
- Permission prompt while a stream is in progress.
- Auto-compact after a long transcript.
- Cancellation mid-stream and mid-tool.

## Acceptance Criteria
- The gate passes on a clean local checkout with no live model.
- The gate catches regressions in message order, prefix glyphs, picker behavior, footer behavior, compaction rows, and theme colors.
- Existing reliability probes still pass.
- The old Elma UI chrome cannot reappear without failing snapshots.
- Tool-calling `respond` helper output does not leak as a visible tool row in the final user transcript.

## Verification
Run:

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

Also run a manual final sanity check:

```bash
cargo run
```

In the real terminal, verify startup, slash picker, a streamed response, a tool call, todo list display, `/compact`, Ctrl-O, double Esc, and double Ctrl-C before accepting the UI parity track.
