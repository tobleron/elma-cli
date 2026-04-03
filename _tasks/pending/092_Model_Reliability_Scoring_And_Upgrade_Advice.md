# Task 092: Model Reliability Scoring And Upgrade Advice

## Priority
**P2 - LATE-STAGE USER TRUST**

## Objective
After the full tuning lifecycle is exhausted, give each model a clear Elma reliability score and practical guidance about what the user should expect from that model, including whether more context or a stronger model is likely needed.

## Why This Exists
Not every model will become a great Elma model, even after extensive tuning.

Elma-cli should be honest and premium in how it communicates that reality:
- how reliable the tuned model is
- what classes of tasks it handles well
- what classes remain risky
- whether the user would benefit from:
  - a larger context window
  - a stronger model
  - different runtime limits
  - reduced ambition on very heavy tasks

## Scope
- Define a reliability scoring system that reflects:
  - benchmark pass rates
  - failure severity
  - residual drift after tuning
  - consistency across retries and long tasks
  - context-pressure stability
- Produce model advice such as:
  - recommended for general Elma use
  - good for light tasks only
  - reliable only with conservative settings
  - not recommended for full autonomy
- Where endpoint metadata allows, include upgrade suggestions:
  - larger context window
  - bigger parameter count
  - higher JSON reliability model
  - safer local-model profile preset

## Deliverables
- A model reliability score and report format.
- A user-facing interpretation layer for the score.
- Upgrade / recommendation advice tied to actual tuning outcomes.
- Clear reporting of residual risks after full tuning.

## Acceptance Criteria
- Every tuned model receives a final reliability assessment.
- The score reflects tuned behavior, not only raw baseline behavior.
- Users receive actionable advice when a model is not strong enough for certain Elma workloads.
- Reliability reporting remains honest, grounded, and easy to understand.

