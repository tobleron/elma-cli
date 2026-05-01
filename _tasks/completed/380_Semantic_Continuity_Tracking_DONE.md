# Task 380: Semantic Continuity Tracking

**Status:** Pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Dependencies:** None
**References:** AGENTS.md Rule 3, objectives.md principle 5, _masterplan.md Task 302

## Problem

Per AGENTS.md Rule 3: "The meaning of the user's request must survive every transformation: intent annotation → routing → formula selection → execution → final answer. If the user asks for X and the answer solves Y, that is a semantic continuity failure."

Currently, there is no automated check that the final answer addresses the original user intent. A hallucinated answer like "17:35:06" for "what time is it now?" passes through because no step checks whether the answer actually answers the question.

## Objective

Implement a `ContinuityTracker` that preserves the user's original intent through every pipeline transformation and verifies semantic alignment at key checkpoints. This tracker must also attach to the pyramid work graph from Task 389 so objectives, goals, sub-goals, plans, and instructions can be compared back to the raw user request.
1. Pre-execution: Does the selected route/formula match the intent?
2. Post-execution: Does the final answer address the original question?
3. On routing change: Did the routing decision preserve the user's objective?
4. On work-graph decomposition: Does each generated goal/sub-goal still serve the original objective?

## Implementation Plan

### Phase 1: ContinuityTracker Struct

Add to `src/types_core.rs`:

```rust
pub(crate) struct ContinuityTracker {
    original_intent: String,        // From annotate_user_intent
    selected_route: String,         // From RouteDecision
    selected_formula: String,       // From FormulaSelection
    checkpoints: Vec<ContinuityCheckpoint>,
    alignment_score: f64,           // 0.0 (total mismatch) to 1.0 (perfect match)
}

pub(crate) struct ContinuityCheckpoint {
    stage: String,                  // "routing", "execution", "finalization"
    verdict: ContinuityVerdict,
    reason: String,
    timestamp_unix: u64,
}

pub(crate) enum ContinuityVerdict {
    Aligned,
    Drifted(String),                // What drifted
    Mismatch(String),               // What mismatched
}
```

### Phase 2: Pre-Execution Alignment Check

In `src/app_chat_loop.rs`, after routing decision is made (after Task 376 wires it in):

```rust
let continuity = ContinuityTracker::new(&rephrased_objective, &route_decision, &formula);
continuity.checkpoint("routing", || {
    // Verify route matches intent
    // e.g., INQUIRE intent + CHAT route with no tools = potential mismatch
    if route_decision.route == "CHAT" && intent_is_inquire(&rephrased_objective) {
        ContinuityVerdict::Drifted("INQUIRE intent routed to CHAT (no tool access)".into())
    } else {
        ContinuityVerdict::Aligned
    }
});
```

### Phase 3: Post-Execution Alignment Check

After `resolve_final_text` (line 1028 in `app_chat_loop.rs`):

```rust
continuity.checkpoint("execution", || {
    // LLM-based check: does the answer address the original question?
    // Uses a focused intel unit (one field: "aligned" or "mismatched")
    check_answer_vs_intent(&original_intent, &final_text)
});

// Surface continuity result in transcript
tui.push_meta_event("CONTINUITY", &format!("score={:.2}", continuity.alignment_score));
```

### Phase 4: Conservative Fallback on Drift

If alignment score < 0.5, trigger a retry with stricter evidence requirements:

```rust
if continuity.alignment_score < 0.5 {
    // Retry: force evidence-grounded response
    // "The user asked X. Provide evidence that your answer addresses X."
}
```

## Files to Modify

| File | Change |
|------|--------|
| `src/types_core.rs` | Add `ContinuityTracker`, `ContinuityCheckpoint`, `ContinuityVerdict` |
| `src/app_chat_loop.rs` | Wire checkpoints at routing, execution, and finalization stages |
| `src/intel_units/` | Add lightweight `AnswerContinuityUnit` (single-field: aligned/mismatched) |

## Verification

```bash
cargo build
cargo test continuity
```

**Manual**: Send 3 test queries:
1. "what time is it now?" → continuity should pass (answer = time)
2. "what time is it now?" while system is misconfigured → continuity should detect drift
3. "rename foo.txt to bar.txt" while model answers with file contents → continuity should flag mismatch
