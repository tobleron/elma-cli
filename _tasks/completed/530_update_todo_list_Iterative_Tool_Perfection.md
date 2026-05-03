# Task 530: update_todo_list - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `update_todo_list` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `update_todo_list` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/todo.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Create a todo task to review AGENTS.md with pending status, then update it to in_progress, then create a second task to fix a bug with high priority, then mark the first as completed, then list all tasks and verify status changes. Then create a task with very long description to test truncation, then create and complete a task in the same turn.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Task Status Lifecycle
pending -> in_progress -> completed lifecycle.

### Approach B: Step Decomposition: Create-Update-Complete
For new work: create, move to in_progress, do work, mark completed.

### Approach C: Task Number Awareness
Model should use next sequential number from _tasks/ directory.

### Approach D: Task Cross-Referencing
Reference task numbers for traceability.

## Success Criteria
- [ ] The model calls `update_todo_list` successfully in every scenario from the stress test
- [ ] No shell fallback when `update_todo_list` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/526_update_todo_list.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
