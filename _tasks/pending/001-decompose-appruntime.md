# Task: Decompose AppRuntime God Object

## Status
- **Priority:** High
- **Assignee:** jules
- **Status:** Pending

## Objective
Split the `AppRuntime` struct in `src/app.rs` into smaller, cohesive context objects to reduce tight coupling across the codebase.

## Requirements
- Identify logical sub-components (e.g., `SessionContext`, `LlmClient`, `WorkspaceConfig`).
- Refactor functions that take `&AppRuntime` to take only the necessary sub-contexts.
- **Accuracy Guarantee:** Ensure 100% parity with existing functionality. No methods or fields should be lost.
- Ensure `AppRuntime` remains as a top-level container that composes these sub-contexts for backward compatibility where necessary during the transition.

## Verification
- `cargo check` must pass.
- All existing tests must pass.
