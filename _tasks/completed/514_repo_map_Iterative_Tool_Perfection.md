# Task 514: repo_map - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `repo_map` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `repo_map` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/repo_map.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Build a repository map with token_budget=2000 and max_files=50, then with token_budget=500 and max_files=10 (small), then with token_budget=5000 and max_files=200 (large). Compare outputs: smaller budget should produce concise map, larger covers more files. Note the tokens_used field.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Budget Parameter Guidance
Tell model: '500 = ~10 files, 2000 = ~50 files, 5000 = ~200 files.'

### Approach B: Step Decomposition: Map-Then-Navigate
Build map first, then explore 2-3 interesting directories with ls or read.

### Approach C: Shell-Fallback Guard: Block tree Command
Block shell command tree to force native repo_map usage.

### Approach D: Budget Scaling Test
Test 200, 2000, 20000 budgets. Verify output scales appropriately.

## Success Criteria
- [ ] The model calls `repo_map` successfully in every scenario from the stress test
- [ ] No shell fallback when `repo_map` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/528_repo_map.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
