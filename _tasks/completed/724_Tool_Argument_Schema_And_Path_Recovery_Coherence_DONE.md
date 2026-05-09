# Task 724: Tool Argument Schema And Path Recovery Coherence

## Type

Tool Calling / Schema Validation / Dense-Model Robustness

## Severity

High

## Evidence

Round 8 prompt testing repeatedly produced malformed path tool calls:

- Prompt 01: `sessions/s_1778146613_189124000/trace_debug.log`
  - `[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing`
  - duplicate skipped searches after the failed read
- Prompt 03: `sessions/s_1778147182_938803000/trace_debug.log`
  - repeated `read:` duplicate failures and stagnation runs
- Prompt 04: `sessions/s_1778147274_655146000/trace_debug.log`
  - `glob src/**/*.rs` succeeded, then `read` continued without a path
- Prompt 06: `sessions/s_1778147724_760976000/session.md`
  - `exists` failed with `path: required field 'path' is missing` even though the public tool schema advertises `paths`

A narrow direct patch was applied in this round to canonicalize `exists.paths[0]` to `path`, support multi-path `exists`, and return `ToolStatus::Success` for successful `exists`. This task covers the broader remaining schema coherence problem.

## Problem

Elma exposes mixed tool schemas and runtime validation contracts. Some model-facing schemas now prefer `path`/`paths`, while runtime validation still talks about `filePath`. When the model emits an incomplete `read` after `glob` or `search`, Elma often returns a correction packet but does not deterministically choose a safe candidate path or force a strategy shift soon enough.

This wastes iterations, causes stagnation, and pushes sessions into evidence-recovery artifacts instead of useful work.

## Requirements

- Unify model-facing and runtime validation contracts for `read`, `exists`, and other path tools.
- Support both `path` and `paths` where the executor supports them; reject unsupported aliases before the model sees conflicting guidance.
- When `glob` or `search` returns valid paths and the next `read` is empty, deterministically repair only when there is a high-confidence candidate from the immediately preceding successful tool result.
- If no high-confidence path exists, inject a compact strategy-shift event that tells the model to choose from a short candidate list instead of repeating empty `read`.
- Do not let duplicate empty reads consume more than one additional iteration.
- Add tests using Round 8 traces or fixtures for:
  - read with `path`
  - read with `paths`
  - read with missing path after glob
  - exists with `paths`
  - duplicate empty read suppression

## Likely Files

- `src/tool_calling.rs`
- `src/tool_repair.rs`
- `src/tools/validation.rs`
- `elma-tools/src/tools/read.rs`
- `elma-tools/src/tools/exists.rs`
- `src/strict_tool_parser.rs`

## Acceptance Criteria

- [ ] Prompts 01, 03, 04, and 05 no longer show repeated empty `read` stagnation after successful discovery.
- [ ] Prompt 06 no longer emits an `exists` validation failure for `paths`.
- [ ] Tool schemas, validation errors, and executor accepted arguments agree.
- [ ] `cargo test -q tool_repair` and relevant tool validation tests pass.

