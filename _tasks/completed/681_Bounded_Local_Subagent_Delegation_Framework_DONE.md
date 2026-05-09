# Task 681: Bounded Local Subagent Delegation Framework

**Status:** pending
**Priority:** LOW
**Type:** Architecture / Optional Offline Feature
**Scope:** future delegation modules, `src/work_graph.rs`, `src/task_persistence.rs`, `src/event_log.rs`
**Source:** deferred task 492, postponed tasks 162/267

## Summary

Design subagent delegation as an optional, bounded, local-first work-graph feature with explicit budgets, ownership, and transcript visibility.

## Evidence And Gap

- Delegation can help open-ended tasks but risks complexity, budget waste, and hidden work.
- User asked this audit to delegate an agent, but Elma itself should prioritize reliable local core behavior before optional delegation.

## Implementation Plan

1. Define delegation only as sibling work-graph branches with explicit objective, scope, files, budgets, and stop conditions.
2. Persist delegated task state and merge results through evidence ledger, not memory-only summaries.
3. Require transcript rows for spawn, status, result, failure, and cancellation.
4. Disable network/API-based delegation by default.

## Acceptance Criteria

- [ ] Delegation cannot mutate unowned files or exceed configured budgets.
- [ ] Parent final answer distinguishes local evidence from subagent evidence.
- [ ] Failed subagents do not block unrelated branches indefinitely.
- [ ] Feature can be disabled with no core behavior change.

## Verification Plan

Use fake local subagent fixtures and work-graph persistence tests.

