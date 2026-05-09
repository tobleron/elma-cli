# Task 708: Read Tool Schema Packet And First Failure Repair

## Type

Tool Reliability / Dense Model Support

## Severity

High

## Session Evidence

Round 4 partial validation still showed malformed `read` calls after Tasks 701-707 were completed:

- `project_tmp/round4_sessions/prompt_01_*` trace contained `TOOL_VALIDATION_ERROR tool=read error=filePath: required field 'filePath' is missing`.
- The same session later repaired a path alias to `filePath`, but only after consuming additional iterations.

This means dense models still do not receive a sufficiently clear, immediate model-facing contract for `read` after the first schema failure.

## Problem

When the model emits a `read` call without `filePath`, the current loop lets the failure consume useful work budget and may repeat before repair. Report-style tasks then hit stop policy and depend on finalization fallback instead of completing from clean evidence.

## Proposed Solution

Implement first-failure `read` repair and schema packet behavior.

Likely source areas:

- `src/tool_loop.rs`
- `src/tool_repair.rs`
- `src/tools/validation.rs`
- `src/tool_calling.rs`
- `src/turn_context_packet.rs`

Requirements:

- On the first missing-`filePath` `read` validation error, emit a compact correction packet naming the exact required field and candidate paths.
- If a recent search/glob/ls produced valid paths, repair immediately without waiting for another model turn.
- Do not inject arbitrary non-path text as a candidate.
- Count repeated empty `read` calls as stagnation, not progress.
- Add regression tests for missing `filePath`, `path` alias, and no-valid-candidate cases.

## Acceptance Criteria

- [ ] Prompt 01 does not spend multiple iterations on missing `filePath`.
- [ ] `trace_debug.log` shows at most one missing-`filePath` validation error per turn.
- [ ] Repair source is logged as `schema_packet`, `alias`, or `evidence_path`.
- [ ] If no valid path exists, Elma switches back to search/glob instead of repeating empty read.

## Verification Plan

Run `_testing_prompts/01_prompt.txt`, `_testing_prompts/02_prompt.txt`, and `_testing_prompts/05_prompt.txt`.

Pass criteria:

- No repeated `read:` duplicate failures.
- Required report artifacts are generated from real evidence.
- `cargo test` includes focused tool repair coverage.

