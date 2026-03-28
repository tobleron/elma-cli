# Task 007: Add Snapshots And Rollback Commands

## Objective
Add efficient workspace snapshotting and rollback support so Elma can preserve recovery points before editing and the user can explicitly restore a chosen snapshot with `/rollback`.

## Context
This task improves editing safety and operational confidence, but it is not the next best solve-rate improvement for Elma's reasoning stack. It should follow the higher-priority verification and planning tasks.

The user explicitly requested:
- snapshot IDs
- automatic snapshotting before editing workflows when appropriate
- `/snapshot` command to create a snapshot manually
- `/rollback <snapshot_id>` command to restore a specific snapshot

## Work Items
- [ ] Design a snapshot storage layout under sessions or another dedicated recovery path.
- [ ] Define a short, user-friendly snapshot ID format.
- [ ] Implement snapshot creation for:
  - manual `/snapshot`
  - automatic pre-edit snapshotting
- [ ] Decide snapshot scope and restore semantics:
  - tracked files only vs workspace subset
  - metadata needed for restore
  - fast rollback vs full-copy tradeoffs
- [ ] Implement `/rollback <snapshot_id>`.
- [ ] Add restore verification and concise user-facing reporting.
- [ ] Integrate snapshot awareness into edit workflows and edit responses.

## Acceptance Criteria
- `/snapshot` creates a usable snapshot and returns an ID.
- Pre-edit workflows can create snapshots automatically.
- `/rollback <snapshot_id>` restores the workspace to that snapshot.
- Snapshot metadata is clear enough for inspection and troubleshooting.
- Rollback verification is explicit and trustworthy.

## Verification
- `cargo build`
- `cargo test`
- live probes for manual snapshot creation, pre-edit snapshotting, rollback, and post-rollback verification
