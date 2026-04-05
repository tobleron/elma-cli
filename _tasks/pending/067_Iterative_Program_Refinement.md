# Task 067: Iterative Program Refinement

## Priority
**P1 - RELIABILITY CORE (Tier A)**
**Was Blocked by:** P0-1, P0-2, P0-3 — **NOW UNBLOCKED** (P0 pillars substantially complete per Task 058)

## Status
**PENDING** — Ready to start

## Renumbering Note
- **Old Number:** Task 013
- **New Number:** Task 012 (per REPRIORITIZED_ROADMAP.md)
- **Reason:** Reprioritized as P0-4.1, must complete after foundational pillars

---

## Problem
Elma produces one program and executes it without iteration. Real autonomous agents iterate: plan → execute → observe → revise.

## Evidence
From "summarize AGENTS.md" session:
- Model produces program with empty `content` field in edit step
- No mechanism to detect incomplete execution and revise
- Single-shot execution model

## Goal
Implement a refinement loop that lets the model revise its program based on execution feedback.

## Additional Reliability Gaps Now Confirmed
- Refinement/recovery can still preserve the wrong workflow shape after hard evidence of failure.
- Broken shell-repair loops can recur with nearly identical command structure instead of switching to a better evidence strategy.
- Recovery may keep semi-interactive shapes such as “ask the user to choose” even when the original objective explicitly asked Elma to choose autonomously.
- Downstream steps can continue after a failed evidence step using artifacts that were never truly grounded.

## Implementation Steps

1. **Create new module** `src/refinement.rs`:
   ```rust
   pub struct RefinementContext {
       pub original_objective: String,
       pub step_results: Vec<StepResult>,
       pub failures: Vec<ExecutionFailure>,
       pub iteration: u32,
   }

   pub async fn refine_program(
       client: &reqwest::Client,

## Scope Additions
- Make refinement explicitly failure-type-aware:
  - unsupported command flag
  - empty evidence
  - semantic-guard rejection
  - placeholder mismatch
  - false-success downstream artifact
- Require refinement to change strategy, not just wording, when the prior step failed for the same root cause.
- Prevent recovery from introducing a user-choice step when the user asked Elma to make the choice.
- Block downstream select/edit/verify steps from treating failed evidence as usable input.

## Acceptance Additions
- A failed shell step with semantically rejected repair cannot feed a selector or compactor as if it were successful evidence.
- Refinement changes the evidence strategy after repeated command-shape failure instead of replaying the same broken pattern.
- Autonomous objectives remain autonomous through recovery; refinement does not silently convert them into user-interactive workflows.

## Additional Session Evidence
- Session `s_1775235404_589084000` exposed this directly:
  - initial bad `rg --no-color` command failed
  - repair stayed on the same bad command family
  - workflow still attempted selection after failed evidence
  - generated steps drifted into “obtain the user's selection” even though the user asked Elma to choose
