# Task 094: Snapshot Coverage For Shell Mutations And Rollback Integrity

## Backlog Reconciliation (2026-05-02)

Superseded by Task 458. Use this file as historical acceptance criteria for shell mutation rollback coverage.


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

## Priority
**P1 - RELIABILITY CORE (Tier A)**
**Created:** 2026-04-03
**Basis:** Live snapshot audit during CLI stability work

## Status
**PENDING**

## Objective
Close the recovery gap between structured `Edit` steps and shell-driven file mutations so Elma can reliably restore sandbox work after bad edits, even when the mutation happened through `Shell`.

## Current Reality
- Manual `/snapshot` and `/rollback <id>` are implemented and working.
- Automatic pre-edit snapshots are implemented for structured `Edit` steps.
- Rollback restores tracked files and removes files created after the snapshot.
- The current automatic coverage does **not** reliably protect shell-based file mutations such as:
  - `python3` / `perl` / `sed -i` file rewrites
  - heredoc file creation from shell
  - `mv`, `cp`, `rm`, or similar workspace-changing shell operations

## Why This Matters
Elma already uses bounded shell fallbacks to keep small local models reliable. That makes shell mutation a real runtime behavior, not an edge case. If shell-based edits are not snapshotted automatically, recovery is incomplete exactly when autonomy becomes more useful.

## Scope
- Detect shell steps that are likely to mutate workspace files.
- Create one automatic recovery snapshot before the first risky shell mutation in a workflow, reusing the same snapshot id for subsequent risky mutations in that run.
- Surface the snapshot id consistently in trace/session artifacts for shell mutations, the same way structured edits already do.
- Verify rollback integrity after mixed `Shell` + `Edit` mutation workflows.
- Keep the implementation sandbox-safe and principle-based; do not add brittle word lists that try to understand arbitrary shell intent exhaustively.

## Proposed Implementation Direction
1. Add a narrow shell-mutation risk detector based on command shape and file-operation semantics already visible in program policy/helpers.
2. Trigger `create_workspace_snapshot(...)` once before the first risky shell mutation executes.
3. Reuse `ExecutionState.auto_snapshot_id` so the workflow has a single recovery anchor.
4. Record snapshot metadata in step summaries/artifacts for risky shell steps.
5. Add tests that prove rollback works after:
   - shell-created file
   - shell-modified existing file
   - mixed shell mutation followed by structured `Edit`

## Acceptance Criteria
- Risky shell mutation steps automatically create a recovery snapshot before mutating workspace state.
- Non-mutating read-only shell steps do not create unnecessary snapshots.
- Mixed shell/edit workflows still use one coherent automatic snapshot id.
- `/rollback <id>` successfully restores the workspace after shell-created and shell-modified files.
- `cargo build` passes.
- `cargo test` passes.
- At least one real CLI sandbox mutation flow demonstrates shell auto-snapshot creation plus rollback success.

## Notes
- This task complements, but does not replace, manual `/snapshot`.
- The goal is not perfect shell-intent understanding; the goal is reliable protection for the bounded mutation patterns Elma actually emits in production workflows.
