# Task 806: Shell Output Fragments — Prevent Malformed Commands

## Status
- **Priority:** Low
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Prevent the model from constructing shell commands from truncated/fragmented output. In the session, the agent ran `cat 'ago)'` — a command constructed from a truncated file listing line fragment.

## Root Cause
Large directory listings are truncated in the transcript (shown as `… [+33 characters truncated]`). The model picks up a fragment like `ago)` from the truncated output and constructs a nonsense command from it.

## Requirements
- When output is truncated, clearly mark the truncation boundary and warn: "Content truncated — do not construct commands from fragments after this point."
- Add a shell command sanity check: if the command contains unmatched quotes, unbalanced parentheses, or is a single word that looks like a file listing fragment (e.g., ends with `)`), warn before executing.
- Consider wrapping shell tool input with a validator that checks for obviously malformed commands.

## Failure Mode Fixed
- Malformed shell commands from truncated output fragments
- Wasted iterations on commands that can never succeed
