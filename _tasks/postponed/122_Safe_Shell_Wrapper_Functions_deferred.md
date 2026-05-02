# Task 122: Safe Shell Wrapper Functions

## Backlog Reconciliation (2026-05-02)

Resume through Task 442 path/argument hardening, Task 458 shell mutation rollback, and Task 459 execution profiles.


## Priority
**P1 — Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection)

## Problem

The model generates raw shell commands with no safety wrapper. `mv`, `rm`, and other destructive operations execute directly. Safe Rust wrapper functions would validate, scope, and log all mutations.

## Scope

### 1. Wrapper Functions
- `safe_mv(source, destination, workdir)` — validates paths, checks counts, logs operation
- `safe_rm(pattern, workdir)` — expands glob, shows count, confirms, then deletes
- `safe_cp(source, destination, workdir)` — validates both paths
- `safe_mkdir(path, workdir)` — creates parent directories if needed
- `safe_write_file(path, content, workdir)` — validates path, creates backup

### 2. Model Exposure
- Expose wrappers as additional tools: `safe_mv`, `safe_rm`, etc.
- Model learns to prefer wrappers over raw `shell` for mutations
- Raw `shell` still available for non-mutation commands

### 3. Integration Points
- `src/tool_calling.rs` — add safe wrapper tool definitions + execution
- `src/safe_shell.rs` (new) — wrapper implementations

## Design Principles
- **Small-model-friendly:** Wrappers return clear success/failure messages
- **Principle-first:** Wrappers encode safety principles, not just convenience
- **Backward compatible:** Raw `shell` tool still available for experts

## Verification
1. `cargo build` clean
2. `cargo test` — each wrapper validates correctly
3. Real CLI: model uses `safe_mv` → validates, executes, logs

## Acceptance Criteria
- [ ] Safe wrapper tools available to model
- [ ] Wrappers validate before mutating
- [ ] Wrappers return clear success/failure messages
- [ ] Raw shell still available for non-mutation commands
- [ ] Wrapper operations logged to session trace
