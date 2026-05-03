# Task 516: summary - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `summary` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `summary` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/summary.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
After several turns, ask for a summary of accomplishments: tools used, files read, decisions made. Then continue working and request an updated summary to confirm it reflects new work. Verify summary does not truncate important context and captures both user requests and assistant actions.
```

## Suggested Approaches

### Approach A: Prompt Engineering: When to Summarize
Only call summary when asked or when conversation exceeds 10 turns.

### Approach B: Content Preservation
After summary, verify continuity score has not dropped below 0.8.

### Approach C: Format Consistency
Summary output format should be consistent across calls.

## Success Criteria
- [ ] The model calls `summary` successfully in every scenario from the stress test
- [ ] No shell fallback when `summary` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/525_summary.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
