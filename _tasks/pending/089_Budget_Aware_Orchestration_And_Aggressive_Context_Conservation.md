# Task 089: Budget-Aware Orchestration And Aggressive Context Conservation

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
**Depends on:** Tasks 087 + 088 (Telemetry + Forecasting)

## Objective
Make token budget a live orchestration input, so Elma can proactively conserve context on small local models by choosing smaller workflows, compressing narrative aggressively when necessary, and protecting the main objective instead of spending the context window carelessly.

## Why This Exists
This is the feature that makes Elma special for low-end local setups.

The goal is not only to summarize when context is already large. The goal is to let Elma actively steer the whole objective-achievement loop around token limits:
- choose cheaper valid workflow shapes when the budget is tight
- compact evidence and conversation context earlier when forecasts say trouble is coming
- preserve the main plan and user objective while aggressively reducing low-value token load
- keep quality as high as possible even on 1B/3B-class models with modest context windows

## Scope
- Consume telemetry from Task 087 and forecasts from Task 088.
- Introduce a budget-aware policy layer for orchestration decisions such as:
  - decomposition depth
  - retry allowances
  - evidence retention vs compaction
  - reviewer/refinement usage
  - narrative compression aggressiveness
  - when to roll summaries forward
  - when to refuse expensive detours that threaten the main objective
- Define conservation modes, for example:
  - normal
  - conservative
  - aggressive conservation
  - critical budget preservation
- Ensure conservation logic remains truth-preserving and objective-preserving.

## Deliverables
- A budget-aware orchestration policy module.
- Integration points with:
  - execution ladder
  - refinement / retry logic
  - evidence compaction
  - rolling conversation summary
  - result presentation safety
- Session traces showing when conservation mode activates and why.
- Stress tests for tiny-budget conditions.

## Design Notes
- This task must not become a brittle rule dump.
- Prefer principle-based thresholds and budget envelopes over keyword heuristics.
- Conservation should reduce waste, not reduce truthfulness.
- The main objective must stay intact; low-value context should be sacrificed first.
- This task is the bridge between baseline reliability and premium local-model efficiency.

## Acceptance Criteria
- On tight budgets, Elma changes behavior before context overflow becomes likely.
- Elma can preserve the main objective while shedding low-value context aggressively.
- Multi-step workflows on small local models survive longer with better answer quality than before.
- Budget-driven conservation is visible, explainable, and testable in real CLI sessions.

