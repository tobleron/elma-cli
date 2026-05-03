# Task 508: stat - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `stat` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `stat` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/stat.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Get file type (regular, directory, symlink) of Cargo.toml, src/, and a symlink. Then get permissions of target/debug/elma-cli (confirm executable). Then stat a non-existent path. Then stat both a file and a directory showing all fields: type, permissions, size, modified time. Verify output is machine-parseable.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Field Completeness
Instruct model to always specify which fields it wants.

### Approach B: Step Decomposition: Size Check Before Read
Before reading large files, stat first to check size.

### Approach C: Permission Check Before Shell
Before shell, stat binary to check it is executable.

### Approach D: Batch Stat: Multiple Files
If stats on multiple files needed, batch them.

## Success Criteria
- [ ] The model calls `stat` successfully in every scenario from the stress test
- [ ] No shell fallback when `stat` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/516_stat.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
