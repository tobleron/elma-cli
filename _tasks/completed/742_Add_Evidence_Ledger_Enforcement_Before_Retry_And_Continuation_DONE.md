# Task 742: Add Evidence-Ledger Enforcement Before Retry And Continuation

## Type

Architecture / Reliability / Hardening

## Severity

Critical

## Scope

Evidence system, retry logic, tool loop, finalization

## Problem

**Architectural Rule 13** mandates:

> "Evidence-Ledger Enforcement: Before any retry or request for input, the agent must provide an `Evidence Ledger` summary. If the failure reason is not explicitly articulated, the task is not ready for remediation."

This is **not implemented** in the codebase.

Current retry behavior:
1. Tool fails → `tool_calling.rs` returns error string to model
2. Model retries with same or modified tool call
3. No evidence ledger summary is provided
4. The model has no visibility into what evidence already exists, what failed, and why

Current continuity retry behavior:
1. Continuity check fails (score < threshold)
2. System injects a retry message: "The previous answer may not fully address your request."
3. The retry message includes evidence count but no actual evidence summary
4. The model retries without knowing what evidence exists

Current stagnation behavior:
1. Stop policy detects stagnation
2. System produces final answer or stops
3. No evidence ledger summary is provided to the user or the model

This violates:
- **Rule 13**: evidence-ledger enforcement
- **Rule 5**: grounded answers only — retries without evidence summaries are ungrounded
- **Rule 11**: eliminate retry loops — without evidence summaries, the model retries blindly

## Root Cause

The evidence ledger tracks evidence but doesn't expose structured summaries for retry/continuation contexts. The retry logic was designed around error strings, not evidence state.

## Proposed Solution

### Phase 1 — Add `EvidenceSummary` struct

In `src/evidence_ledger.rs`:

```rust
pub(crate) struct EvidenceSummary {
    pub total_entries: usize,
    pub successful_reads: Vec<String>, // paths
    pub successful_searches: Vec<String>, // patterns
    pub successful_shells: Vec<(String, i32)>, // (cmd, exit_code)
    pub failed_operations: Vec<(String, String)>, // (tool, error)
    pub quality_distribution: (usize, usize, usize), // (direct, indirect, weak)
    pub staleness_check: Vec<String>, // warnings about stale evidence
}
```

### Phase 2 — Generate summary before retry

Before any retry (tool retry, continuity retry, approach fork):

1. Call `evidence_ledger.generate_summary()`
2. Include the summary in the retry context:
   ```
   [Evidence Summary]
   - 3 files read: src/main.rs, src/lib.rs, Cargo.toml
   - 1 search: "fn main" (12 results)
   - 1 shell: cargo test (exit 0)
   - 1 failure: read filePath=src/nonexistent.rs (file not found)
   - Quality: 3 direct, 1 weak
   ```

### Phase 3 — Include summary in tool loop retry

In `tool_loop.rs`, when a tool fails and the model is about to retry:

1. Append evidence summary to the tool result message
2. This gives the model context: "You already tried X and Y; here's what succeeded and failed"

### Phase 4 — Include summary in continuity retry

In `app_chat_loop.rs`, when continuity retry is triggered:

1. Replace the current generic retry message with one that includes the evidence summary
2. Highlight the gap: "You've read 3 files but haven't examined the docs/ directory"

### Phase 5 — Include summary in stagnation halt

In `stop_policy.rs` (Task 745), when stagnation halt is triggered:

1. Include the evidence summary in the halt message
2. This helps the user understand what the agent did before getting stuck

## Acceptance Criteria

- [ ] `EvidenceSummary` struct exists with all fields
- [ ] Every retry (tool, continuity, approach) includes an evidence summary
- [ ] The summary is concise (≤ 10 lines)
- [ ] Failed operations are explicitly listed
- [ ] Quality distribution is included
- [ ] The summary is visible in the transcript (not just trace logs)
- [ ] `cargo build && cargo test` passes

## Verification Plan

- Unit test: `generate_summary()` with 3 entries → correct counts
- Unit test: retry with evidence summary → summary appears in context
- Integration test: tool fails twice → second retry includes summary of first attempt
- Scenario test: stagnation halt → user sees evidence summary in halt message

## Dependencies

- `src/evidence_ledger.rs` (summary generation)
- `src/tool_loop.rs` (tool retry)
- `src/app_chat_loop.rs` (continuity retry)
- `src/stop_policy.rs` (stagnation halt)
- Task 741 (session-scoped evidence ledger)
- Task 745 (stagnation halt)

## Notes

This is about **transparency**, not just tracking. The model and user must both see what evidence exists before retrying.

The summary must be concise. Dumping raw evidence entries into the context window is counterproductive. Use the existing `compact_summary()` method as a base.

The summary should be generated at retry time, not cached. Evidence state changes between retries.

This directly addresses the "completion-adequacy gap" identified in Task 735: the model stops because it doesn't know what evidence it still needs. The evidence summary makes the gap visible.
