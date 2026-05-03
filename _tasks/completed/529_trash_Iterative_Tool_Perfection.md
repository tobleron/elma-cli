# Task 529: trash - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `trash` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `trash` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/trash.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Move test_dir to trash, then trash single file test.txt, then trash non-existent file (verify error), then trash a directory with files (confirm all trashed), then attempt to trash outside workspace (verify block), then trash and check exists returns false for the path.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Irreversible Action Warning
Add warning that trashing is irreversible in the current session.

### Approach B: Step Decomposition: Exists-Trash-Exists
Check exists before, check gone after.

### Approach C: Safety Gate: Workspace Paths Only
Model should ONLY trash paths within the workspace.

## Success Criteria
- [ ] The model calls `trash` successfully in every scenario from the stress test
- [ ] No shell fallback when `trash` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/521_trash.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
