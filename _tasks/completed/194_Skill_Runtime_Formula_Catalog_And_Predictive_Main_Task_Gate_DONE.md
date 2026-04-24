# Task 194: Skill Runtime, Formula Catalog, And Predictive Main-Task Gate

## Priority
P0

## Objective
Introduce a built-in skill subsystem that classifies each request as either a simple request or a persisted main task, then selects a bounded formula from a curated catalog.

## Why This Exists
The old single-skill model is too weak for the new product direction. Elma needs to decide not only which skill is relevant, but whether the request deserves persisted task treatment and whether multiple skill stages are required.

## Required Behavior
- Every request must produce an `ExecutionPlanSelection`.
- The selector must decide both:
  - `RequestClass = Simple | MainTask`
  - a bounded `SkillFormulaId`
- Formula selection must come from a curated catalog, not arbitrary model-generated stage graphs.
- Formulas are sequential and capped at 3 stages in v1.

## Required Types
- `SkillId`
- `RequestClass`
- `SkillFormulaId`
- `FormulaStage`
- `SkillFormula`
- `MainTaskGateVerdict`
- `ExecutionPlanSelection`

## Built-In Skills For V1
- `general`
- `task_steward`
- `repo_explorer`
- `document_reader`
- `file_scout`

## Built-In Formulas For V1
- `general_reply`
- `repo_explore_then_reply`
- `document_read_then_reply`
- `file_scout_then_reply`
- `file_scout_document_reply`
- `project_task_steward`

## Gate Rules
A request becomes `MainTask` when any of the following are predicted:
- 3 or more tool calls
- multiple dependent stages
- cross-file or cross-root evidence gathering
- resume value after interruption
- explicit planning/task/audit intent
- multi-skill execution is clearly needed

A request remains `Simple` when it is bounded, one-stage or directly answerable, and has no meaningful resume value.

## Implementation Requirements
- Keep selection model-driven; do not use hardcoded keyword routing.
- Fallback must be deterministic: `Simple + general_reply`.
- `ExecutionPlanSelection` must contain enough information to:
  - render in the UI,
  - seed runtime task persistence,
  - build system prompt context.
- Each built-in skill must have a code-authoritative `SkillPlaybook` or equivalent instruction contract, not just a one-line directive. The playbook must define:
  - what the skill is trying to achieve,
  - what evidence/tool strategy it prefers,
  - how it budgets work,
  - when it should process in full versus stage work,
  - when it may ask the user for clarification,
  - what a successful output looks like.
- Keep formula catalog code-authoritative.
- Add a short human-readable reason to the verdict for trace and UI use.

## Integration Points
- request entry point in `app_chat_loop.rs`
- system prompt shaping in orchestration/tool loop
- runtime task creation in Task 195
- `/skills` rendering in Task 199

## Acceptance Criteria
- Every request gets an execution-plan selection.
- Simple requests do not create runtime task state.
- Main tasks create runtime task state before execution starts.
- Formula selection is visible to the user and grounded in a bounded catalog.
- The system still has a safe fallback path if the selector call fails.

## Playbook Requirement For This Task
- Add the cross-skill playbook contract that later skills can plug into.
- The selector/runtime must be able to reference these playbooks when building execution context.
- Playbooks must be code-authoritative and stable, not ad hoc prompt fragments scattered across call sites.

## Required Tests
- fallback returns `Simple + general_reply`
- known repo analysis request yields a repo-oriented formula
- document search request can yield `file_scout_document_reply`
- project planning request can yield `project_task_steward`
- no branch in the selector depends on string keyword triggers in code
