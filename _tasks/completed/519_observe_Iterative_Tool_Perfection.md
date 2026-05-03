# Task 519: observe - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `observe` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `observe` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/observe.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Get metadata about Cargo.toml: type, size, permissions, modified time. Then observe the src/ directory and get child count. Then observe a symlink and confirm the target is shown. Then observe a non-existent file. Then observe both a file and a directory to compare metadata. Then use observe before reading a file to confirm it exists.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Multi-Field Collection
Ask model to collect ALL fields on every observe call.

### Approach B: Step Decomposition: Observe-Before-Action
Train model to always observe a file before reading or editing it.

### Approach C: Symlink Handling: Explicit Check
Model should observe first to check IS a symlink, then decide to follow it.

### Approach D: Bulk Observation: Directory + Multiple Files
Model should observe directory then individual files within it.

## Success Criteria
- [ ] The model calls `observe` successfully in every scenario from the stress test
- [ ] No shell fallback when `observe` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/511_observe.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
