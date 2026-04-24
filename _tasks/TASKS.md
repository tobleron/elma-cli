# Task Management - Follow Instructions In Exact Order

## Task Creation Rule

### Main Project Tasks (Numbered 0XX / 1XX / 2XX)
- Every new task uses the next available numeric prefix across `_tasks/active/`, `_tasks/pending/`, `_tasks/completed/`, and `_tasks/postponed/`.
- Use three-digit padding.
- Task files must be self-documenting enough that status can be inferred from filename and header.

### Dev-System Tasks (D###)
- Stored in `_dev-tasks/`.
- Advisory only.

### Troubleshooting Tasks (T###)
- Use the same numeric sequence with a `T` prefix.
- Start immediately when a real regression or failure class is being investigated.

## Current Master Plan
- **Task 191**: Elma Skill-First Rebaseline Master Plan — COMPLETE.
- Supporting active tracks: All complete (023, 064, T206-T209, T210).
- Next pending tasks: 213-234 (dependency-free library additions).


## Supporting Active Verification Gate
- **204** Task 191 completion verification and Task 203 unblock gate.

## Canonical Implementation Sequence
1. ✅ **193** Project guidance loader and `/init` scaffold.
2. ✅ **194** Skill runtime, formula catalog, and predictive main-task gate.
3. ✅ **195** Runtime task engine and dual ledger.
4. ✅ **192** Stuck detection and stop policy.
5. ✅ **199** `/skills` and execution-plan UX.
6. ✅ **205** Transcript-native runtime telemetry and final-answer presentation.
7. ✅ **196** Repo explorer and analyzer skill.
8. ✅ **197** Document intelligence skill stack.
9. ✅ **198** Read-only whole-system file scout skill.
10. ✅ **202** Project task steward skill and task protocol.
11. ✅ **203** Extended ebook and archival format adapters.
12. ✅ **200** Branded splash and compact header.
13. ✅ **201** Final inventory normalization and instruction-drift cleanup pass.
14. 🔄 **204** Task 191 completion verification and Task 203 unblock gate.

## Dependency Notes
- `194` depends on `193` being in place or at least stabilized enough that guidance is available to the selector.
- `195` depends on `194` because runtime task records are seeded from `ExecutionPlanSelection`.
- `192` depends on `194` and `195` because budgets and stop reasons must bind to formula stages and persisted main tasks.
- `199` depends on `194` and `195` because the UI must show execution-plan and runtime-task state.
- `205` depends on `194`, `195`, and `199` because transcript telemetry and final-answer presentation need execution-plan state, runtime task state, and the `/skills` surface.
- `197` and `198` should be implemented together conceptually, but `197` owns extraction and `198` owns discovery.
- `202` must not absorb generic runtime task persistence from `195`.
- `203` is blocked on the normalized document pipeline from `197`.
- `200` should land after the task/formula UX surfaces exist so the header and splash expose real state.
- `201` is a final consistency pass and should be revisited any time instruction drift appears.
- `206` (thiserror) should land before `209` (miette) and `214` (color-eyre).
- `208` (tracing) and `214` (color-eyre) are independent but both land post-sequence.
- `210` (clap_complete) and `218` (clap_mangen) depend only on the existing `clap` setup and can be done in any order.
- `211` (dialoguer) depends on `inquire` being present (already in Cargo.toml).
- `226` (zip) should land before or alongside `233` (quick-xml / DOCX).
- `225` (serde_with) should land after `206` (thiserror) since both touch serde error patterns.
- `227` (once_cell) is low-priority and can land at any time post-sequence.
- `228` (derive_more + itertools), `229` (tap), `230` (strum) are ergonomic utilities — no dependencies, can be parallelized.
- `231` (ron) and `232` (toml_edit) are independent config-layer additions.
- `234` (comrak) is a drop-in evaluation for the markdown renderer — can be parallelized with `222` (tokio-stream).

## Workflow Instructions
1. Pickup: move the intended task to `_tasks/active/` if starting formal implementation.
2. Implement surgically.
3. Verify with `cargo build`.
4. Verify with relevant tests and real CLI or PTY validation.
5. Report while the task is still active.
6. Archive only after approval.

## Folder Meaning
- `_tasks/active/`: current implementation tracks.
- `_tasks/pending/`: next approved work in the task-first, formula-driven direction.
- `_tasks/completed/`: finished work.
- `_tasks/postponed/`: deferred, absorbed, or superseded work kept for history.
- `_dev-tasks/`: analyzer guidance.
