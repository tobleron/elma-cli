# Task 002: Add Editing Formula Workflow

## Objective
Ensure Elma has a dedicated formula for safe file editing tasks instead of forcing editing requests through generic shell execution or vague multi-step improvisation.

## Context
Elma currently handles inspection, planning, decisions, and shell execution, but editing needs its own explicit reasoning path. Editing tasks should be able to:
- inspect the relevant file first
- understand the requested change
- apply the change in a controlled way
- verify the result
- explain what changed

This formula must support both small surgical edits and multi-step edit flows while staying compatible with Elma's reasoning-first architecture.

## Work Items
- [ ] Define a dedicated editing formula pattern, for example `inspect_edit_verify_reply`.
- [ ] Decide how the orchestrator should prefer the editing formula when the user asks to create, modify, patch, update, or rewrite local files.
- [ ] Make sure the editing workflow includes:
  - pre-edit inspection
  - edit execution
  - post-edit verification
  - concise final response
- [ ] Ensure editing requests can distinguish between:
  - create new file
  - modify existing file
  - targeted patch
  - broader refactor request
- [ ] Integrate the editing formula into formula selection, orchestration, evaluation, and calibration coverage.

## Acceptance Criteria
- Elma has a named editing formula or equivalent first-class editing workflow.
- Editing requests no longer rely on ad hoc generic shell handling alone.
- The workflow verifies edits after applying them.
- The final response clearly states what changed and whether verification succeeded.

## Verification
- `cargo build`
- `cargo test`
- live probes for:
  - edit existing file
  - create new file
  - small patch request
  - failed edit requiring correction
