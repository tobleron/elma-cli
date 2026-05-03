# 556 — Replace Hardcoded Keyword Matchers in execution_ladder/depth.rs

- **Priority**: High
- **Category**: Architecture
- **Depends on**: None
- **Blocks**: 562, 575

## Problem Statement

`src/execution_ladder/depth.rs` contains six functions that use hardcoded `str.contains()` keyword matching against user input:

```rust
pub fn requests_planning(user_message: &str) -> bool {
    let planning_indicators = [
        "step-by-step", "step by step", "give me a plan",
        "create a plan", "break down", "breakdown", ...
    ];
    planning_indicators.iter().any(|indicator| lower.contains(indicator))
}
```

Similar patterns exist in `requests_strategy()`, `requests_phases()`, `requests_bulk()`, `requests_multi_step_verbs()`, and `has_dependencies()`. Each function has 10-15 hardcoded indicator strings.

This directly violates AGENTS.md Rule 1: "Routing, classification, and behavior selection must never use hardcoded word triggers. If you find yourself writing `if input.contains("word")`, you are violating Elma's philosophy."

## Why This Matters for Small Local LLMs

The whole point of using model-based classification is to handle the fuzzy, contextual nature of user requests. Small models are explicitly supposed to handle this via intel units (complexity assessment, workflow planner). Having deterministic keyword matchers as escalation triggers means:

1. A small model might correctly assess a request as DIRECT, but keyword matchers force-escalate it to PLAN
2. The system doesn't trust its own model's classification — if the model can't be trusted, the whole pipeline is suspect
3. Keyword matchers are fragile: "Help me break down this crate" triggers `requests_planning` because it contains "break down", even though "break down this crate" means "unpack"

## Current Behavior

In `assess_execution_level()` (execution_ladder/mod.rs:289-322), these keyword matchers are used as escalation factors that can override the model's complexity assessment:

```rust
if explicit_planning_request {
    if level < ExecutionLevel::Plan {
        level = ExecutionLevel::Plan;
        escalation_factors.push("explicit planning request");
    }
}
```

## Recommended Target Behavior

Replace all keyword matchers with principle-based escalation:

1. **Trust the model's classification first**. The complexity assessment unit already determines DIRECT/INVESTIGATE/MULTISTEP/OPEN_ENDED. This should be the primary signal.

2. **Use feature vector signals instead of keyword matching**. The `ClassificationFeatures` struct already has `speech_act_probs`, `route_probs`, and `entropy`. Use these probabilistic signals for escalation decisions, not hardcoded strings.

3. **Make keyword heuristics soft hints for the intel unit, not hard escalation rules**. If certain patterns strongly correlate with planning requests, inject them as context into the complexity/planning intel unit prompt rather than making deterministic decisions.

4. **Remove all six keyword-matching functions** and replace with:
   - A single `assess_structural_complexity(message, features) -> EscalationHint` function
   - Uses `ClassificationFeatures` (entropy, margin, speech_act/route mismatch)
   - Optionally uses regex for true structural patterns (imperative mood, sequential connectors) — not word matching

## Source Files That Need Modification

- `src/execution_ladder/depth.rs:115-245` — Remove `requests_planning`, `requests_strategy`, `requests_phases`, `requests_bulk`, `requests_multi_step_verbs`, `has_dependencies`
- `src/execution_ladder/mod.rs:278-322` — Replace call sites with feature-vector-based escalation
- `src/execution_ladder/mod.rs:325-399` — Escalation logic that uses these predicates
- `src/execution_ladder/depth.rs:4` — Remove `pub use` re-exports of removed functions

## New Files/Modules

Optionally: `src/execution_ladder/escalation.rs` — New escalation logic based on feature vectors

## Step-by-Step Implementation Plan

1. Audit all call sites of the six keyword-matching functions (grep for each function name)
2. Verify that the `ClassificationFeatures` struct has all needed signals (speech_act, route, entropy, margin)
3. Add any missing signals to `ClassificationFeatures` if needed
4. Create new escalation logic in `execution_ladder/depth.rs` or new `escalation.rs`:
   ```rust
   fn assess_escalation_hints(
       features: &ClassificationFeatures,
       complexity: &ComplexityAssessment,
       route: &RouteDecision,
   ) -> Vec<String> {
       let mut hints = Vec::new();
       if features.entropy > 0.7 { hints.push("high_entropy".into()); }
       if route.margin < 0.2 { hints.push("low_margin".into()); }
       if complexity.risk == "HIGH" { hints.push("high_risk".into()); }
       // Check for speech act / route mismatches (already done, lines 349-377)
       hints
   }
   ```
5. Replace call sites in `assess_execution_level()` to use new logic
6. Remove the six keyword-matching functions
7. Remove `pub use` re-exports from `depth.rs`
8. Run full test suite
9. Run scenario tests to verify classification behavior is correct

## Recommended Crates

None new — use existing `ClassificationFeatures` and `RouteDecision` types.

## Validation/Sanitization Strategy

- The new escalation logic must be deterministic (not model-dependent) for reproducibility
- Feature vector thresholds should be configurable constants, not magic numbers
- Log all escalation decisions with rationale in trace output

## Testing Plan

1. Remove existing tests for keyword-matching functions
2. Add tests for new feature-vector-based escalation
3. Test that a message containing "break down" (but meaning "analyze a crate") does NOT trigger escalation
4. Test that high-entropy classifications DO trigger appropriate escalation
5. Test that low-margin decisions DO trigger appropriate escalation
6. Scenario test: "help me plan my day" → should NOT escalate to Plan level (it's a chat request)
7. Scenario test: "create a step-by-step migration strategy" → model should correctly classify as OPEN_ENDED

## Acceptance Criteria

- Zero `str.contains()` keyword matchers in `execution_ladder/` directory
- Complexity assessment still correctly escalates when the model is uncertain (low confidence, high entropy)
- AGENTS.md Rule 1 is no longer violated in the execution ladder
- All existing tests pass (after updating tests that tested keyword matchers)
- Scenario tests show correct behavior for planning vs non-planning requests

## Risks and Migration Notes

- **Behavior change risk**: The keyword matchers were added because the model's classification wasn't reliable enough. Removing them may cause the model to under-escalate. Mitigate by running scenario tests before/after and comparing.
- **Gradual removal**: Consider first making keyword hints advisory (injected into intel unit context) rather than deterministic, then remove after validation.
- The `requests_multi_step_verbs` function catches sequential language ("X then Y"). This is the hardest to replace with purely probabilistic signals. Consider using `needs_plan` from the action needs intel unit instead.
