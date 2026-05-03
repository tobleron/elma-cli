# Task 528: move - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `move` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `move` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/move.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Rename Cargo.toml.bak to Cargo.toml.old, then move it into subdirectory test_move/, then rename test_move/ to renamed_move/, then move a file outside workspace, then move a non-existent file, then move a directory with contents and verify all moved, then attempt to move a file onto itself.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Source-Then-Dest Ordering
Source first, dest second. The model frequently reverses these.

### Approach B: Step Decomposition: Exists-Move-Exists Pattern
Check source before, check both paths after.

### Approach C: Path Normalization: Conflicting Names
The tool is registered as r#move due to Rust keyword. Ensure model calls it correctly.

### Approach D: Cross-Filesystem Move Handling
Cross-filesystem moves should copy+delete. Verify model handles this.

## Success Criteria
- [ ] The model calls `move` successfully in every scenario from the stress test
- [ ] No shell fallback when `move` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/518_move.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
