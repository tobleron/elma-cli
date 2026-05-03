# Task 506: exists - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `exists` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `exists` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/exists.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Check if Cargo.toml exists (true), then nonexistent_file.txt (false), then src/ directory, then create file, check exists=true, trash it, check exists=false, then check multiple paths in sequence, then check /etc/hosts absolute path.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Boolean Result Expectation
Tell model: exists returns simple true/false. Use stat for metadata.

### Approach B: Step Decomposition: Exists-Before-Action
Before EVERY read, edit, copy, move, trash, or stat, first call exists.

### Approach C: Null Pattern: Non-Existent Path Error Handling
When false, double-check spelling and use glob to find similar files.

### Approach D: Directory vs File Check
Use separate exists checks for directories vs files.

## Success Criteria
- [ ] The model calls `exists` successfully in every scenario from the stress test
- [ ] No shell fallback when `exists` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/522_exists.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
