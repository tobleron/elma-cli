# Task 192: Stuck Detection And Stop Policy

## Priority
P0

## Objective
Turn the existing loop guards into a unified stop policy with explicit stop reasons, stage-aware budgets, repeated-failure detection, repeated-no-new-evidence detection, and user-visible explanations.

## Why This Exists
Elma currently has partial guardrails such as max iterations and stagnation thresholds, but they are not yet a first-class execution policy. The system can still feel like it is "trying again" without explaining why it stopped or what narrower next step is appropriate.

This task makes stop behavior explicit, predictable, and visible.

## Required Behavior
- Every execution path must finish with one of:
  - success,
  - stopped with a specific `StopReason`,
  - interrupted by the user.
- Main-task formulas must track both:
  - per-stage budget,
  - whole-task budget.
- When Elma stops, it must emit:
  - the reason,
  - the stage that stopped,
  - one narrower suggested next step.
- Stop policy must work for both:
  - simple requests,
  - main tasks with persisted session state.

## Required Types
- `StopReason`
  - `StageBudgetExceeded`
  - `TaskBudgetExceeded`
  - `RepeatedToolFailure`
  - `RepeatedNoNewEvidence`
  - `RepeatedSameCommand`
  - `RepeatedSameConclusion`
  - `WallClockExceeded`
  - `ModelProgressStalled`
  - `UserInterrupted`
- `StageBudget`
  - `max_tool_calls`
  - `max_iterations`
  - `max_repeated_failures`
  - `max_stagnation_cycles`
  - `max_wall_clock_s`
- `StopOutcome`
  - `reason`
  - `stage_index`
  - `stage_skill`
  - `summary`
  - `next_step_hint`

## Implementation Requirements
- Absorb the current `MAX_TOOL_ITERATIONS` and stagnation logic into shared stop-policy logic rather than leaving split policy in `tool_loop.rs` only.
- Do not duplicate budget logic across each skill; the policy must be centrally enforced.
- Add hooks so formula stages can provide their own budgets without owning the stop algorithm.
- Persist stop outcome into `RuntimeTaskRecord.stop_reason` for main tasks.
- Surface stop outcome in:
  - transcript,
  - trace,
  - final answer text for user-visible stoppages.

## Detection Rules
- `RepeatedToolFailure`: same tool family failing with equivalent error class more than the configured threshold.
- `RepeatedNoNewEvidence`: multiple iterations with no new file, no new command output, and no new grounded claim.
- `RepeatedSameCommand`: same shell command or equivalent normalized command repeated without changed scope.
- `RepeatedSameConclusion`: assistant keeps restating effectively the same summary without advancing evidence.
- `ModelProgressStalled`: model emits activity but no new actionable tool call or grounded answer within budget.

## Integration Points
- `tool_loop.rs`
- orchestration loop / retry path
- runtime task persistence
- UI notification surface
- final answer resolver

## Acceptance Criteria
- Elma never silently loops beyond configured stage or task budgets.
- Stop reason is visible to the user and persisted for main tasks.
- Each stop reason maps to a narrower suggested next step.
- Existing loop guard behavior is not duplicated in multiple modules.

## Required Tests
- repeated identical failing tool call triggers `RepeatedToolFailure`
- repeated no-op evidence loop triggers `RepeatedNoNewEvidence`
- stage budget stops before whole-task budget when the stage is the bottleneck
- whole-task budget stops after multiple stages even if each stage is locally valid
- user interrupt is preserved distinctly from budget exhaustion
- final answer includes stop explanation instead of a generic failure sentence
