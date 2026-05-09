# Task 704: Finalization Evidence Gate And Artifact Persistence For Dense Models

## Type

Finalization / Evidence Integrity

## Severity

High

## Session Evidence

Round 3 traces show unstable finalization:

- Prompt 02 wrote `project_tmp/security_report.md`, then `respond` was blocked by the evidence gate for claims that were already inside the written report.
- Prompt 05 hit `finalization_failed_nonfatal stage=evidence error=error decoding response body`, retried finalization, and still logged `finalization_missing_artifacts count=1 paths=project_tmp/testing_report.md`.
- Prompt 01 and 03 relied on artifact persistence after `respond_abuse`.

## Problem

Finalization currently mixes three jobs:

- produce the user-facing answer
- validate unsupported claims
- synthesize or persist missing required artifacts

Dense models can produce noisy, truncated, or report-like final answers. The evidence gate sometimes blocks useful completion and sometimes fails to persist the required artifact.

## Proposed Solution

Separate finalization into deterministic stages.

Likely source areas:

- `src/tool_loop.rs`
- `src/finalization_verifier.rs`
- `src/artifact_verifier.rs`
- `src/provider_recovery.rs`
- `src/final_answer.rs`

Requirements:

- If a required artifact path exists, verify the artifact on disk before blocking final answer claims.
- If a required artifact is missing, synthesize the artifact from gathered evidence first, then produce a short final answer that references the artifact.
- Evidence gate should validate final answer claims separately from report body content.
- On provider decode failure, fall back to a deterministic evidence summary and persist the required artifact if possible.
- Avoid long free-form finalizer retries for dense models; use a compact finalization packet.

## Acceptance Criteria

- [ ] Written artifacts are not treated as unsupported final-answer claims.
- [ ] Missing required report artifacts are persisted before final answer.
- [ ] Provider decode failure still yields a useful artifact or explicit failure notice.
- [ ] `respond_abuse` does not prevent artifact completion for report tasks.

## Verification Plan

Run prompts 01, 02, 03, 05, and 06.

Pass criteria:

- Required report files exist and contain task-specific content.
- `trace_debug.log` does not show finalization missing artifacts for successful report tasks.
- Final answer is short and grounded in actual artifact paths.

