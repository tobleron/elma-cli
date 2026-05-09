# Task 774: Implement Evidence-Ledger Enforcement for Retries

## Type
Hardening / Reliability

## Severity
High

## Scope
System-wide / Orchestration

## Problem
Architectural Rule 13 ("Reliability & Hardening") requires that "Before any retry, is an Evidence Ledger summary provided?". Currently, the system often retries tool calls or approaches without explicitly surfacing the accumulated evidence or the specific reason for the prior failure in a structured narrative.

## Root Cause
The `Evidence Ledger` exists as a data structure but its enforcement as a mandatory pre-condition for retries is not fully integrated into the `tool_loop` or `orchestration_retry` logic.

## Proposed Solution
Enforce evidence-led retries across the orchestration pipeline.

- Phase 1: Modify `src/orchestration_retry.rs` to require an `EvidenceSummary` in its classification context.
- Phase 2: Update `src/tool_loop.rs` to inject a collapsible `Evidence Ledger` summary into the chat history when a stagnation or failure-based retry is triggered.
- Phase 3: Ensure that the model receives a clear articulation of "what we already know" before it is asked to try a different approach.
- Phase 4: Add verification that the retry narrative actually uses the evidence ledger data.

## Acceptance Criteria
- [ ] Every retry triggered by the orchestration layer includes a visible Evidence Ledger summary in the transcript.
- [ ] The system prompt or injected context for retries explicitly cites the evidence ledger.
- [ ] No "blind retries" (repeating the same prompt without new context) occur.

## Verification Plan
- Integration test: Trigger a tool failure and verify (via transcript) that the subsequent retry includes an "Evidence Ledger" section.
- Regression test: Verify that successful tasks do not include redundant evidence summaries.

## Dependencies
None

## Notes
- Align with "Rule 5 — Grounded Answers Only".
- Ensure the evidence summary is concise and does not bloat the context window unnecessarily.
