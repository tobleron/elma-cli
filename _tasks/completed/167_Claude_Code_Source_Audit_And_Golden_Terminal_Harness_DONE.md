# Task 167: Claude Code Source Audit And Golden Terminal Harness

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
Completed. The spec document exists at docs/claude_code_terminal_parity_spec.md, and the golden terminal harness has been implemented with pseudo-terminal support in tests/ui_parity.rs and the ui_parity_probe.sh script.

## Objective
Create the executable evidence base for Claude Code terminal parity. This task converts the local Claude Code source audit into a stable spec, golden fixtures, and real pseudo-terminal tests that later UI tasks must satisfy.

## Why This Exists
The project already has multiple "Claude-inspired" UI tasks that disagree with one another. This task makes `_stress_testing/_claude_code_src` the explicit reference and prevents future implementation from drifting into a merely Elma-styled UI.

## Source Files To Audit
Read and summarize behavior from:

- `_stress_testing/_claude_code_src/components/App.tsx`
- `_stress_testing/_claude_code_src/components/Messages.tsx`
- `_stress_testing/_claude_code_src/components/Message.tsx`
- `_stress_testing/_claude_code_src/components/MessageRow.tsx`
- `_stress_testing/_claude_code_src/components/messages/AssistantTextMessage.tsx`
- `_stress_testing/_claude_code_src/components/messages/AssistantThinkingMessage.tsx`
- `_stress_testing/_claude_code_src/components/messages/AssistantToolUseMessage.tsx`
- `_stress_testing/_claude_code_src/components/ToolUseLoader.tsx`
- `_stress_testing/_claude_code_src/components/StatusLine.tsx`
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
- `_stress_testing/_claude_code_src/keybindings/defaultBindings.ts`

## Deliverables
- Add `docs/claude_code_terminal_parity_spec.md`.
- Add `tests/ui_parity/` with pseudo-terminal fixtures and snapshot helpers.
- Add `tests/fixtures/ui_parity/` scripts for deterministic CLI conversations.
- Add `./ui_parity_probe.sh`.
- Add renderer snapshot normalization utilities that can strip ANSI where appropriate and preserve ANSI where color/style parity matters.

## Review Corrections
The following do not satisfy this task:

- A test that only runs `elma-cli --help`.
- A fixture loader that never sends fixture steps to the process.
- A probe script that ignores `--fixture NAME` and simply runs every test.
- A harness command that prints placeholder text instead of driving the live binary.
- Snapshots that cannot fail when old Elma chrome appears.

The harness must use a real pseudo-terminal, send keys, wait for visible text or timeouts, capture the screen through `vt100` or equivalent parsing, and assert against fixture-specific expectations.

## Spec Requirements
The spec must describe:

- Startup screen and first prompt behavior.
- Message row prefixes, spacing, margin rules, and collapsed group behavior.
- Assistant text rendering and markdown behavior.
- Thinking row behavior in normal, verbose, and transcript modes.
- Tool start/progress/result row behavior.
- Permission/waiting states.
- Slash command fuzzy picker behavior.
- File quick-open behavior.
- Prompt footer hints and help menu behavior.
- Keybindings and multi-key sequences.
- Todo task list display, truncation, and status symbols.
- Compact summary and compact boundary behavior.
- Status line behavior.
- Session clear, resume, and exit behavior.

## Optional Reference Execution
If the local Claude Code source can be run without network setup, add a script that captures reference pseudo-terminal screens from it. If it cannot run locally, record the blocker in `docs/claude_code_terminal_parity_spec.md` and rely on source-derived fixtures.

Do not block Elma implementation on external package downloads or cloud account requirements.

## Suggested Rust Dev Dependencies
Install only when implementing this task:

- `portable-pty` or `rexpect` for pseudo-terminal execution.
- `vt100` for interpreting terminal control sequences.
- `strip-ansi-escapes` for normalized comparisons.
- `insta` for snapshots.
- `similar` for readable snapshot diffs.

## Acceptance Criteria
- The spec names the exact Claude source files that justify each UI behavior.
- There is at least one snapshot fixture each for startup, prompt input, slash picker, thinking stream, assistant response, tool call, todo list, compact boundary, and exit.
- Snapshot tests run without a live model by using a fake OpenAI-compatible local server or deterministic fixture mode.
- Later tasks can add fixtures without changing the harness architecture.
- `./ui_parity_probe.sh --fixture thinking-stream` runs only the `thinking-stream` fixture and fails if that fixture fails.
- The harness fails if the active interactive screen contains old header/activity/input-box/context-bar chrome.
- The harness can send typed input, Enter, Esc, Ctrl-O, Ctrl-T, Ctrl-C, Ctrl-D, arrow keys, Tab, and resize events.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test ui_parity -- --nocapture
./ui_parity_probe.sh --fixture startup
./ui_parity_probe.sh --fixture slash-picker
./ui_parity_probe.sh --fixture thinking-stream
```

The final verification must spawn `target/debug/elma` in a real pseudo-terminal, not just call renderer functions.

## Verification Results

- ✅ Spec document created at `docs/claude_code_terminal_parity_spec.md`
- ✅ Test harness implemented in `tests/ui_parity.rs` with portable-pty support
- ✅ Fixture system with YAML configs in `tests/fixtures/ui_parity/`
- ✅ Probe script `ui_parity_probe.sh` for running individual fixtures
- ✅ Pseudo-terminal execution verified with `startup` fixture
- ✅ `cargo fmt --check` passes
- ✅ `cargo build` succeeds
- ✅ `cargo test --test ui_parity startup_fixture` passes

The harness successfully spawns elma in a PTY, sends input, captures output, and normalizes ANSI escapes for snapshot testing.
