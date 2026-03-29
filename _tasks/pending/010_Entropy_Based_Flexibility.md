# Task 010: Entropy-Based Flexibility for Routing

## Status
PENDING

## Problem
Route classifications have 100% confidence and zero entropy, making the system inflexible. Perfect confidence indicates pattern-matching, not reasoning.

## Evidence
From session trace:
```
route_dist=CHAT:1.00 SHELL:0.00 PLAN:0.00 MASTERPLAN:0.00 DECIDE:0.00
route=CHAT p=1.00 margin=1.00 entropy=0.00
```

## Goal
Add controlled uncertainty to classification outputs to encourage the orchestrator to actually weigh options rather than following deterministic paths.

## Implementation Steps

1. **Add entropy calculation to routing** (`src/routing.rs`):
   ```rust
   fn calculate_entropy(distribution: &[(String, f64)]) -> f64 {
       // Shannon entropy calculation
   }
   ```

2. **Add noise injection when entropy is too low**:
   ```rust
   if entropy < 0.1 {
       // Add small noise to distribution
       // Re-normalize
   }
   ```

3. **Update `infer_route_prior` to return entropy metadata**:
   - Add `entropy` field to `RouteDecision`
   - Pass to orchestrator as context

4. **Update orchestrator to use entropy**:
   - When entropy is low, prompt model to consider alternatives
   - Add to system prompt: "Consider alternative routes even if priors are confident"

## Acceptance Criteria
- [ ] Entropy is calculated for all classification outputs
- [ ] When entropy < 0.1, noise is added to encourage reasoning
- [ ] Route decisions include entropy metadata in traces
- [ ] Orchestrator prompts mention uncertainty when present

## Files to Modify
- `src/routing.rs` - Add entropy calculation and noise injection
- `src/types.rs` - Add entropy field to RouteDecision
- `src/orchestration.rs` - Use entropy in orchestrator prompts

## Priority
HIGH - Enables flexible reasoning

## Dependencies
- Task 009 (Harden OODA Loop) - Partially related
- None blocking
