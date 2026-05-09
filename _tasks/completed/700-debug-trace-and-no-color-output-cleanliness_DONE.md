# Task 700: Debug Trace And No-Color Output Cleanliness

## Type

Observability

## Severity

Medium

## Scope

Terminal output, session transcript, debug artifacts

## Session Evidence

Round 2 terminal capture files such as `project_tmp/elma_round2_01_terminal.out` and `project_tmp/elma_round2_02_terminal.out` contain ANSI cursor-control sequences and repeated streaming fragments even when Elma is launched with `--no-color`.

This made forensic inspection noisy and required external cleanup commands to read the output. The structured `session.md` was more useful, but it did not always include the full final displayed transcript.

## Problem

Debug artifacts should be directly readable when `--no-color` is passed. If terminal captures contain control sequences and repeated streaming fragments, test analysis becomes slower and harder to automate.

## Proposed Solution

Clean debug output paths:

- Make `--no-color` disable ANSI styling and cursor-control escape sequences in redirected terminal output.
- Persist a plain final transcript artifact separate from the interactive terminal rendering stream.
- Ensure `session.md` includes the authoritative final answer and important finalization notices.
- Add a small regression test that runs a headless prompt and asserts the terminal output has no escape sequences.

## Acceptance Criteria

- [ ] `target/debug/elma-cli --debug-trace --no-color > out.txt` produces readable plain text.
- [ ] Streaming partial-token redraws do not dominate redirected output.
- [ ] `session.md` and plain transcript artifacts include the same final answer and final notices.

## Verification Plan

Run one simple prompt and one tool-using prompt with redirected output. Check the output with a regex for ANSI escape sequences and compare final-answer text against `session.md`.

