# Task 534: run_node - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `run_node` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `run_node` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/run_node.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Execute console.log("hello from node"), then multi-line prime calculator, then code with syntax error (verify message), then code using process.argv, then code with timeout (verify handling), then read a file with fs.readFileSync, then require non-existent module (verify error).
```

## Suggested Approaches

### Approach A: Prompt Engineering: Code-First Convention
Compose in reasoning first, then pass to code parameter.

### Approach B: Prerequisite Checking
Model should check if node is available before calling run_node.

### Approach C: Shell-Fallback Guard: Block node in Shell
Block shell commands containing node to force native tool.

### Approach D: Error Handling: Module Not Found
When module not found, install it or use alternative approach.

## Success Criteria
- [ ] The model calls `run_node` successfully in every scenario from the stress test
- [ ] No shell fallback when `run_node` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/531_run_node.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
