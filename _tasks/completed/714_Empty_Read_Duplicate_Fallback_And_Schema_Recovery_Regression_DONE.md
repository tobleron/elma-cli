# Task 714: Empty Read Duplicate Fallback And Schema Recovery Regression

## Type

Tool Runtime / Small-Model Robustness

## Severity

High

## Evidence

Round 6 prompt testing completed all eight prompts, but prompts 02-05 repeatedly generated invalid `read` calls with missing `filePath`:

- `project_tmp/round6_sessions/prompt_02_s_1778140405_595961000/trace_debug.log`
- `project_tmp/round6_sessions/prompt_03_s_1778140497_762270000/trace_debug.log`
- `project_tmp/round6_sessions/prompt_04_s_1778140549_451538000/trace_debug.log`
- `project_tmp/round6_sessions/prompt_05_s_1778140604_278026000/trace_debug.log`

The trace pattern is:

- `[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing`
- `tool_loop: duplicate skipped (previous failure) signal=read:`
- `tool_loop: stagnation run ... (tool: read)`
- `tool_loop: stopping reason=iteration_limit_reached`

A small direct patch has already routed duplicate empty `read` retries into the fallback path instead of skipping them, but this task must complete the underlying repair contract.

Focused rerun after the direct patch:

- `project_tmp/round6_sessions/prompt_02_rerun_after_signal_patch_s_1778141295_816763000/trace_debug.log`
- Duplicate `read:{}` suppression stopped, but the model then emitted `path="_tasks/pending/02_Search_Database_Connection_Strings.md"`, which canonicalized to `filePath` and failed with `file_not_found`.
- This confirms the remaining problem is broader than duplicate empty reads: `read` path alias repair also needs existence validation and evidence-grounded candidate selection.

## Problem

The `read` validation recovery path is still too passive. An empty `read` after evidence discovery should not consume multiple model turns. Elma should deterministically recover by selecting an evidence-backed path, switching to `shell cat/head` only when appropriate, or asking the model for a different discovery step with a compact correction packet.

## Requirements

- Make repeated empty `read` calls impossible to loop on.
- Prefer evidence-backed path injection when recent `glob`, `search`, `ls`, or `write` results contain valid workspace-relative paths.
- Do not inject arbitrary search result lines that include match content or terminal noise.
- Do not accept model-supplied `path` aliases for `read` unless the target exists or is otherwise evidence-backed.
- If no reliable candidate path exists, force a strategy shift once and then suppress identical failed `read:{}` calls.
- Add trace events that distinguish:
  - empty read repaired from evidence
  - empty read converted to shell fallback
  - empty read suppressed because no candidate exists
- Add regression tests for duplicate failed `read:{}` calls reaching fallback logic instead of duplicate-skip logic.
- Keep this outside `src/prompt_core.rs`.

## Acceptance Criteria

- [ ] Round 6-style prompt traces no longer show repeated `duplicate skipped (previous failure) signal=read:` after an empty read failure.
- [ ] Empty read recovery has tests covering no candidate, search candidate, glob candidate, and prior write candidate.
- [ ] Recovered paths are workspace-relative and do not include line-number/match-content suffixes.
- [ ] `cargo test -q read` and the relevant `tool_repair`/`tool_loop` tests pass.
