# Task 379: Dynamic Decomposition On Weakness

**Status:** Pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Dependencies:** Task 377 (retry loop provides decomposition hook)
**References:** AGENTS.md Rule 4, objectives.md principle 7, _masterplan.md Task 306

## Problem

When a small model fails to produce valid output (malformed JSON, stagnation, hallucination), the current system tries retry with temperature escalation but does not decompose the task into simpler sub-tasks. AGENTS.md Rule 4 states: "When a small model struggles: first tighten the narrative/context, then reuse an existing intel unit if it fits, then add a new focused intermediary intel unit."

The existing `decompose_on_failure` hook in `src/orchestration_retry.rs:37-40` is a placeholder:

```rust
fn decompose_on_failure(_attempt: u32, _error_summary: &str) -> bool {
    false  // TODO
}
```

## Objective

Implement dynamic decomposition: when a model consistently fails at a task, the system automatically splits it into simpler sub-tasks that the model can handle reliably. Decomposition should align with the pyramid work graph from Task 389 so the system repairs the smallest failed unit instead of restarting the whole request.

## Implementation Plan

### Phase 1: Define Failure Signal Detection

Add failure classifiers to `src/orchestration_retry.rs`:

```rust
enum FailureClass {
    JsonParseFailure(String),       // Unable to parse intel unit output
    Stagnation(u32),                // Same respond N times
    HallucinatedClaim(String),      // Claim with no supporting evidence
    ToolRepeatedFailure(String),    // Same tool failed N times
    Timeout,                        // Wall clock exceeded
    EmptyOutput,                    // Model produced no output
}
```

### Phase 2: Strategy-Shift Table

Map failure classes to decomposition strategies:

| Failure Class | Strategy Shift | Description |
|--------------|----------------|-------------|
| JsonParseFailure | Split fields | One-field-per-call instead of multi-field JSON |
| Stagnation(3) | InspectFirst | Force evidence collection before responding |
| HallucinatedClaim | Evidence gate | Require cited sources for every claim |
| ToolRepeatedFailure(3) | Alternative tool | Try a different tool for the same goal |
| EmptyOutput | Clean context | Reset conversation, retry with minimal context |
| Timeout | Bounded sub-goal | Split into smaller sub-goals with per-goal timeout |

### Phase 3: Extract and Tighten

When decomposition is triggered, execute a lightweight "extract and tighten" wrapper before retrying:

1. Extract: Isolate the specific sub-task the model failed at
2. Tighten: Reduce the prompt to ONLY what's needed for that sub-task
3. Re-run: Execute only the failed sub-task with tightened context

### Phase 4: Bounded Decomposition

Never decompose more than 2 levels deep (original → sub-task → sub-sub-task). At level 3, accept the best output available rather than decomposing further.

### Phase 5: Tool-aware repair

When failure is caused by an unsupported or poorly chosen tool, retry by discovering a better capability through Task 388 and preferring rust-native tools from Task 387 before shell fallback.

## Files to Modify

| File | Change |
|------|--------|
| `src/orchestration_retry.rs` | Replace placeholder `decompose_on_failure` with actual strategy-shift logic |
| `src/orchestration_planning.rs` | Add `build_decomposed_program()` for sub-task generation |
| `src/tool_loop.rs` | Emit failure classification events to retry layer |

## Verification

```bash
cargo build
cargo test decomposition
cargo test retry
```

**Manual**: Send a deliberately complex multi-step request. Verify:
1. Initial attempt fails
2. Decomposition produces simpler sub-tasks
3. Each sub-task succeeds independently
4. Results are recombined into a coherent final answer
