# Task 747: Replace Keyword-Based Complexity Assessment With Model-Inferred Classification

## Type

Architecture / Rule 1 Violation

## Severity

Critical

## Scope

System-wide routing and budget allocation

## Problem

`src/complexity_assessor.rs` implements complexity classification entirely through hardcoded keyword string matching:

```rust
let code_change_signals = [
    "refactor", "implement", "create", "write a", "add a",
    "change", "modify", "update all", "migrate", "convert",
    "rename", "move", "extract",
];
let has_code_signal = code_change_signals.iter().any(|s| lower.contains(s));
```

This is a direct, unambiguous violation of **Architectural Rule 1: No Word-Based Routing**:

> "Never implement routing, classification, or behavior selection through hardcoded word triggers."
> "If you are checking user text for words to force a route, you are violating Elma's philosophy."

The current implementation:
1. Uses 37 hardcoded English strings to classify user intent
2. Maps those classifications directly to iteration budgets (DIRECT=3, INVESTIGATE=6, MULTISTEP=12, OPEN_ENDED=20)
3. Is tested with tests that validate the keyword heuristics rather than the desired behavior
4. Will fail on non-English requests, paraphrased requests, or any input that doesn't contain the exact keywords
5. Creates a brittle coupling between user vocabulary and system behavior

The docs explicitly call this out:
- **Wrong:** `if input.contains("hello") { route = "CHAT"; }`
- **Right:** "Use model confidence (entropy, margin)" / "Use evidence availability" / "Use bounded fallback principles"

## Root Cause

`complexity_assessor.rs` was created as a "lightweight heuristic" before the intel unit framework was mature. It has never been upgraded to use the model-based classification path that already exists in `routing_infer.rs` and the intel unit system.

## Proposed Solution

Replace the keyword heuristic with a **small-model-friendly complexity gate**. The gate may use deterministic non-semantic facts such as prior tool failures, active coverage ledger state, and available context budget, but it must not route from user-input word triggers. When semantic classification is needed, use a compact intel unit JSON contract:

```json
{"choice":"<NUMBER>","label":"<DIRECT|INVESTIGATE|MULTISTEP|OPEN_ENDED>","reason":"<ULTRA_CONCISE_JUSTIFICATION>","entropy":0.42}
```

### Phase 1 — Create `ComplexityClassificationUnit`

In `src/intel_units/` (or `src/complexity_gate.rs` if it already exists):

1. Implement the `IntelUnit` trait for complexity classification
2. Profile: use a lightweight profile (small model, low temperature, strict JSON grammar)
3. Input: current user message + minimal workspace brief + active coverage/goal state summary when present. Do not include raw full history or file trees.
4. Output: `ComplexityClassificationOutput` with `choice`, `label`, `reason`, `entropy`
5. Fallback: on any failure, return `INVESTIGATE` (conservative default)

### Phase 2 — Replace heuristic call site

In `src/app_chat_loop.rs` (and any other callers of `assess_complexity`):

1. Call the new complexity gate instead of `assess_complexity()`
2. Use the returned entropy to decide confidence:
   - High confidence (entropy < 0.2, margin > 0.5): use classification directly
   - Low confidence: default to `INVESTIGATE` (never downgrade to `DIRECT` on uncertainty)
3. Pass the classification through to `StageBudget::from_complexity()`

### Phase 3 — Remove or deprecate heuristic module

1. Delete `src/complexity_assessor.rs`
2. Move its tests to test the new intel unit's fallback behavior
3. Update `main.rs` to remove the module declaration

## Acceptance Criteria

- [ ] `src/complexity_assessor.rs` is deleted; no keyword-based complexity classification remains in the codebase
- [ ] Semantic complexity is classified via `IntelUnit` with compact JSON contract when needed
- [ ] The complexity gate does not consume or shrink conversation history
- [ ] Non-English requests receive reasonable classification (not defaulting to wrong complexity because of missing English keywords)
- [ ] Paraphrased requests ("make the code better" vs "refactor") receive consistent classification
- [ ] Unit tests verify the intel unit's fallback behavior and JSON contract adherence
- [ ] Integration test: a request without any hardcoded keywords still gets classified correctly by the model
- [ ] `cargo build && cargo test` passes

## Verification Plan

- Unit test: `ComplexityClassificationUnit` returns valid JSON with required fields
- Unit test: Fallback returns `INVESTIGATE` on parse failure
- Unit test: Non-English input does not crash or misclassify due to missing keywords
- Scenario test: "make the code better" → classified as `MULTISTEP` or `INVESTIGATE` (not `DIRECT`)
- Regression test: Verify old keyword tests are removed, not adapted to new system

## Dependencies

- `src/intel_trait.rs` (IntelUnit framework)
- `src/routing_infer.rs` (for entropy/margin calculation patterns)
- `src/json_grammar.rs` (for strict JSON grammar enforcement)

## Notes

This is not a prompt-tuning task. Do not "improve" the keyword lists or add more examples. The correct fix is to delete the keyword heuristic and use a narrow complexity gate that can rely on the model's own classification capability without bloating or compacting the main conversation context.

This task directly addresses Rule 1 and Rule 3 (decomposition for small models). The classification unit should be a single, narrow decision — not merged with routing or formula selection.
