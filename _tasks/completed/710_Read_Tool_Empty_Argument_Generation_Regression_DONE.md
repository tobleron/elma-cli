# Task 710: Read Tool Empty Argument Generation Regression

## Type

Tool Calling / Schema Contract

## Severity

High

## Evidence

Round 5 prompt testing still produced malformed `read` calls after Task 708 was completed:

- `TOOL_VALIDATION_ERROR tool=read error=filePath: required field 'filePath' is missing`
- Seen in prompts 02, 03, 04, 05, 06, and 07.
- The schema packet repair prevents total failure, but Elma still wastes turns and enters stagnation after the first invalid read.

Example sessions:

- `sessions/s_1778119677_812786000`
- `sessions/s_1778119753_596095000`
- `sessions/s_1778120010_274506000`

## Problem

The model is still being allowed to emit an empty `read` call instead of receiving a sufficiently direct first-failure repair path. This is not a prompt-core problem. The tool execution layer should make the correct next action obvious and cheap.

## Requirements

- Convert an empty `read` call into a bounded model-facing correction before it consumes repeated iterations.
- Prefer a deterministic fallback when recent search/glob output contains a clear candidate path.
- Do not inject bogus paths from unrelated history.
- Persist a concise trace row showing whether repair used schema, search history, or blocked execution.
- Ensure the next turn context packet includes the failed tool name and the exact missing field.

## Acceptance Criteria

- [ ] Prompt suite runs show no repeated empty `read` validation loops.
- [ ] A single malformed `read` call either repairs into a valid `read` or redirects to a specific `shell cat/head` command.
- [ ] Trace includes the repair source and selected fallback path when one exists.
- [ ] No changes are made to `src/prompt_core.rs`.

