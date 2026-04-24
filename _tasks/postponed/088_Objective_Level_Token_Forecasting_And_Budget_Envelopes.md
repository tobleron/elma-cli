# Task 088: Objective-Level Token Forecasting And Budget Envelopes

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
**Depends on:** Task 087 (Token Telemetry)

## Objective
Teach Elma to estimate the likely token cost of a complex objective before and during execution, so it can reason about the affordability of a workflow, not just react after the context window is already under pressure.

## Why This Exists
For small local models, high-quality orchestration requires more than a good plan. Elma must also know whether the plan is affordable within the available context budget.

This task makes Elma budget-aware at the objective level:
- estimate the likely token cost of planning, execution, verification, retries, and presentation
- forecast whether the current workflow is cheap, moderate, or expensive relative to remaining context
- expose a budget envelope that later tasks can use to trigger conservation strategies

## Scope
- Define a forecasting model for expected token usage across workflow phases:
  - routing and helper units
  - ladder/planning units
  - program generation / repair
  - execution evidence accumulation
  - verification / reviewer / revision passes
  - final presentation
- Use real telemetry from Task 087 plus configurable heuristics to estimate:
  - expected cost for the current turn
  - projected cost for a multi-step workflow
  - retry-adjusted worst-case range
  - safety reserve before context exhaustion
- Produce a runtime budget envelope such as:
  - healthy
  - constrained
  - high-risk
  - compaction-required

## Deliverables
- A token forecasting module with budget-envelope output.
- Runtime attachment of the forecast to the active objective / workflow.
- Trace visibility showing why Elma considers a workflow affordable or risky.
- Tests covering low-budget, medium-budget, and near-overflow scenarios.

## Design Notes
- Forecasting should be conservative and honest, not falsely precise.
- This task should remain model-friendly:
  - narrow inputs
  - low bookkeeping overhead
  - no giant accounting traces injected into prompts
- Budget envelopes should become orchestration signals, not just developer diagnostics.

## Acceptance Criteria
- Elma can estimate whether a complex task is likely to fit within the remaining context budget before deep execution begins.
- Retry ladders and decomposition depth can be evaluated against a budget envelope.
- Forecasts are grounded in real runtime telemetry rather than arbitrary fixed numbers.
- CLI traces and session artifacts make the forecast understandable and debuggable.

