# Task 531: job_start - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `job_start` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `job_start` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/job_start.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Start background job with command "sleep 30" name "sleep_job", then immediately check status with job_status. Start second job "echo hello && sleep 10 && echo done" name "echo_job". Check both statuses, read echo_job output, stop sleep_job. Test with memory_limit_mb and timeout_seconds. Test non-existent command.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Command Hygiene
Command param should be simple shell string, NOT Python script.

### Approach B: Step Decomposition: Start-Monitor-Stop Cycle
start -> status -> output -> stop. Missing any step is failure.

### Approach C: Shell-Fallback Guard: Block Backgrounding
Block shell commands ending in & or containing nohup.

### Approach D: Error Recovery: Duplicate Job Name
If duplicate, use unique name and retry.

## Success Criteria
- [ ] The model calls `job_start` successfully in every scenario from the stress test
- [ ] No shell fallback when `job_start` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/532_job_start.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
