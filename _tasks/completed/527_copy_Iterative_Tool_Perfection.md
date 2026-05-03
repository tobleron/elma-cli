# Task 527: copy - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `copy` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `copy` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/copy.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Copy Cargo.toml to Cargo.toml.bak, then copy src/main.rs to src/main.rs.bak, then copy the entire src/ directory to src_backup/, then copy a file to a non-existent directory (verify error), then copy into a directory, then copy a symlink and confirm preserve-or-follow behavior.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Source-Then-Dest Ordering
Remind: source path first, destination path second.

### Approach B: Step Decomposition: Exists-Copy-Exists Pattern
Check exists on source before, destination after.

### Approach C: Directory Copy Depth Awareness
Model should understand copy handles recursive contents.

### Approach D: Overwrite Protection: Confirm Before Overwrite
If destination exists, model should inform user.

## Success Criteria
- [ ] The model calls `copy` successfully in every scenario from the stress test
- [ ] No shell fallback when `copy` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/517_copy.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
