# Task 798: Shell Redirection Awareness in System Prompt

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Prevent the model from repeatedly using shell redirection (`>`, `2>&1`, `|`) which is blocked by the safety preflight. In the session, 5+ shell commands were blocked for redirection, wasting iterations.

## Root Cause
The model doesn't know (or forgets) that shell redirection is restricted. It uses redirection as a reflex for capturing command output, even though the tool already captures stdout/stderr.

## Requirements
- Add explicit instruction in the system prompt: "Shell redirection operators (`>`, `>>`, `2>&1`, `|`) are blocked. The shell tool automatically captures stdout and stderr. Run commands without redirection."
- When a shell command is blocked by preflight, inject a context-specific hint: "This command was blocked for shell redirection. Re-run without `>` or `|`."
- Track shell preflight blocks and if 2+ consecutive blocks occur for the same reason, escalate to a hard rule injection.

## Failure Mode Fixed
- Repeated shell preflight blocks wasting iterations
- Model unawareness of tool constraints
