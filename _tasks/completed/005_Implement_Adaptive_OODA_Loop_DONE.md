# Task 005: Implement Adaptive OODA Loop

## Objective
Evolve Elma from executing a fixed `Program` into a bounded adaptive loop that can observe outcomes, pivot intelligently, and continue only when doing so improves the chance of satisfying the user request.

## Context
This is the long-term autonomy task. It should come after stronger verification and a unified workflow planner exist.

The current system can recover in limited ways, but it still tends to:
- repair into weak programs
- fail to pivot when a step partially succeeds
- over-commit to the original plan even when evidence says it should change direction

## Technical Details
- [ ] Define an `AgentPlan` or equivalent adaptive state in `src/types.rs`.
- [ ] Implement a bounded `run_autonomous_loop` in `src/orchestration.rs` that iterates:
  1. Observe current step results and verification outputs.
  2. Orient around the current objective, scope, and failure mode.
  3. Decide the next best action or small sub-plan.
  4. Act by executing only that next action.
- [ ] Use structured observations and verification outputs as the primary control input.
- [ ] Treat `reasoning_content` as optional supplemental audit/context only, not as a mandatory control dependency.
- [ ] Add explicit stop conditions:
  - success achieved
  - safe clarification required
  - repeated recovery failure
  - maximum step count reached
- [ ] Ensure the loop can hand off to plan/masterplan/edit flows without duplicating their logic.

## Acceptance Criteria
- Elma can add or replace a step after unexpected failure instead of rerunning a stale workflow.
- The loop terminates cleanly under success, clarification, failure, and max-step conditions.
- Recovery quality improves on real failing scenarios, not just synthetic tests.
- The new loop does not require reasoning tokens to function correctly.

## Verification
- `cargo build`
- `cargo test`
- live scenario where a shell step fails and Elma must pivot
- live scenario where evidence shows the current plan is incomplete
- confirm max-step safety prevents infinite loops
