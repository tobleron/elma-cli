# Task 709: Artifact Synthesis Timeout And Retry Budget

## Type

Finalization / Provider Resilience

## Severity

Medium

## Session Evidence

Round 4 prompt 01 showed that model-based artifact synthesis can waste time retrying after a short or failed provider response:

- `artifact_synth_failed api project_tmp/security_report.md: Model API timeout after 30s`
- fallback artifact was written successfully after the synthesis retries failed
- later runs showed synthesis retrying because a short response was considered truncated

The new deterministic fallback prevents total failure, but synthesis still waits too long before falling back.

## Problem

Artifact synthesis is a recovery path, not the main autonomous work loop. It should be bounded tightly. If the endpoint is unstable or returns a short response, Elma should not spend multiple long attempts before writing a deterministic evidence-backed fallback.

## Proposed Solution

Make artifact synthesis explicitly bounded and fallback-first under provider instability.

Likely source areas:

- `src/tool_loop.rs`
- `src/provider_recovery.rs`
- `src/finalization_verifier.rs`
- `src/artifact_verifier.rs`

Requirements:

- Use one short synthesis attempt for required artifacts.
- Accept short non-empty synthesis output for artifact files; do not treat it like a final answer that needs retry.
- If synthesis fails once, immediately write deterministic evidence fallback.
- Include stop reason and evidence summary in fallback artifacts.
- Trace synthesis attempt count and fallback reason.

## Acceptance Criteria

- [ ] Artifact synthesis does not perform three long retries during report-task recovery.
- [ ] Required artifact fallback is written within one failed synthesis attempt.
- [ ] Final answer exits deterministically once required artifacts exist.

## Verification Plan

Run prompt 01 with the endpoint available and again with the endpoint temporarily unavailable.

Pass criteria:

- In both cases `project_tmp/security_report.md` exists.
- Trace shows at most one synthesis attempt before fallback.
- Session exits without hanging in finalization.

