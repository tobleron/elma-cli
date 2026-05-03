# 571 — Implement Session Snapshot and Rollback System

- **Priority**: Medium
- **Category**: State Management
- **Depends on**: 458 (already marked DONE but may need enhancement), 554 (session state)
- **Blocks**: None

## Problem Statement

The existing snapshot system (`snapshot.rs`) creates snapshots before risky shell commands (Task 458). However:
1. Snapshots are only created for shell commands, not for edit/write/patch operations
2. There's no user-facing rollback command
3. Snapshot metadata may not capture enough context for informed rollback decisions
4. No automatic rollback on tool failure

## Why This Matters for Small Local LLMs

Small models are more likely to make destructive mistakes (editing the wrong file, writing incorrect content, deleting needed files). A robust snapshot/rollback system provides a safety net.

## Current Behavior

```rust
// tool_calling.rs - exec_shell (lines 344-358)
if matches!(preflight.risk, RiskLevel::Caution | RiskLevel::Dangerous(_)) {
    match crate::snapshot::create_workspace_snapshot(
        session, workdir, &format!("pre-shell snapshot before: {}", command), true,
    ) {
        Ok(snapshot) => { trace(...); }
        Err(e) => { trace(...); }
    }
}
```

Only shell commands with Caution/Dangerous risk get snapshots. Edit, write, patch, move, and trash operations do not.

## Recommended Target Behavior

1. Extend snapshot creation to ALL mutating tool operations (edit, write, patch, move, trash, copy)
2. Add `/rollback` command to restore workspace to a previous snapshot
3. Add `rollback_last()` convenience function for undoing the most recent mutation
4. Integrate with the tool lifecycle (Task 560) for automatic pre-mutation snapshots
5. Add snapshot listing and inspection

## Source Files That Need Modification

- `src/snapshot.rs` — Enhance snapshot system
- `src/tool_calling.rs` — Add snapshot calls to all mutating tools
- `src/tool_lifecycle.rs` (after Task 560) — Add snapshot stage
- `src/input_parser.rs` — Add `/rollback` command

## New Files/Modules

- `src/rollback.rs` — Rollback execution logic

## Acceptance Criteria

- Snapshots created before ALL mutating tool operations
- `/rollback` command available to users
- Snapshot listing shows timestamp, triggering tool, affected files
- Rollback restores exact pre-mutation state
- Snapshots cleaned up on session end (configurable retention)

## Risks and Migration Notes

- Disk space: Snapshots can be large for big workspaces. Add configurable retention (keep last N, max total size).
- Git-based workspaces: Consider integrating with git (git stash/git reset) instead of file copies for efficiency.
