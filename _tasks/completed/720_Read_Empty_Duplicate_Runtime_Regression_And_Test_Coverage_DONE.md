# Task 720: Read Empty Duplicate Runtime Regression And Test Coverage

## Type

Tool Loop / Runtime Regression

## Severity

High

## Evidence

Round 7 prompt testing still shows repeated empty `read` failures and duplicate suppression:

- `project_tmp/round7_sessions/prompt_02_s_1778143030_235992000/trace_debug.log`
- `project_tmp/round7_sessions/prompt_03_s_1778143082_46506000/trace_debug.log`

Observed trace pattern:

```text
[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing
trace: tool_loop: duplicate skipped (previous failure) signal=read:
trace: tool_loop: stagnation run ... (tool: read)
trace: tool_loop: stopping reason=respond_abuse
```

The source currently contains an empty-read duplicate branch, but this trace proves the compiled runtime still reached the generic duplicate-skip path in this scenario.

## Problem

Task 714 did not fully close the runtime behavior. Empty `read` calls remain capable of consuming turns, triggering stagnation, and causing deterministic finalization instead of evidence-grounded completion.

## Requirements

- Add a focused regression test that constructs the exact duplicate gate state:
  - prior failed `tool_outcomes` entry with signal `read:`
  - subsequent `read` call with `{}`, `{"filePath": ""}`, and `{"path": ""}`
  - expected branch is empty-read strategy shift, not generic duplicate skip.
- Ensure runtime trace emits one of:
  - `empty_read_repaired_from_evidence`
  - `empty_read_suppressed no_candidate_exists`
  - `empty_read_converted_to_shell`
- Ensure no prompt trace can show `duplicate skipped (previous failure) signal=read:` after an empty-read validation failure.
- Validate that candidate paths recovered from search/glob are existing workspace-relative paths before suggesting them.
- Keep the fix outside `src/prompt_core.rs`.

## Acceptance Criteria

- [ ] New unit/regression test fails before the fix and passes after it.
- [ ] Prompt 02 and prompt 03 no longer show `duplicate skipped (previous failure) signal=read:`.
- [ ] Empty-read loops terminate through a deterministic strategy shift, not respond abuse.
- [ ] `cargo test -q read` and relevant `tool_loop` tests pass.

