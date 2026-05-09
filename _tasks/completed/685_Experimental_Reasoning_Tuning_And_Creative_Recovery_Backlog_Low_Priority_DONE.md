# Task 685: Experimental Reasoning Tuning And Creative Recovery Backlog Low Priority

**Status:** pending
**Priority:** LOW
**Type:** Research / Model Robustness
**Scope:** `src/intel_units/`, `src/tune*`, `config/`, scenarios
**Source:** postponed tasks 077-085, 086, 090-093, 161

## Summary

Keep experimental cross-scenario correlation, tactical memory, predictive failure, constraint relaxation, analogy reasoning, and autonomous prompt evolution as low-priority research.

## Evidence And Gap

- These ideas can help robustness but risk bloating prompts, overfitting, or violating principle-first prompt constraints.
- Core decomposition, strict JSON, evidence, and session reliability should land first.

## Implementation Plan

1. Treat each experiment as a small, focused intel unit or deterministic analyzer, never as prompt bloat.
2. Require offline scenarios and measurable improvements before activation.
3. Keep autonomous prompt mutation disabled unless the user explicitly approves a separate prompt-governance task.
4. Prefer failure taxonomies and retry planners over broad self-learning.

## Acceptance Criteria

- [ ] No experimental feature changes core prompts by default.
- [ ] Each experiment has a hypothesis, metric, and rollback path.
- [ ] Failed experiments stay disabled and documented.
- [ ] Small-model effectiveness improves without harming truthfulness or context efficiency.

## Verification Plan

Run controlled scenario A/B tests with protected baselines.

