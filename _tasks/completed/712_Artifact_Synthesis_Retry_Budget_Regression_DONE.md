# Task 712: Artifact Synthesis Retry Budget Regression

## Type

Finalization / Provider Resilience

## Severity

Medium

## Evidence

Task 709 was completed, but Round 5 still showed three long model attempts before deterministic fallback:

- `sessions/s_1778119753_596095000`
- `sessions/s_1778119895_295962000`

Trace pattern:

- `artifact_synth_attempt path=...`
- `HTTP_ATTEMPT attempt=1/3`
- `HTTP_ATTEMPT attempt=2/3`
- `HTTP_ATTEMPT attempt=3/3`
- `artifact_synth_failed api ... Model API timeout after 15s`
- `artifact_synth_fallback_written ...`

## Problem

Artifact synthesis is still using the generic provider retry budget. This wastes time in recovery paths and delays deterministic completion even though the fallback is good enough to finish the task.

## Requirements

- Give artifact synthesis its own retry policy independent of generic chat completion retries.
- Use at most one short model attempt for synthesis.
- Fall back immediately on timeout, transport error, or empty response.
- Accept short non-empty synthesis output if it writes the required artifact.
- Trace `artifact_synth_retry_budget=...` and `fallback_reason=...`.

## Acceptance Criteria

- [ ] Required artifacts are written after at most one failed synthesis attempt.
- [ ] Round prompt traces no longer show `HTTP_ATTEMPT attempt=2/3` or `3/3` during artifact synthesis.
- [ ] Final answer returns deterministically after artifact existence is verified.

