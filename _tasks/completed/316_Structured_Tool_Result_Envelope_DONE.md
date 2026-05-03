# Task 316: Structured Tool Result Envelope (Proposal 007)

**Status:** pending  
**Proposal:** [docs/_proposals/007-structured-tool-result-envelope.md](../../docs/_proposals/007-structured-tool-result-envelope.md)  
**Depends on:** None  

## Summary

Add `exit_code: Option<i32>`, `timed_out: bool`, `signal_killed: Option<i32>` to `ToolExecutionResult` in `src/tool_calling.rs`. Pass these fields from `ShellExecutionResult` through `exec_shell`. Replace string-based `classify_error()` timeout detection with `result.timed_out` boolean check.

## Why

`ShellExecutionResult` (`types_api.rs:148`) already carries `timed_out`, `exit_code`, `truncated`. But `exec_shell` (`tool_calling.rs:306-311`) only extracts `ok` and `content`. The `timed_out` boolean is lost. The stop policy's `classify_error()` (`stop_policy.rs:462`) uses fragile string matching for `"timeout"`/`"timed out"`. This violates Elma's principle against hardcoded keyword triggers and causes the policy layer to miss structured timeout detection.

## Implementation Steps

1. Add 3 fields to `ToolExecutionResult` struct in `src/tool_calling.rs:7-13`
2. Update `exec_shell` success path (`tool_calling.rs:286-311`) to pass `timed_out`, `exit_code` from `ShellExecutionResult`
3. Update `exec_shell` error path (`tool_calling.rs:313-323`) to set `timed_out: true` when error message contains "timed out"
4. Update all other executors (exec_read, exec_search, exec_respond, exec_tool_search, exec_update_todo_list, exec_read_evidence) with defaults: `exit_code: None`, `timed_out: false`, `signal_killed: None`
5. Replace `classify_error()` string matching for timeout in `stop_policy.rs:452-466` with `result.timed_out` check
6. Pass real `exit_code` from `result` to `EvidenceSource::Shell` in `tool_loop.rs:940-942`
7. Build and test

## Success Criteria

- [x] `ToolExecutionResult` has `exit_code: Option<i32>`, `timed_out: bool`, `signal_killed: Option<i32>`
- [x] `exec_shell` passes all three from `ShellExecutionResult`
- [x] `classify_error()` uses `result.timed_out` not string matching for timeout
- [x] `EvidenceSource::Shell { exit_code }` receives real exit code
- [x] All constructors updated
- [x] `cargo build` succeeds
