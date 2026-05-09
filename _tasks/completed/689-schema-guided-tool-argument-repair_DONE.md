# Task 689: Schema Guided Tool Argument Repair

## Type

Model Robustness

## Severity

High

## Scope

Tool-specific

## Session Evidence

Malformed `read` calls occurred in nearly every test session:

- `sessions/s_1778084330_810579000/session.md`: two failures, `filePath: required field 'filePath' is missing`.
- `sessions/s_1778084542_908955000/session.md`: `read` failed with missing `filePath`.
- `sessions/s_1778084708_633588000/session.md`: `read` failed with missing `filePath`.
- `sessions/s_1778084857_555628000/session.md`: `read` failed, then repeated duplicate skipped failures.
- `sessions/s_1778085073_737796000/trace_debug.log`: failed `read` for `project_tmp/INSECURE_HTTP_ENDPOINTS.md` even after a successful write.

The model received a schema error message with an example, but still repeated malformed calls.

## Problem

Dense/small local models are likely to produce imperfect tool JSON. Elma currently reports schema errors back to the model but does not reliably repair obvious argument-shape failures or choose deterministic alternatives, causing wasted cycles and premature budget exhaustion.

## Root Cause Hypothesis

Confirmed: validation errors are recorded, but no Rust-side argument repair resolved missing `filePath` failures.

Likely: schema feedback is too dependent on the model self-correcting in the next call.

## Proposed Solution

Implement schema-guided deterministic repair and targeted retry:

- Inspect `src/strict_tool_parser.rs`, `src/json_repair.rs`, `src/tool_calling.rs`, `src/tools/validation.rs`, and `elma-tools/src/tools/read.rs`.
- For required path arguments, infer a candidate path only from explicit current tool-call fields, previous successful write target, or immediately preceding model text that names a path.
- Retry once with a repaired argument when confidence is high; otherwise generate a model-facing correction with the exact missing field and a small valid JSON object.
- Add a per-tool failure circuit so repeated identical schema errors trigger strategy repair instead of repeated skipped calls.
- Ensure all repair decisions are logged as transcript rows and structured events.

## Acceptance Criteria

- [ ] Missing `filePath` read calls are either deterministically repaired or stop after one correction attempt.
- [ ] Repeated identical malformed tool calls do not consume multiple normal tool iterations.
- [ ] Tests cover missing `filePath`, wrong path key aliases, and repaired read-after-write.

## Verification Plan

Add unit tests around strict parser/tool validation, then replay prompts 01, 04, and 06. Confirm malformed `read` loops disappear from `trace_debug.log`.

## Dependencies

None.

