# Task: Persistent Guarded Shell

## Status
- **Type:** Engineering / Infrastructure
- **Priority:** CRITICAL
- **Status:** ACTIVE
- **Assigned to:** Elma (via Gemini CLI)

## Objective
Implement a persistent, long-running shell process for tool execution. This solves the "profile noise" bug by initializing the shell only once and provides state persistence (directory, env vars) across tool calls.

## Context
Spawning a new `sh -lc` for every command is slow and exposes Elma to "profile noise" (like `.zshrc` greetings) that wipes out command output in the PTY. A persistent shell remains open, allowing us to "flush" initial noise once and then exchange pure command/output pairs.

## Implementation Steps

### Phase 1: Persistence Infrastructure
1. **Define Shell Manager:** Create or update a struct to hold the persistent PTY child process, its stdin, and its stdout reader.
2. **Implement Marker-Based Capture:** 
   - Define a unique execution marker (e.g., `__ELMA_SHELL_DONE_[RANDOM]__`).
   - Append `echo [MARKER] $?` to every command sent to the shell.
   - Read from the PTY stream until the marker is detected.
3. **Handle Initial Flush:** On startup, read and discard all output until the first marker is hit to clear out login profile noise.

### Phase 2: Guarding & Safety
1. **Timeout Logic:** Implement a watchdog that kills and restarts the shell if a command doesn't return the marker within the timeout period.
2. **Reset Capability:** Add a `reset()` method to the shell manager to handle cases where the shell becomes "poisoned" (e.g., stuck in an interactive process).
3. **OS-Agnostic Shims:** Ensure the command wrapping logic works for both POSIX (macOS/Linux) and Windows (PowerShell).

### Phase 3: Integration
1. **Update `program_utils.rs`:** Replace `run_shell_one_liner_via_pty` with a call to the persistent shell manager.
2. **Refactor `tool_calling.rs`:** Ensure the `exec_shell` function correctly interacts with the persistent state.
3. **Telemetry:** Ensure tool duration and exit codes are correctly captured from the marker.

## Success Criteria
1. **Pure Output:** Commands like `date` return only the date, with zero profile noise.
2. **State Persistence:** Running `cd /tmp` in one tool call is reflected in the working directory of the next call.
3. **Robust Recovery:** Starting a hanging process (like `cat`) is timed out, the shell is reset, and Elma remains functional.
4. **Performance:** Shell tool calls become significantly faster after the first initialization.
