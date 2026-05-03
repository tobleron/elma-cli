# Task 522: touch - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `touch` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `touch` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/touch.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Create empty file new_file.txt, then update timestamp of Cargo.toml, then touch a file in a non-existent directory (verify error), then touch and verify size 0 with stat, then touch inside a directory created moments earlier.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Zero-Content File Creation
Emphasize: touch creates an EMPTY file. Use write for content.

### Approach B: Step Decomposition: Touch-Stat-Delete
Create with touch, verify size 0 with stat, then trash.

### Approach C: Timestamp Update vs Create
Model should know touch on existing file updates its timestamp.

## Success Criteria
- [ ] The model calls `touch` successfully in every scenario from the stress test
- [ ] No shell fallback when `touch` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/520_touch.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
