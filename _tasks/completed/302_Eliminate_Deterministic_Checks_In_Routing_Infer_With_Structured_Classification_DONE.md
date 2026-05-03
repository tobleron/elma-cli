# Task 302: Eliminate Deterministic Checks In Routing_Infer With Structured Classification

## Problem Analysis
The file `src/routing_infer.rs` contains multiple instances of hardcoded routing logic that violates Elma's architectural principles:

### Hardcoded String Comparisons (Violates AGENTS.md Rule #1):
- **Lines 11, 14, 20**: `speech_act.choice.eq_ignore_ascii_case("CHAT")` and `workflow.choice.eq_ignore_ascii_case("CHAT")`
- **Lines 198, 266, 268**: Similar direct string comparisons for "CHAT" and "INSTRUCT" choices
- **Lines 333, 336, 339**: Confidence checks based on hardcoded labels (`WORKFLOW`, `CHAT`, `INSTRUCT`)
- **Lines 381-383**: Hardcoded keyword list `["identify", "choose", "select", "which"]` for routing decisions

These deterministic checks bypass model reasoning and create brittle routing logic that doesn't adapt to nuanced user intent.

## Solution Approach
Replace hardcoded deterministic checks with structured classification approaches that:
1. Use model-provided confidence scores (entropy/margin) for decisions
2. Implement structured classification results instead of binary true/false checks
3. Remove hardcoded keyword lists in favor of model-driven intent detection
4. Maintain all existing functionality while improving architectural compliance

## Implementation Plan

### Phase 1: Replace Direct String Comparisons
- [ ] Replace `choice.eq_ignore_ascii_case("CHAT")` checks with confidence-based assessments
- [ ] Use entropy and margin scores from ProbabilityDecision to make routing decisions
- [ ] Create helper functions for confidence-based choice evaluation

### Phase 2: Eliminate Hardcoded Keyword Lists
- [ ] Remove lines 381-383: `let identify_request = ["identify", "choose", "select", "which"]`
- [ ] Replace with model-based detection using speech_act/workflow classifications
- [ ] Leverage existing intent detection from speech_act classifications

### Phase 3: Improve Confidence-Based Decision Making
- [ ] Replace hardcoded threshold values (0.20, 0.70, 0.15, 0.50, etc.) with configurable parameters
- [ ] Implement structured decision functions that consider multiple confidence factors
- [ ] Create reusable utilities for common routing decision patterns

### Phase 4: Refactor Short-Circuit Logic
- [ ] Replace `should_short_circuit_chat_route` function with confidence-based alternative
- [ ] Replace `should_apply_speech_chat_boost` with structured assessment
- [ ] Ensure all routing decisions consider full context rather than isolated checks

### Phase 5: Testing and Validation
- [ ] Ensure all existing tests pass (update test expectations as needed)
- [ ] Add tests verifying confidence-based decision making
- [ ] Verify with real CLI validation using ui_parity_probe.sh
- [ ] Confirm routing behavior remains correct for all scenarios

## Success Criteria
- [ ] All deterministic string comparisons removed from routing_infer.rs
- [ ] Routing decisions based on model confidence metrics (entropy/margin)
- [ ] No hardcoded keyword lists for intent detection
- [ ] All existing tests pass
- [ ] Real CLI validation shows no regressions in routing accuracy
- [ ] Task can be marked as complete and moved to _tasks/completed/

## References
- AGENTS.md: Non-Negotiable Architecture Rules #1 (No Word-Based Routing)
- AGENTS.md: Use Decomposition To Help Small Models principle
- Stress testing analysis: Claude Code's classifier-based systems
- Existing patterns in src/routing_infer.rs: Already uses entropy/margin in some places