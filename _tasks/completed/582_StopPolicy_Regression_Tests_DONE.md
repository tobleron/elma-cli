# 582 — Add Regression Tests for Stagnation and Stop Policy

- **Priority**: Medium
- **Category**: Testing
- **Depends on**: None
- **Blocks**: None

## Problem Statement

The stop policy (`stop_policy.rs`, 1280 lines) has unit tests for individual components but lacks regression tests for complex scenarios:

1. **False positive regression**: Valid tool usage that should NOT trigger stop
2. **False negative regression**: Stagnation patterns that SHOULD trigger stop but don't
3. **Interaction between stop reasons**: Multiple stop conditions firing simultaneously
4. **Realistic command sequences**: Sequences that mimic actual small-model behavior patterns
5. **Signal normalization edge cases**: Commands that should normalize to the same signal

## Why This Matters for Small Local LLMs

The stop policy is the primary mechanism for preventing small models from wasting context window on unproductive loops. False positives (stopping too early) prevent task completion. False negatives (not stopping) waste tokens and time.

## Recommended Target Behavior

Add regression tests for known failure patterns:

### Test Scenarios

1. **Chat loop**: Model calls `respond` repeatedly without tools → should trigger RespondAbuse
2. **Empty search loop**: Model calls `search` with slightly different patterns → should trigger stagnation
3. **Shell retry with same strategy**: 3 consecutive `find . -type f` failures → retry loop detected
4. **Shell retry with DIFFERENT strategy**: `find` fails → switch to `ls` → should NOT trigger (strategy change)
5. **Read-read-respond pattern**: Normal workflow → should NOT trigger stop
6. **Gradual refinement**: Model tries `find .`, then `find . -maxdepth 1`, then `find . -maxdepth 1 -name '*.rs'` → should NOT trigger stagnation (arguments changing)
7. **Shell failure without retry**: `find` fails once, model switches to `search` → should NOT trigger
8. **Wall clock limit**: Simulate time passing faster than wall clock
9. **Goal consistency milestone**: 18+ tool calls → goal check triggered
10. **Boundary: 17 calls → 18 calls**: Verify milestone fires exactly once

## Source Files That Need Modification

- `src/stop_policy.rs` — Add regression test module

## Acceptance Criteria

- 10 regression test scenarios added
- Tests cover false positive and false negative cases
- Tests use realistic command sequences
- Tests run in CI
- All existing stop policy tests still pass

## Risks and Migration Notes

- These tests depend on the current stop policy behavior. If the stop policy is intentionally changed, tests must be updated to match new expected behavior — not just blindly deleted.
