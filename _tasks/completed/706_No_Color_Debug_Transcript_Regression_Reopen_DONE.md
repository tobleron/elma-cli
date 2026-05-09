# Task 706: No-Color Debug Transcript Regression Reopen

## Type

Observability / Test Harness

## Severity

Medium

## Session Evidence

Even after Task 700 was marked completed, redirected prompt outputs such as `project_tmp/elma_round3_01_terminal.out` still contain cursor-control sequences and repeated streaming fragments when launched with `--debug-trace --no-color`.

The terminal output required cleanup with a Perl ANSI-strip command before it was readable during this round.

## Problem

Forensics should not require terminal escape cleanup. `--no-color` should produce a plain, stable transcript when stdout is redirected, while the interactive TUI can still use cursor control in a real terminal.

## Proposed Solution

Reopen this behavior as a regression task.

Likely source areas:

- `src/ui/ui_terminal.rs`
- `src/claude_ui/claude_render.rs`
- `src/session_write.rs`
- `src/terminal_transcript.txt` generation paths
- terminal detection / `--no-color` handling

Requirements:

- Detect non-TTY stdout and disable alternate-screen/cursor redraw output.
- Keep streaming fragments out of redirected terminal captures.
- Persist one canonical plain transcript per session.
- Add a regression test or smoke harness that asserts no ANSI escape/control sequences in redirected `--no-color` output.

## Acceptance Criteria

- [ ] `target/debug/elma-cli --debug-trace --no-color > out.txt` creates readable plain text.
- [ ] Redirected output does not contain cursor movement sequences.
- [ ] Session transcript includes final answer and final notices without repeated partial-token redraws.

## Verification Plan

Run one simple prompt and one tool-using prompt with stdout redirected. Search output for ANSI/control escape regex and compare final text against session transcript.

