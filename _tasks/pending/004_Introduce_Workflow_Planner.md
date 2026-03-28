# Task 004: Introduce Workflow Planner

## Objective
Replace the current `complexity -> scope -> formula` chain with a single model-driven `workflow_planner` that produces a tighter, lower-latency planning prior before orchestration.

## Context
Elma currently makes several separate planning calls before building a program:
- complexity assessment
- scope building
- formula selection

That creates latency, opportunities for drift, and inconsistent internal priors. Recent failures show that even when routing is correct, the planning chain can still produce a weak or contradictory program.

This task is intentionally smaller than a full OODA rewrite. It should improve planning quality first without replacing the entire execution model.

## Work Items
- [ ] Define a new `workflow_planner` intel unit config.
- [ ] Design a structured output that includes at minimum:
  - objective
  - complexity
  - scope
  - preferred formula
  - risk
  - whether evidence is required
- [ ] Update orchestration input-building so the orchestrator consumes the unified planner output instead of three separate priors.
- [ ] Keep the existing complexity/scope/formula units available as fallback during migration.
- [ ] Ensure formula-memory candidates and recent verification signals can be included in planner context.
- [ ] Measure whether the new planner reduces unnecessary model calls and weak program drift.

## Acceptance Criteria
- A single planner output can replace the separate complexity/scope/formula chain for normal workflow turns.
- Planning latency is reduced or at least not worsened materially.
- The planner improves consistency on known weak scenarios such as:
  - select-then-show tasks
  - summarize project/codebase tasks
  - cleanup-style inspection and decision tasks
- The old chain can still be used as fallback until the planner is stable.

## Verification
- `cargo build`
- `cargo test`
- live comparisons on representative prompts before/after planner integration
- compare number of model calls and quality of generated programs
