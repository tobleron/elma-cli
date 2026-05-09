# Task 688: Artifact Deliverable Verifier

## Type

Finalization

## Severity

Critical

## Scope

System-wide

## Session Evidence

Prompts 01 through 05 requested reports in `project_tmp`, but no requested report files were created. Only terminal capture files from this external test harness appeared.

- Prompt 01 requested a detailed security report in `project_tmp`; `sessions/s_1778084330_810579000/session.md` has no `write` tool call and ends at `iteration_limit_reached`.
- Prompt 02 requested a security report in `project_tmp`; `sessions/s_1778084542_908955000/session.md` has no `write` tool call and ends at `iteration_limit_reached`.
- Prompt 03 final answer claims `Created: external_api_reference_report.md`, but `find project_tmp` showed no such file.
- Prompt 05 final answer contains `project_tmp/test_organization_report.md`, but no such file was created.
- Prompt 06 did create `project_tmp/INSECURE_HTTP_ENDPOINTS.md`, proving file output is possible but not reliably enforced.

## Problem

Elma can answer in the transcript while leaving explicit file deliverables undone. This breaks semantic continuity and makes the final answer unreliable for real work.

## Root Cause Hypothesis

Confirmed: finalization does not verify that requested output artifacts exist.

Likely: the tool loop does not track user-requested deliverables as first-class completion criteria.

## Proposed Solution

Add deterministic deliverable tracking:

- Inspect `src/tool_loop.rs`, `src/final_answer.rs`, `src/continuity.rs`, `src/work_graph_bridge.rs`, `src/task_persistence.rs`, and write/patch tool handling in `src/tool_calling.rs`.
- Extract required artifacts from the objective into structured completion criteria, without keyword-trigger routing.
- Record artifact requirements in session runtime state.
- Before finalization, verify required files exist, are non-empty when applicable, and match requested location.
- If missing, force continuation with a concise repair objective such as "create the missing report file and verify it exists".
- If the model cannot complete the artifact, final answer must explicitly say which deliverable is missing and why.

## Acceptance Criteria

- [ ] A prompt asking for `project_tmp/<report>.md` cannot finalize as complete unless that file exists.
- [ ] Final answers cannot claim a file was created unless a successful write/shell/mkdir/copy event supports it.
- [ ] Missing deliverables are visible in `session.md`, `session.json`, and `trace_debug.log`.

## Verification Plan

Replay prompts 01, 02, 03, and 05. Verify each creates a concrete report in `project_tmp` or ends with an explicit incomplete-state final answer citing the missing file.

## Dependencies

Task 687.

