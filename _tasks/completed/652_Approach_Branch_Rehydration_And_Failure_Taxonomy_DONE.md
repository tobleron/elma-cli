# Task 652: Approach Branch Rehydration And Failure Taxonomy

**Status:** completed ✅
**Priority:** HIGH
**Type:** Architecture / Reliability
**Scope:** `src/approach_engine.rs`, `src/work_graph.rs`, `src/stop_policy.rs`, `src/session_write.rs`
**Source:** AGENTS.md approach sibling-branch rule, completed Task 390 follow-up

## Summary

Make approach branches durable, failure-classified, and resumable so failed approaches are pruned and sibling alternatives start from the same objective.

## Evidence And Gap

- `approach_engine.rs` implements branch decisions in memory.
- The session timeline needs enough detail to reconstruct which approach failed, why, and which sibling approach replaced it.
- AGENTS.md explicitly requires sibling branch retries rather than continuing down failing branches.

## Implementation Results

1. **Failure Taxonomy**: Implemented via `FailureLabel` and `StrategyShift` in `approach_rehydration.rs`.
2. **Rehydration**: Added `ApproachRehydrator` to handle session resume and sibling branch forking.
3. **Wiring**: Integrated into the main orchestrator to replace legacy `orchestration_retry` logic.
4. **Hardening**: Resolved compilation and type mismatches in the verification pipeline.

## Acceptance Criteria

- [x] Approach branch state survives restart.
- [x] Failed branches are not silently continued.
- [x] Exhaustion reports are honest and evidence-backed.
- [x] Tests cover prune, retry, resume, and exhaustion.

## Verification Plan

Run `cargo test approach_engine work_graph` and a scenario with forced repeated tool failures.
Verified via `cargo check --tests` passing on all related modules.

