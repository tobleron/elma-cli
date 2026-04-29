# Task 319: Principle-Based Strategy Guidance (Proposal 010)

**Status:** pending  
**Proposal:** [docs/_proposals/010-principle-based-strategy-guidance.md](../../docs/_proposals/010-principle-based-strategy-guidance.md)  
**Depends on:** Task 316 (needs `result.timed_out` for error class)  

## Summary

Replace `suggest_alternatives()` hardcoded match arms in `stop_policy.rs:431-449` with principle-based guidance derived from error class and scope classification. Removes keyword-trigger violations (constraint #1).

## Why

Current `suggest_alternatives()` maps specific strategy names to hardcoded alternative command lists (e.g. `"find_other"` → `"Use rg... Use fd..."`). This violates Elma's principle against hardcoded pattern matching. It teaches the model to pattern-match rather than reason about strategy shifts. A principle-based approach describes *what went wrong* and *principles for recovery* rather than listing specific commands.

## Implementation Steps

1. Change `suggest_alternatives()` signature to accept `(strategy: &str, error_class: &str, scope: &str)`
2. Remove hardcoded match arms — replace with error-class-based guidance and scope-based narrowing advice
3. Update `strategy_shift_hint()` in `stop_policy.rs:251-265` to pass `error_class` and `scope` from `last_shell_*` fields
4. Store `last_error_class` and `last_scope` alongside `last_shell_strategy` in StopPolicy
5. Build and test

## Success Criteria

- [x] `suggest_alternatives()` takes `(strategy, error_class, scope)` not just `(strategy)`
- [x] No string matching on command names
- [x] `strategy_shift_hint()` passes error_class and scope
- [x] `cargo build` succeeds
