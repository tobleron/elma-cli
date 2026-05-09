# Task 796: Pre-Build Binary Existence Check

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Before spending iterations searching for a compiled binary, always check if it exists at the expected path. In the session, the agent searched for `elma-cli` in `target/release` 15+ times without ever running `cargo build --release` first.

## Root Cause
The model assumes binaries exist rather than verifying. It prioritizes search over build.

## Requirements
- For Rust projects: before any glob/search/ls for a binary, run `stat` or `ls` on the expected path (`target/release/<name>`, `target/debug/<name>`).
- If the binary doesn't exist, immediately run `cargo build --release` (or `cargo build` if debug is acceptable).
- Add a prompt instruction: "Before searching for a compiled binary, verify it exists with a direct file check. If absent, build the project first."

## Failure Mode Fixed
- Wasted iterations searching for non-existent binaries
- Premature respond abuse (answering without evidence)
- Stagnation from repeated failed searches
