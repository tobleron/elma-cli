# Task 520: job_status - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `job_status` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `job_status` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/job_status.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
After starting a job, check status (should show running), wait and check again, then after completion check for completed/exited. Check non-existent job_id (verify error). Start multiple jobs and check all statuses. Verify output includes: job_id, name, status, exit_code, runtime, memory.
```

## Suggested Approaches

### Approach A: Prompt Engineering: When to Poll
Call repeatedly only when waiting for completion.

### Approach B: Step Decomposition: Start-Status-Output-Stop
Always pair status with corresponding start and output calls.

### Approach C: Error Handling: Job Not Found
Do NOT retry same job_id. Start new job.

### Approach D: Multi-Job Monitoring
Start multiple jobs and check statuses by job_id.

## Success Criteria
- [ ] The model calls `job_status` successfully in every scenario from the stress test
- [ ] No shell fallback when `job_status` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/533_job_status.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
