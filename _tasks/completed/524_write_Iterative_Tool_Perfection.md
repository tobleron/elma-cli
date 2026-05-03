# Task 524: write - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `write` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `write` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/write.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Create a new file output/summary.txt with the text "Analysis complete." in it, then overwrite it with a multi-line report containing Rust code blocks and structured sections, then attempt to write to a path outside the workspace like ~/Desktop/test.txt and observe the error, then write a JSON config file, then write a Python script and execute it with shell to verify correctness.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Content-First Formatting
Instruct model to compose the full file content in its reasoning block BEFORE calling write.

### Approach B: Step Decomposition: Write-Then-Read Verification
Always read back the file immediately after writing to confirm correctness.

### Approach C: Safety Gate Tuning: Workspace Scope
Include the allowed root path in error messages.

### Approach D: Token Budget: Large Content Truncation
Test with 2000-char and 5000-char files to verify truncation behavior.

## Success Criteria
- [ ] The model calls `write` successfully in every scenario from the stress test
- [ ] No shell fallback when `write` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/508_write.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
