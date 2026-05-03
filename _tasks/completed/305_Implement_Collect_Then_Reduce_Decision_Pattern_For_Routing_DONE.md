# Task 305: Implement Collect-Then-Reduce Decision Pattern For Routing

## Problem Analysis
Elma's current routing logic in `src/routing_infer.rs` uses early returns and sequential decision-making that can bypass important security and correctness checks. This creates vulnerabilities where an early 'ask' decision can mask a later 'deny' decision, violating the principle that deny should take precedence over ask.

The stress testing analysis revealed that Claude Code uses a 'collect-then-reduce' pattern where:
1. All decisions are collected first (deny, ask, allow, passthrough)
2. A single reduction process applies precedence rules (deny > ask > allow > passthrough)
3. This structurally prevents decision bypass vulnerabilities

## Solution Approach
Replace sequential early-return routing logic with a collect-then-reduce pattern that:
1. Gathers all potential routing decisions into a collection
2. Applies a consistent precedence hierarchy to reduce to a final decision
3. Ensures higher precedence decisions (deny/safety) always override lower ones (ask/convenience)
4. Makes the decision-making process transparent and auditible

## Implementation Plan

### Phase 1: Define Decision Types and Precedence
- [ ] Create `RoutingDecisionType` enum with variants: Deny, Ask, Allow, Passthrough
- [ ] Define precedence hierarchy: Deny > Ask > Allow > Passthrough
- [ ] Create `RoutingDecision` struct containing type, message, reasoning, and metadata
- [ ] Implement reduction function that applies precedence rules

### Phase 2: Refactor Speech Act Classification
- [ ] Modify speech act inference to collect rather than early-return
- [ ] Replace direct returns with decision collection
- [ ] Ensure all speech act classifications contribute to decision pool

### Phase 3: Refactor Workflow Classification
- [ ] Modify workflow inference to use collect-then-reduce pattern
- [ ] Replace early returns with decision collection
- [ ] Ensure workflow classifications are properly prioritized

### Phase 4: Refactor Mode Classification
- [ ] Modify mode inference to use collect-then-reduce pattern
- [ ] Replace direct returns with decision collection
- [ ] Ensure mode decisions follow precedence rules

### Phase 5: Update Final Route Determination
- [ ] Replace the complex if/else logic for route determination with decision reduction
- [ ] Ensure final route selection respects the collected decision precedence
- [ ] Maintain all existing functionality while improving decision safety

### Phase 6: Testing and Validation
- [ ] Ensure all existing tests pass (update as needed for new decision flow)
- [ ] Add tests specifically verifying decision precedence works correctly
- [ ] Create tests that attempt to bypass security checks via early returns (should fail)
- [ ] Validate with real CLI testing using ui_parity_probe.sh
- [ ] Confirm no regressions in routing accuracy or performance

## Success Criteria
- [ ] All routing decisions use collect-then-reduce pattern instead of early returns
- [ ] Decision precedence is consistently applied (deny > ask > allow > passthrough)
- [ ] No possibility for early returns to bypass safety or correctness checks
- [ ] All existing tests pass
- [ ] Real CLI validation shows maintained or improved routing accuracy
- [ ] Task can be marked as complete and moved to _tasks/completed/

## References
- AGENTS.md: Reliability Over Speed principle (prefer correct decisions over fast ones)
- AGENTS.md: Use Decomposition To Help Small Models principle
- Stress testing analysis: Claude Code's collect-then-reduce pattern in bashPermissions.ts
- Existing patterns: Some decision collection already exists in routing_infer.rs lines 900-930
- Pattern: Decision reasoning collection in src/execution_steps_shell.rs