# Task 070: Hierarchical Evidence Compaction

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
**P2 - EFFICIENCY & OBSERVABILITY (Tier B)**
**Depends on:** Tier A stability (tasks 065-069)

---

# Task 027: Implement Hierarchical Evidence Compaction

## Context
`compact_evidence_once` currently processes raw output in a single call. Large outputs can exceed model context or cause poor summary quality.

## Objective
Implement a "Map-Reduce" style compaction for large shell outputs:
- Split large outputs into manageable chunks.
- Process chunks in parallel to extract key facts.
- Synthesize chunk-level facts into a final, coherent evidence summary.
- Update `src/intel_compression.rs` (from Task 024) with this logic.

## Success Criteria
- System handles 10,000+ line command outputs without context overflow.
- Summary quality remains high regardless of input size.
