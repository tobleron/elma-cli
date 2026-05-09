# Task 697: Required Artifact Contract And Filename Inference

## Type

Finalization / Artifact Reliability

## Severity

High

## Scope

Artifact verifier, finalization, task outputs

## Session Evidence

This round added a direct fix that persists missing required artifacts during finalization. It worked for prompt 01 by creating `project_tmp/security_report.md`, and prompt 06 created `project_tmp/insecure_endpoints_report.md` via the `write` tool.

Remaining issues:

- Prompt 03 inferred and wrote generic `project_tmp/report.md`, which can collide across unrelated report tasks.
- Prompt 04 and 06 also used generic report inference in traces.
- Prompt 07 claimed `security_todo_summary.md`, but the verifier did not infer or create that exact artifact because the requested summary filename was implicit in model output, not the user request.

## Problem

Required artifact tracking is now partially effective, but filename inference is too generic and finalization-time persistence can produce broad filenames such as `report.md`. Enterprise-grade CLI behavior needs stable, task-specific artifact names and clear evidence that the artifact fulfills the user's request.

## Proposed Solution

Improve artifact contract handling:

- Derive task-specific filenames from the user intent, not generic `report.md`, using bounded slug generation.
- Track requested artifact type separately from path: report, summary, backup directory, edited files, verification log.
- If the model later claims a concrete output path, register and verify that path before final answer.
- Avoid overwriting generic artifacts from previous prompts; use collision-safe names.
- Persist an artifact manifest in the session with requested, inferred, created, and verified outputs.

## Acceptance Criteria

- [ ] Prompts 02-07 produce distinct, task-specific artifact names.
- [ ] Final answers cannot claim an output file that is missing.
- [ ] Session artifacts include an output manifest with requested and verified deliverables.
- [ ] Replaying prompts in sequence does not overwrite unrelated report files.

## Verification Plan

Replay prompts 01-07 after deleting `project_tmp` test artifacts. Verify distinct files and manifest entries for each session.

