# Task: Refactor Monolithic Orchestration Loops

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending

## Objective
Refactor `run_chat_loop` and `run_tool_loop` into structured state machines.

## Requirements
- Extract discrete phases (Model Turn, Tool Execution, UI Sync) into separate modules/structs.
- Reduce individual function lengths to under 500 lines.
- Maintain existing error handling and diagnostic logging behavior.

## Verification
- `cargo check` must pass.
- Integration tests for tool-calling must pass.
