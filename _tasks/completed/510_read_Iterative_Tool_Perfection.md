# Task 510: read - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `read` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `read` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/read.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Read the file Cargo.toml to see the dependencies, then read only lines 10-30 of src/main.rs, then read both README.md and AGENTS.md simultaneously, then read all markdown files in the docs/ directory using a glob pattern, then read src/types_core.rs with a snippet of only lines 40-45, then read a non-existent file and handle the error gracefully, then read a binary file like target/debug/elma-cli and see what happens.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Path Prefix Enforcement
Add: 'When reading files, ALWAYS use a path relative to workspace root. NEVER construct paths by joining random directory names.'

### Approach B: Step Decomposition: Verify-Target-Then-Read
Before reading, use exists or glob to confirm the file exists first.

### Approach C: Signal Novelty: Path Diversity Reward
Modify tool signal to hash only the file path (not content) when checking stagnation.

### Approach D: Schema Simplification: Remove snippet Complexity
If model struggles with snippet/offset/limit, register simplified read accepting only paths and returning first 2000 chars.

## Success Criteria
- [ ] The model calls `read` successfully in every scenario from the stress test
- [ ] No shell fallback when `read` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/507_read.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
