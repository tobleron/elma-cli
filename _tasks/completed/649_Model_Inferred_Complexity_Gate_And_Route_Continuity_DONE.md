# Task 649: Model Inferred Complexity Gate And Route Continuity

**Status:** pending
**Priority:** CRITICAL
**Type:** Architecture / Model Robustness
**Scope:** `src/app_chat_loop.rs`, `src/complexity_assessor.rs`, `src/routing_infer.rs`, `src/intel_units/`
**Source:** AGENTS.md complexity rule, postponed task 096, deferred/task drift audit, `_knowledge_base` orchestration comparisons

## Summary

Replace runtime heuristic complexity/routing gates with a model-informed strict JSON decision path and continuity checks from raw prompt through final answer.

## Evidence And Gap

- `app_chat_loop.rs` calls `complexity_assessor::assess_complexity(&rephrased_objective)` and maps it to graph depth.
- `docs/ARCHITECTURE.md` still describes a conservative heuristic route table in places.
- AGENTS.md says complexity is the main gate and must never be bypassed.

## Implementation Plan

1. Introduce a compact complexity intel unit returning `DIRECT`, `INVESTIGATE`, `MULTISTEP`, or `OPEN_ENDED` with confidence/entropy.
2. Use bounded deterministic fallback only when model output is invalid or unavailable.
3. Record raw prompt, intent annotation, complexity, route, graph depth, and final-answer continuity in a visible operational timeline row.
4. Add scenario fixtures for trivial chat, repo investigation, multi-file edits, long open-ended tasks, and ambiguous user input.

## Acceptance Criteria

- [ ] Graph depth is decided by the complexity gate before work begins.
- [ ] Route/complexity decisions are not based on hardcoded prompt word triggers.
- [ ] Semantic continuity evidence is persisted and transcript-visible.
- [ ] Tests catch direct-planning bypass regressions.

## Verification Plan

Run routing/complexity scenario tests and inspect sessions for route/complexity rows.

