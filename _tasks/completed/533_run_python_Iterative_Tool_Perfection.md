# Task 533: run_python - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `run_python` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `run_python` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/run_python.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Execute: print("hello from python"), then multi-line Fibonacci, then code using sys module for argv, then code with syntax error (verify message), then code exceeding timeout (verify handling), then write a script file and execute via run_python, then import non-existent module (verify error).
```

## Suggested Approaches

### Approach A: Prompt Engineering: Code-First Convention
Model should compose code in reasoning first, then paste into code param.

### Approach B: Step Decomposition: Write-Execute-Verify
For complex scripts: write to file, execute, verify output.

### Approach C: Shell-Fallback Guard: Block python3 -c
Block shell commands containing python3 to force native tool.

### Approach D: Timeout Testing
Test 5, 30, 60 second timeouts. Verify graceful handling.

## Success Criteria
- [ ] The model calls `run_python` successfully in every scenario from the stress test
- [ ] No shell fallback when `run_python` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/530_run_python.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
