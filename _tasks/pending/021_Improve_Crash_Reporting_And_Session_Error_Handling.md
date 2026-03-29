# 021_Improve_Crash_Reporting_And_Session_Error_Handling

## Problem
Session `s_1774826560_84116000` ended with no clear error:
```
💡 Retry 1/4 (temp=0.0, strategy=standard)
[END OF LOG - NO ERROR MESSAGE]
```

There was no:
- Panic message
- Error written to log
- Error JSON file created
- Explanation of what went wrong

This makes debugging impossible - we don't know if it was:
- API timeout
- JSON parse failure
- Process crash
- User interrupt
- Out of memory

## Objective
Implement comprehensive crash reporting so all session failures produce actionable error information.

## Technical Tasks

- [ ] **Create error reporting module**
  - New file: `src/error_report.rs`
  - Struct `SessionError` with fields:
    - `error_type`: "timeout" | "parse_error" | "api_error" | "panic" | "unknown"
    - `component`: which module failed (orchestrator, reflection, etc.)
    - `message`: human-readable description
    - `timestamp`: when error occurred
    - `last_action`: what was being attempted
    - `context`: relevant state (model, temperature, attempt number)

- [ ] **Add panic hook**
  - Install custom panic hook with `std::panic::set_hook()`
  - On panic: write error report to session directory
  - Include stack trace (if available)
  - Exit gracefully with error code

- [ ] **Wrap main entry points with error handling**
  - `run_chat_loop()` - catch all errors
  - `orchestrate_with_retries()` - log before giving up
  - `chat_once()` - distinguish timeout vs connection errors

- [ ] **Create `error.json` in session directory**
  - Written on any fatal error
  - Machine-readable format for analysis
  - Includes all `SessionError` fields

- [ ] **Improve trace logging for errors**
  - Always write error to `trace_debug.log` before exiting
  - Format: `[ERROR] component: message (context)`
  - Include retry attempt number if applicable

- [ ] **Add session status tracking**
  - `session_status.json` with:
    - `status`: "success" | "error" | "interrupted"
    - `turns_completed`: N
    - `last_turn`: user message
    - `error_summary`: if failed

## Acceptance Criteria
- [ ] No session ends without `error.json` or success marker
- [ ] Panic hook writes stack trace to log
- [ ] All error types clearly distinguished in reports
- [ ] Error messages are actionable (suggest fixes)

## Verification
1. Simulate panic with `panic!("test")` in test code
2. Confirm `error.json` created with stack trace
3. Simulate timeout (kill llama.cpp server)
4. Confirm error type is "timeout" with component info
5. Review error report clarity and actionability

## Related
- Session: `s_1774826560_84116000` (silent failure)
- Files: `src/main.rs`, `src/app_chat.rs`, `src/session.rs`
- T018: Timeout handling (complementary)
