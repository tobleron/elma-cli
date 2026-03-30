# 018_Add_Timeout_Handling_For_Model_API_Calls

## Problem
Session `s_1774826560_84116000` with granite-4.0-h-micro model ended abruptly during retry attempt with no error message:
```
💡 Retry 1/4 (temp=0.0, strategy=standard)
[NO FURTHER OUTPUT - SESSION CRASHED/HUNG]
```

The trace log shows no:
- Shell command saved
- Execution output
- Error message or panic

This suggests the model API call timed out or the process crashed without proper error handling.

## Objective
Add comprehensive timeout handling for all model API calls to prevent silent session failures.

## Technical Tasks

- [ ] **Audit all `chat_once` and `chat_json_with_repair` calls**
  - Identify all locations where model API calls are made
  - Check if timeouts are currently configured

- [ ] **Add configurable timeout per component**
  - Orchestrator: 120s (default, already exists)
  - Reflection: 60s
  - Router/Classification: 30s
  - JSON outputter: 60s
  - Critics/Reviewers: 90s

- [ ] **Implement timeout error handling**
  - Catch `reqwest::Error` with timeout variants
  - Log clear error message: "Model API timeout after Xs"
  - Return structured error to caller
  - Allow retry with backoff if appropriate

- [ ] **Add session-level timeout tracking**
  - Track total session time
  - Warn when approaching timeout threshold
  - Gracefully fail with summary if exceeded

- [ ] **Improve crash reporting**
  - Always write error to `trace_debug.log`
  - Create `error.json` in session directory on fatal errors
  - Include: error type, component, timestamp, last action

## Acceptance Criteria
- [ ] No session ends silently - all failures produce error logs
- [ ] Timeout errors clearly identified in trace logs
- [ ] Retry logic respects timeout configuration
- [ ] Session artifact includes error summary on failure

## Verification
1. Simulate API timeout (e.g., stop llama.cpp server mid-request)
2. Confirm error is logged to `trace_debug.log`
3. Confirm session directory contains error information
4. Confirm no hanging processes left behind

## Related
- Session: `s_1774826560_84116000` (granite-4.0-h-micro crash)
- Files: `src/ui.rs`, `src/orchestration.rs`, `src/reflection.rs`
