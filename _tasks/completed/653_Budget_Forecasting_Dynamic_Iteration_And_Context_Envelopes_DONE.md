# Task 653: Budget Forecasting Dynamic Iteration And Context Envelopes

**Status:** pending
**Priority:** HIGH
**Type:** Performance / Context Efficiency
**Scope:** `src/context_budget.rs`, `src/auto_compact.rs`, `src/tool_loop.rs`, `src/stop_policy.rs`, `src/model_capabilities.rs`
**Source:** deferred tasks 544/545, user priority on efficient offline architecture

## Summary

Replace static or loosely coupled budgets with objective-level token forecasting, dynamic iteration envelopes, and aggressive context conservation.

## Evidence And Gap

- `tool_loop.rs` and `stop_policy.rs` use iteration/tool-call caps, while context budget and compaction operate separately.
- Deferred tasks 544/545 already identified objective-level budget envelopes and budget-aware orchestration.
- Local models need smaller, more predictable context loads than cloud models.

## Implementation Plan

1. Estimate input, output, tool-result, evidence, and finalization budgets before each turn.
2. Tie max iterations/tool calls to complexity, risk, model context, and available evidence.
3. Trigger pre-turn or mid-turn compaction before overflow, not after failure.
4. Emit budget decisions as visible transcript rows.
5. Add per-tool result budgets that persist large outputs as artifacts with compact references.

## Acceptance Criteria

- [ ] Budget forecasts are saved in session artifacts.
- [ ] Tool loop caps adapt to complexity and model context.
- [ ] Context overflow cannot produce a false completed answer.
- [ ] Budget/compaction rows are visible in transcript.

## Verification Plan

Run long-output and long-session scenarios; inspect `session.json`, transcript rows, and final answers.

