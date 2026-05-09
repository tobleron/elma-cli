# Task 693: Provider Finalization Recovery

## Type

Model Robustness

## Severity

Medium

## Scope

Model-specific

## Session Evidence

Prompt 06 completed several tool actions and wrote `project_tmp/INSECURE_HTTP_ENDPOINTS.md`, but finalization hit provider errors:

- `sessions/s_1778085073_737796000/trace_debug.log`: `finalization_failed_nonfatal stage=evidence error=error decoding response body`.
- The same trace logs three `[HTTP_ERROR] error sending request` entries and `finalization_failed_nonfatal stage=retry error=Model API timeout after 60s`.
- The final answer degraded to `[I found the following information, but the answer could not be finalized. Here's what I know:]`.

## Problem

When a late finalization call fails, Elma should preserve and present the completed tool evidence cleanly, avoid losing artifact verification, and allow continuation or retry without corrupting the session outcome.

## Root Cause Hypothesis

Confirmed: finalization errors are nonfatal but produce a degraded answer.

Likely: the fallback finalizer does not have enough structured state to produce a concise, verified completion summary without another large model call.

## Proposed Solution

Implement provider-aware finalization recovery:

- Inspect `src/llm_provider.rs`, `src/retry.rs`, `src/final_answer.rs`, `src/tool_loop.rs`, and `src/session_write.rs`.
- Add deterministic fallback finalization from structured tool events when the model finalizer fails.
- Preserve a retryable finalization state in `session.json`.
- Include model/transport error class, attempts, timeout, and surviving verified artifacts in `session.md`.
- Reduce finalization payload size when a previous finalization attempt fails due decode or timeout.

## Acceptance Criteria

- [ ] Finalization HTTP failures produce a concise evidence-backed fallback answer.
- [ ] Existing successful artifacts are listed and verified even when finalization model calls fail.
- [ ] Session state supports retrying finalization without rerunning all tools.

## Verification Plan

Replay prompt 06 with a short finalization timeout or simulated provider failure. Verify `session.md`, `trace_debug.log`, and final answer all preserve the successful write and explain the provider failure.

## Dependencies

Task 690.

