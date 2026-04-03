# Task 091: Comprehensive Per-Model Tuning Lifecycle

## Priority
**P2 - LATE-STAGE PRODUCTION READINESS**

## Objective
Create a full tuning lifecycle that can spend as much time as needed to calibrate a model for Elma-cli, iteratively testing and adjusting runtime behavior until the model reaches the best achievable compliance with Elma's expected task behavior.

## Why This Exists
Some models will not naturally satisfy Elma's workflow contracts. A production-grade Elma-cli should be able to run an exhaustive tuning cycle against a target model and discover:
- what parameter ranges improve reliability
- what role-specific settings reduce failure rates
- where bounded prompt-level overrides help
- where no amount of tuning is sufficient

This task operationalizes the mapping system from Task 090 into a real end-to-end tuning lifecycle.

## Scope
- Define a full tuning loop for a target model:
  1. establish baseline benchmark behavior
  2. detect failure classes
  3. choose allowable mitigation dimensions
  4. run iterative tuning experiments
  5. compare against canonical expected outcomes
  6. keep best-known profile state
  7. stop when improvement plateaus or full compliance is reached
- Include task classes such as:
  - routing and classification
  - JSON reliability
  - shell/read/search/edit reliability
  - summarization and presentation
  - long-workflow orchestration
  - context pressure behavior
- Allow comprehensive runtime tuning, including prompt overrides only as a last resort and only within Elma's philosophy constraints.

## Deliverables
- A repeatable per-model tuning lifecycle.
- Experiment history and best-known configuration persistence.
- Support for exhaustive long-running tuning sessions.
- Guardrails preventing philosophy-breaking “fixes” from being accepted as wins.

## Design Notes
- This task should assume hours-long tuning runs are acceptable.
- The objective is not speed; it is best-achievable behavioral alignment.
- The lifecycle should preserve a clean distinction between:
  - harmless operational tuning
  - philosophy-breaking prompt overfitting
- The output should be usable by end users without forcing them to understand the tuning internals.

## Acceptance Criteria
- A new model can be put through a comprehensive Elma tuning lifecycle.
- The system can converge toward best-known settings per task family and per role.
- Best-known tuned states are reproducible and inspectable.
- Philosophy-breaking prompt regressions are rejected even if they superficially pass some tests.

