# Task 033: State-Aware Guardrails (Context Drift)

## Context
Complex workflows can "drift" from the original goal as the OODA loop iterates.

## Objective
Implement a "Context Drift" monitor:
- At each OODA step, compare the current `Program` and `StepResult` against the original `Goal` (Level 1).
- Trigger a mandatory `Refinement` phase if the agent is diverging into irrelevant sub-tasks.

## Success Criteria
- Increased goal-alignment for complex, multi-step tasks.
- Fewer "rabbit holes" in autonomous execution.
