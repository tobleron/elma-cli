# Task 507: ls - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `ls` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `ls` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/ls.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
List the contents of the current directory, then list src/ with depth 2, then list docs/ with one file per line showing sizes and modified times, then list only .rs files in src/, then list with sorting by size (largest first), then list a directory with 1000+ items and verify truncation, then list a non-existent path and handle the error.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Depth Awareness Cue
Add: 'Depth 1 = immediate children. Depth 2 = grandchildren. Omit depth for top-level.' The 4B model often omits depth, producing overwhelming 158K-item listings.

### Approach B: Step Decomposition: Depth Escalation
Always start with ls depth 1 for overview, THEN use deeper only for directories of interest.

### Approach C: Default Override: Safety Cap
Cap ls output at 200 lines regardless of how many items exist.

### Approach D: Output Truncation Awareness
Last session showed ls listing 158K items in artifacts/ which flooded the TUI. Add hard cap of 200 lines to ls output.

## Success Criteria
- [ ] The model calls `ls` successfully in every scenario from the stress test
- [ ] No shell fallback when `ls` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/510_ls.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
