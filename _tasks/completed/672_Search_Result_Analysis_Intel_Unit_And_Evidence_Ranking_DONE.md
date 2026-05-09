# Task 672: Search Result Analysis Intel Unit And Evidence Ranking

**Status:** pending
**Priority:** MEDIUM
**Type:** Model Robustness / Evidence
**Scope:** `src/intel_units/`, `src/evidence_ledger.rs`, `src/tool_calling.rs`, `src/search*`
**Source:** deferred task 505

## Summary

Add a focused search-result analysis unit that ranks grounded local search evidence before the model reasons over it.

## Evidence And Gap

- Search can return many lines, but later reasoning may overfocus on noisy matches.
- A narrow intel unit can reduce cognitive load for small models without bloating prompts.
- Must remain strict JSON and principle-first.

## Implementation Plan

1. Summarize search results into path clusters, exact matches, surrounding context quality, and likely next reads.
2. Return compact JSON: selected paths, reason, and confidence/entropy.
3. Feed ranked evidence into follow-up read/repo-map steps.
4. Persist ranking decisions as evidence metadata.

## Acceptance Criteria

- [ ] No keyword routing decisions are added.
- [ ] Search result ranking is grounded in actual output lines/paths.
- [ ] Follow-up reads prefer high-quality evidence.
- [ ] Tests cover noisy, empty, and multi-file search results.

## Verification Plan

Run search fixtures and verify ranked paths match expected evidence.

