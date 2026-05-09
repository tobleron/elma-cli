# Task 695: Evidence-Derived Tool Argument Recovery

## Type

Model Robustness

## Severity

Critical

## Scope

Tool loop, tool validation, dense-model recovery

## Session Evidence

Prompt testing round 2 still produced malformed `read` calls after Task 689 was implemented:

- `project_tmp/round2_sessions/prompt_02_s_1778089174_321202000/session.md`: `TOOL FAIL [read] ... filePath: required field 'filePath' is missing`.
- `project_tmp/round2_sessions/prompt_03_s_1778089402_105164000/session.md`: missing `filePath`, followed by repeated read stagnation.
- `project_tmp/round2_sessions/prompt_05_s_1778089606_646514000/session.md`: missing `filePath`, then stagnation runs 3-5.
- `project_tmp/round2_sessions/prompt_07_s_1778090023_815304000/session.md`: `read` failed with `absolute path or parent traversal not allowed`.

## Problem

Elma can repair `path` aliases to `filePath`, but it still cannot recover when a dense model emits an empty `read` call or an absolute path. These failures waste tool iterations and often end in `respond_abuse` or `iteration_limit_reached`.

## Proposed Solution

Implement evidence-derived path recovery before schema validation:

- In `src/tool_loop.rs` and `src/tool_repair.rs`, track candidate file paths from successful `search`, `glob`, `ls`, `write`, and shell outputs, not only from tool arguments.
- For empty `read` arguments, select a candidate only when the immediately preceding evidence contains a clear file path and the task context implies inspection of that path.
- Canonicalize absolute workspace paths to workspace-relative `filePath` before validation.
- If no confident path exists, block repeated empty `read` calls after one failure and inject a deterministic alternate strategy such as `shell sed -n` or a narrower `search`.
- Record recovery decisions in `trace_debug.log` and as transcript-native operational rows.

## Acceptance Criteria

- [ ] Empty `read` calls do not repeat more than once per turn.
- [ ] Absolute workspace paths are converted to valid relative `filePath` values.
- [ ] Search-result paths can seed a subsequent repaired `read` when confidence is high.
- [ ] Prompt tests 01, 02, 03, 05, and 07 show no repeated missing-`filePath` loops.

## Verification Plan

Add unit tests for `path` alias, empty read with recent search result, absolute workspace path, and no-confidence fallback. Replay prompts 01, 03, 05, and 07 and inspect their traces.

