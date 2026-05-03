# Task 532: job_stop - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `job_stop` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `job_stop` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/job_stop.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Start long job (sleep 60), immediately stop (verify success). Stop same job again (verify already-stopped error). Start two jobs, stop one, verify other unaffected. Stop non-existent job_id (verify error). Start job, stop, confirm via ps that process is gone.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Cleanup Responsibility
Every background job must be stopped. Track all started jobs.

### Approach B: Step Decomposition: Stop-Then-Confirm
After stop, call job_status to confirm.

### Approach C: Error Recovery: Already Stopped
If job finished before stop, read output instead of error.

### Approach D: Force Stop on Stuck Jobs
Retry after 2s. Use kill -9 as last resort.

## Success Criteria
- [ ] The model calls `job_stop` successfully in every scenario from the stress test
- [ ] No shell fallback when `job_stop` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/535_job_stop.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
