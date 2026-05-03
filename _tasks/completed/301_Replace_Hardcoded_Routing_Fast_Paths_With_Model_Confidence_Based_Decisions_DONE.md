# Task 301: Replace Hardcoded Routing Fast Paths With Model Confidence Based Decisions

## Problem Analysis
The file `src/app_chat_fast_paths.rs` contains hardcoded routing logic that violates Elma's architectural principles:

1. **Function `looks_like_literal_shell_command`** uses deterministic heuristics:
   - Line 21-22: Rejects commands ending with punctuation (.!?)
   - Line 27-33: Rejects commands if first character is uppercase
   - These are word/character-based triggers that bypass model reasoning

2. **Function `should_use_direct_shell_fast_path`** makes routing decisions based on these hardcoded checks rather than model confidence metrics

This violates AGENTS.md Non-Negotiable Architecture Rule #1: "Never implement routing, classification, or behavior selection through hardcoded word triggers."

## Solution Approach
Replace hardcoded routing heuristics with model confidence-based decisions using:
1. Entropy and margin scores from routing classifications
2. Feature flags for safe migration
3. Structured decision-making that considers all factors collectively

## Implementation Plan

### Phase 1: Prepare Infrastructure
- [ ] Add confidence scoring types to support structured decisions
- [ ] Create feature flag for enabling/disabling new routing logic
- [ ] Define interfaces for confidence-based routing assessment

### Phase 2: Replace looks_like_literal_shell_command
- [ ] Remove hardcoded punctuation check (lines 21-22)
- [ ] Remove hardcoded uppercase first character check (lines 27-33)
- [ ] Replace with assessment based on:
  * Route decision confidence scores
  * Model entropy/margin metrics
  * Workflow complexity assessment

### Phase 3: Update should_use_direct_shell_fast_path
- [ ] Modify function to use confidence-based assessments instead of hardcoded checks
- [ ] Maintain existing safety checks (command_exists, program_safety_check, command_is_readonly)
- [ ] Integrate with feature flag for gradual rollout

### Phase 4: Update should_use_direct_reply_fast_path
- [ ] Evaluate if similar hardcoded logic exists that should be replaced
- [ ] Apply same confidence-based principles if needed

### Phase 5: Testing and Validation
- [ ] Ensure all existing tests pass
- [ ] Add tests verifying confidence-based decision making
- [ ] Verify with real CLI validation using ui_parity_probe.sh
- [ ] Confirm no regression in direct execution performance for appropriate cases

## Success Criteria
- [ ] All deterministic word/character triggers removed from fast path logic
- [ ] Routing decisions based on model confidence metrics (entropy/margin)
- [ ] Feature flag allows safe rollback if issues detected
- [ ] All existing tests pass
- [ ] Real CLI validation shows no regressions
- [ ] Task can be marked as complete and moved to _tasks/completed/

## References
- AGENTS.md: Non-Negotiable Architecture Rules #1 (No Word-Based Routing)
- src/routing_infer.rs: Existing pattern of using entropy/margin for decisions
- Stress testing analysis: Claude Code's classifier-based systems