# Task 306: Dynamic Decomposition On Model Weakness

**Status:** completed
**References:** Directive 007

## Objective

Add a runtime monitor that detects when the model is struggling (repeated failures, high entropy, stagnation) and automatically triggers further decomposition into narrower sub-steps, rather than retrying the same failing approach.

## Scope

1. Add struggle detection monitor to `tool_loop.rs`: track repeated tool failures, same-command loops, high response entropy, and stagnation counters
2. Add `decompose_on_failure` hook to `orchestration_retry.rs`: before retrying, check if decomposition would help
3. Support dynamic execution depth escalation in `execution_ladder/depth.rs`
4. Surface decomposition events as transcript rows
5. Write unit tests for struggle thresholds and integration test for automatic decomposition

## Verification

```bash
cargo build
cargo test decomposition
cargo test struggle
```
