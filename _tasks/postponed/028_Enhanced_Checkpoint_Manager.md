# Task 164: Enhanced Checkpoint Manager (Shadow Git)

## Backlog Reconciliation (2026-05-02)

Superseded by Task 472 session rewind/checkpoint UX and Task 458 shell mutation rollback coverage.


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

Transparent filesystem snapshots before file-mutating operations using shadow git repos. Provides rollback to any previous checkpoint.

## Motivation

- Rollback after destructive operations
- No git state leaks into user projects
- Automatic before writes

## Source

Hermes `tools/checkpoint_manager.py`

## Architecture

```
{hermes_home}/checkpoints/{sha256(abs_dir)[:16]}/
    HEAD, refs/, objects/    — git internals
    HERMES_WORKDIR          — original dir path
    info/exclude            — default excludes
```

Uses GIT_DIR + GIT_WORK_TREE for isolation.

### Trigger

- write_file, patch operations
- Once per conversation turn

### Features

- **Shadow git repo**: No .git in project dir
- **Auto-excludes**: node_modules, .env, __pycache__, venv/, .git/
- **Max files**: 50,000 (skip huge directories)
- **Validation**: Commit hash validation to prevent git injection
- **Path traversal protection**: Restore targets must be relative to workdir

### API

```rust
fn create_checkpoint(working_dir: &str) -> Result<String>  // Returns commit hash

fn restore_checkpoint(working_dir: &str, commit_hash: &str) -> Result<()>

fn list_checkpoints(working_dir: &str) -> Vec<CheckpointInfo> {
    commit: String,
    timestamp: DateTime,
    message: String,
}
```

### Excluded Paths

```rust
const DEFAULT_EXCLUDES = [
    "node_modules/",
    "dist/",
    "build/",
    ".env",
    ".env.*",
    "__pycache__/",
    "*.pyc",
    ".DS_Store",
    "*.log",
    ".cache/",
    ".next/",
    ".nuxt/",
    "coverage/",
    ".pytest_cache/",
    ".venv/",
    "venv/",
    ".git/",
]
```

## Security

### Commit Hash Validation

```rust
fn validate_commit_hash(hash: &str) -> Result<(), String> {
    if hash.starts_with('-') {
        return Err("Invalid: starts with '-'");
    }
    if !hash.matches(r"^[0-9a-fA-F]{4,64}$") {
        return Err("Invalid: not hex chars");
    }
    Ok(())
}
```

### Path Validation

```rust
fn validate_restore_path(path: &str, workdir: &str) -> Result<()> {
    // Must be relative to workdir
    // Must not escape workdir
    // No .. traversal to parent
}
```

## Implementation Notes

- No tool for LLM to see - transparent infrastructure
- Enabled via config: `checkpoints: true` or CLI: `--checkpoints`
- Git timeout: 30s default

## Verification

- Checkpoint created before write
- Restore to previous works
- No .git in project directory

## Dependencies

- Git CLI
- File tools (existing)