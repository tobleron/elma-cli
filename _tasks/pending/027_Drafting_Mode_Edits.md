# Task 036: Drafting Mode for Code Edits

## Context
`execution_steps_edit.rs` currently applies edits directly. This is high-risk without verification.

## Objective
Implement a "Draft -> Review -> Apply" cycle for code edits:
- First, generate a "Draft" or "Diff" of the change.
- Run a `verification` step (e.g., `cargo check` or `cargo fmt`) on the draft.
- Only apply the edit if the verification passes or if a `critic` approves the risk.

## Success Criteria
- Zero "broken build" edits in autonomous mode.
- Higher reliability for complex refactoring tasks.
