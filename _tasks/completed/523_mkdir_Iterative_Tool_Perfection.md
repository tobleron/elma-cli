# Task 523: mkdir - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `mkdir` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `mkdir` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/mkdir.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Create a single directory test_dir, then create nested dirs test/a/b/c in one call, then create a directory that already exists (verify no error), then create inside target/ and see if allowed, then create dir and immediately create a file inside it.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Path Parameter Clarity
Mkdir takes a single path. Model sometimes tries to pass multiple paths.

### Approach B: Step Decomposition: Mkdir-Then-Exists
Create dir, verify with exists, then create file inside.

### Approach C: Nested Directory Awareness
Model should know mkdir with a/b/c creates ALL intermediates.

### Approach D: Error Recovery: Already Exists
If already exists, treat as soft error and proceed.

## Success Criteria
- [ ] The model calls `mkdir` successfully in every scenario from the stress test
- [ ] No shell fallback when `mkdir` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/519_mkdir.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
