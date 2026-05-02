# Task 160: Background Shell Job Manager

## Backlog Reconciliation (2026-05-02)

Superseded by Task 460 background job tooling and Task 459 sandboxed execution profiles.


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

## Summary

Manage long-running shell commands in the background with job control (status, output, kill).

## Motivation

Crush supports running shell commands in the background:
- Long builds (`cargo build`, `npm run dev`)
- Servers (`python -m http.server`)
- File watching (`watchman`)

This allows users to continue using elma while commands run.

## Source

Crush's shell background package at `_stress_testing/_crush/internal/shell/background.go`

## Implementation

### Types

```rust
struct BackgroundShell {
    id: String,
    command: String,
    description: String,
    shell: Shell,
    working_dir: String,
    ctx: Context,
    cancel: CancelFunc,
    stdout: SyncBuffer,
    stderr: SyncBuffer,
    done: chan (),
    exit_err: Arc<Mutex<Option<Error>>>,
    completed_at: AtomicI64,
}

struct BackgroundShellManager {
    shells: Map<String, BackgroundShell>,
}
```

### Constants

```rust
const MaxBackgroundJobs = 50
const CompletedJobRetentionMinutes = 8 * 60  // 8 hours
```

### API

```rust
// Start new background shell
fn start(ctx, working_dir, command, description) -> Result<BackgroundShell, Error>

// Get background shell by ID
fn get(id) -> Option<BackgroundShell>

// Remove (cleanup only)
fn remove(id) -> Result<()>

// Kill (terminate)
fn kill(id) -> Result<()>

// List all IDs
fn list() -> Vec<String>

// Cleanup old completed jobs
fn cleanup() -> usize

// Kill all
fn kill_all(ctx)
```

### BackgroundShell Methods

```rust
// Get current output
fn get_output() -> (stdout, stderr, done, err)

// Check if done
fn is_done() -> bool

// Wait for completion
fn wait()

// Wait with context
fn wait_context(ctx) -> bool
```

### SyncBuffer

Thread-safe wrapper for bytes.Buffer:

```rust
struct SyncBuffer {
    buf: Mutex<bytes::Buffer>,
}

fn write(p: &[u8]) -> Result<usize, Error>
fn write_string(s: &str) -> Result<usize, Error>
fn string() -> String
```

### User Commands

```
/jobs           - List background jobs
/job 001       - Show output of job 001
/job 001 kill  - Kill job 001
```

### Integration

- Shell tool with `&` suffix for background
- BlockFuncs for streaming output
- Session integration

## Verification

- Background jobs start and track correctly
- Output streaming works
- Kill terminates properly
- Cleanup removes old jobs

## Dependencies

- Shell tool
- Session management (for working directory)

## Notes

- Not highest priority - can run commands in foreground
- Useful for dev workflows
- Max 50 concurrent jobs (prevent resource exhaustion)