# Task 509: file_size - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `file_size` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `file_size` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/file_size.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Get size of Cargo.toml (human-readable), then sizes of Cargo.toml, src/main.rs, target/debug/elma-cli together, then size of empty file after touch, then size of a directory, then size of non-existent file (verify error).
```

## Suggested Approaches

### Approach A: Prompt Engineering: Unit Preference
Tell model: file_size returns human-readable sizes. Use stat for bytes.

### Approach B: Step Decomposition: Size-Before-Read
Before reading any file, check its size. If >100KB, only read a snippet.

### Approach C: Comparison: Multiple File Sizes
Check multiple files to identify large vs small.

### Approach D: Zero-Size File Detection
After touch, use file_size to confirm 0 bytes. After write, confirm changed.

## Success Criteria
- [ ] The model calls `file_size` successfully in every scenario from the stress test
- [ ] No shell fallback when `file_size` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/523_file_size.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
