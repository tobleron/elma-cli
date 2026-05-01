# Task 391: Instruction-Level Repair And Result Recombiner

**Status:** Pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Dependencies:** Task 379, Task 389, Task 390
**References:** user objective for miniaturized failure points

## Objective

Make every instruction in the work graph independently repairable, retryable, and summarizable, then recombine successful instruction outputs into the next level of the graph.

## Problem

If an instruction fails, Elma should repair only that instruction, not restart or bloat the whole request. If enough instruction results succeed, Elma should combine them into a grounded goal/sub-goal result.

## Implementation Plan

1. Add an `InstructionOutcome` type with:
   - `status`
   - `result_summary`
   - `evidence_refs`
   - `repair_needed`
2. Add an instruction repair selector with simple JSON:
   - `repair_action`
   - `reason`
   - `retryable`
3. Implement repair actions:
   - tighten context
   - choose native tool
   - request missing evidence
   - split instruction
   - abandon branch
4. Add a recombiner that only uses successful outcomes and evidence references.
5. Make recombination fail closed when required evidence is missing.

## Verification

```bash
cargo test instruction
cargo test recombine
cargo test evidence
cargo test orchestration
cargo build
```

## Done Criteria

- A failed instruction can be repaired without restarting the whole objective.
- Recombined results are evidence-grounded.
- Missing evidence blocks unsupported final claims.
- Repair decisions obey Task 378 JSON limits.

