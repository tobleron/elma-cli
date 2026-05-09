# Task 784: Add Timeout Guards to Blocking Shell Calls in Async Context

## Type
Reliability / I/O Safety

## Severity
High

## Scope
Multiple modules

## Problem

Several code paths execute blocking shell commands (`std::process::Command::new()`, `run_shell_persistent_sync()`) without timeouts, inside an async Tokio runtime. A hanging shell command will block a Tokio worker thread indefinitely, potentially deadlocking the entire application.

**Confirmed unguarded blocking calls in production paths:**

| File | Line | Command | Risk |
|------|------|---------|------|
| `shell_preflight.rs` | 336 | `run_shell_persistent_sync(&dry_cmd, workdir)` | Runs `find | wc -l` on arbitrary directory — can hang on NFS/network mounts |
| `tool_trait.rs` | 143 | `Command::new("sh").arg("-c")` | Tool evaluation — no timeout |
| `tools/implementations/git_inspect.rs` | 34 | `Command::new("git")` | Git operations on large repos can be slow |
| `tools/implementations/workspace_info.rs` | 98, 112 | `Command::new("git")` | Git branch/status — can hang if git index locked |
| `tools/implementations/search.rs` | 28 | `Command::new("rg")` | Ripgrep on large directories — unbounded |
| `tools/helpers.rs` | 80 | `Command::new("cargo")` | Cargo build — very long-running |
| `execution_steps.rs` | 784 | `Command::new("rg")` | Search step — unbounded |

## Root Cause

Shell command execution was added incrementally without a unified timeout contract. The `command_budget.rs` module tracks budgets but does not enforce per-command timeouts.

## Proposed Solution

Phase 1: Create a `shell_timeout.rs` utility:
```rust
pub(crate) async fn run_with_timeout(
    cmd: &str,
    workdir: &Path,
    timeout: Duration,
) -> Result<ShellResult, ShellTimeoutError>
```

Phase 2: Replace every `std::process::Command::new()` in production code with the timeout-guarded version. For `shell_preflight.rs:336`, wrap in `tokio::time::timeout`.

Phase 3: Set default timeouts based on command type:
  - Git operations: 10s
  - Search operations: 30s
  - Build operations: 120s
  - Preflight dry-run: 5s

## Acceptance Criteria
- [ ] No `std::process::Command::new()` call in production code lacks a timeout
- [ ] `run_shell_persistent_sync()` calls from async contexts are wrapped in `tokio::time::timeout`
- [ ] Default timeout values are configurable via `CommandBudget`
- [ ] Timeout errors produce clear user-facing messages
- [ ] `cargo test` passes

## Verification Plan
- Unit test: Verify timeout fires for a `sleep 100` command
- Integration test: Existing shell execution tests pass
- Regression test: `cargo test`

## Dependencies
None.

## Notes
- **Architectural Rule violated:** Rule 7 (Real Timeout Mechanisms) — "Do add real timeout mechanisms for blocking I/O instead of relying on model self-correction"
- **AGENTS.md Rule 7:** "Do add real timeout mechanisms for blocking I/O"
- This is a reliability hazard — a single hung NFS mount or locked git index will freeze Elma permanently
