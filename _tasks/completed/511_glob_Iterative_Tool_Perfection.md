# Task 511: glob - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `glob` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `glob` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/glob.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Find all Rust source files (*.rs) in the src/ directory recursively, then find all Cargo.toml files at any depth, then find all files matching *.md but exclude files in the sessions/ directory, then find all files matching test_*.rs in any subdirectory, then find all files matching *.txt with max depth 2, then combine glob with read to read all found Cargo.toml files.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Schema Pattern Examples
Include concrete examples: glob(**/*.rs) -> [src/main.rs, ...] in the description.

### Approach B: Step Decomposition: Glob-Then-Read Pipeline
Train model to glob first, then iterate over results with read.

### Approach C: Exclusion Integration: Ignoring sessions/ and target/
Verify model uses exclude_patterns to skip sessions/ and target/.

### Approach D: Parallelism: Multi-Glob Coverage
Model should issue multiple glob calls (one per pattern) rather than one complex one.

## Success Criteria
- [ ] The model calls `glob` successfully in every scenario from the stress test
- [ ] No shell fallback when `glob` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/509_glob.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
