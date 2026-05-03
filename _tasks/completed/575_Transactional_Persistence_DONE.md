# 575 — Refactor Session State Persistence to Be Transactional

- **Priority**: High
- **Category**: State Management
- **Depends on**: 554 (session-scoped state), 562 (FSM)
- **Blocks**: None

## Problem Statement

Session state is persisted through multiple independent write operations:
- `session.json` (via `session_write.rs`)
- `evidence/ledger.json` (via `evidence_ledger.rs::persist()`)
- `event_log.json` (via `event_log.rs::change_log`)
- `sessions/<id>/runtime_tasks/tasks.json` (via `task_persistence.rs`)
- Tool artifacts in `sessions/<id>/artifacts/`

These writes are not coordinated. If Elma crashes or is killed between writes:
- `session.json` might reference evidence entries that weren't persisted
- Event log might be missing tool execution events
- Task state might diverge from conversation state

This is critical for session resume — partially written state can make a session unrecoverable.

## Why This Matters for Small Local LLMs

Sessions with small models are longer (more iterations needed) and more failure-prone. Session resume is more important for small-model workflows because tasks often don't complete in one session.

## Current Behavior

```rust
// evidence_ledger.rs — writes to TWO places
pub(crate) fn persist(&self) -> Result<()> {
    // Write 1: session.json evidence field
    crate::session_write::mutate_session_doc(&session_root, |doc| {
        doc["evidence"] = compact;
    });
    // Write 2: evidence/ledger.json
    std::fs::write(&ledger_path, json)
}
```

If Write 1 succeeds and Write 2 fails, `session.json` references evidence entries whose raw data never hit disk.

## Recommended Target Behavior

1. **Single source of truth**: All session state in `session.json` (or SQLite, per Task 277 which is DONE)
2. **Atomic writes**: Write to temp file, fsync, rename — never write in-place
3. **Consistency check on load**: Validate that all referenced artifacts/evidence exist
4. **Crash recovery**: On session load, detect incomplete writes and repair or warn

## Source Files That Need Modification

- `src/session_write.rs` — Add transactional write helpers
- `src/evidence_ledger.rs` — Use transactional writes
- `src/event_log.rs` — Use transactional writes
- `src/task_persistence.rs` — Use transactional writes
- `src/session_store.rs` — SQLite already provides transactions; ensure consistent use
- `src/session.rs` — Add integrity check on load

## New Files/Modules

- `src/atomic_write.rs` — `atomic_write(path, content)` helper with temp-file-then-rename

## Step-by-Step Implementation Plan

1. Create `src/atomic_write.rs`:
   ```rust
   pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
       let temp = path.with_extension("tmp");
       std::fs::write(&temp, content)?;
       temp.fsync()?; // ensure durability
       std::fs::rename(&temp, path)?;
       Ok(())
   }
   ```
2. Update all session persistence to use `atomic_write`
3. Add `validate_session_integrity()` to session load
4. Consolidate writes where possible (single write instead of multi-file scatter)
5. Add crash recovery test (simulate kill during write, verify next load recovers)

## Recommended Crates

- `tempfile` — already a dependency; for temp file creation
- `fsync` — platform-specific; use `File::sync_all()` on Unix

## Acceptance Criteria

- All session state writes use atomic write-then-rename
- Session load validates cross-file consistency
- Crash during write does not corrupt existing session state
- Recovery test passes

## Risks and Migration Notes

- SQLite session store (Task 277) already provides transactions — ensure it's used for ALL session writes, not just some.
- Atomic writes don't guarantee cross-file consistency (two files written atomically can still diverge). Consider single-file session state or proper two-phase commit.
