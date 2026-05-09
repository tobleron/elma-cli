# Task 805: Timeout-Aware Build Strategy

## Status
- **Priority:** Medium
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Prevent `cargo build --release` timeouts by checking for pre-existing binaries and offering incremental/non-blocking build options. In the session, two consecutive `cargo build --release` commands timed out after 30 seconds of no output, wasting iterations.

## Root Cause
The model runs full release builds without checking if the binary already exists or if an incremental build would suffice. It doesn't handle long-running commands gracefully.

## Requirements
- Before running `cargo build --release`, check `stat target/release/<binary>` to see if it already exists and is recent.
- If the binary exists and is newer than the source files, skip the build.
- If building is needed, consider:
  - Running `cargo build` (debug, faster) instead of `--release` when optimization isn't explicitly required.
  - Using `cargo build --release` as a background task with polling.
- Add a shell timeout hint: "Build commands may take >30s. Check for existing binaries first. If you must build, increase the timeout or run in background."
- After a timeout, inject: "The build timed out. Check if the binary already exists before retrying."

## Failure Mode Fixed
- Repeated build timeouts
- Not checking for existing binaries before building
