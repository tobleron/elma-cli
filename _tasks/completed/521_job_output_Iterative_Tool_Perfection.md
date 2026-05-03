# Task 521: job_output - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `job_output` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `job_output` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/job_output.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Start a job printing sequential lines (python3 -c for i loop), wait for completion, then read output (verify all lines). Start a job with both stdout and stderr (verify both captured). Read output of still-running job (verify partial). Read non-existent job_id (verify error). Read completed job with no output.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Completion Check First
Always check job_status before job_output.

### Approach B: Output Parsing: stdout vs stderr
Model should distinguish stdout from stderr in output.

### Approach C: Empty Output Handling
If empty, check job_status to confirm job actually ran.

## Success Criteria
- [ ] The model calls `job_output` successfully in every scenario from the stress test
- [ ] No shell fallback when `job_output` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/534_job_output.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
