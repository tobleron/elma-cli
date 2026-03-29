# Task 013: Decouple Classification from Execution

## Status
PENDING

## Problem
Classifications (speech-act, workflow, mode, route) are treated as hard decisions, not soft features. This prevents the orchestrator from reasoning about alternatives.

## Current Flow (Problematic):
```
User Input → Classifiers → Route Decision → Formula → Program → [Guard] → Execute
                                      (hard decisions)
```

## Recommended Flow:
```
User Input → Classifiers → Feature Vector → Orchestrator (with reasoning) → Program → Execute
                                                      ↑
                                              Priors as soft features
```

## Goal
Treat classification outputs as probabilistic features for the orchestrator, not deterministic constraints.

## Implementation Steps

1. **Create feature vector structure** in `src/types.rs`:
   ```rust
   pub struct ClassificationFeatures {
       pub speech_act_probs: Vec<(String, f64)>,
       pub workflow_probs: Vec<(String, f64)>,
       pub mode_probs: Vec<(String, f64)>,
       pub route_probs: Vec<(String, f64)>,
       pub entropy: f64,
   }
   ```

2. **Update routing functions** to return features, not decisions:
   ```rust
   // Old
   pub fn infer_route_prior(...) -> RouteDecision;
   
   // New
   pub fn extract_route_features(...) -> ClassificationFeatures;
   ```

3. **Update orchestrator prompt** to receive features:
   ```
   Classification Features (use as evidence, not rules):
   - Speech act: CAPABILITY_CHECK (85%), INFO_REQUEST (10%), ACTION_REQUEST (5%)
   - Workflow: CHAT (60%), WORKFLOW (40%)
   - Route: CHAT (70%), SHELL (20%), PLAN (10%)
   
   These are probabilistic signals. Reason about whether they apply to this specific request.
   ```

4. **Remove formula-based program generation**:
   - Currently: `build_program(formula, ...)` selects template
   - New: `orchestrator_reasoning(features, workspace, user_input)` generates freely

5. **Update all routing call sites**:
   - `src/app_chat.rs`
   - `src/orchestration.rs`
   - `src/evaluation_workflow.rs`
   - `src/tune_scenario.rs`

## Acceptance Criteria
- [ ] Classifiers return probability distributions, not single choices
- [ ] Orchestrator receives features as context, not decisions
- [ ] Formula-based program generation is removed
- [ ] Model can override priors when appropriate
- [ ] Traces show feature distributions

## Files to Modify
- `src/routing.rs` - Return features instead of decisions
- `src/orchestration.rs` - Use features in orchestrator
- `src/types.rs` - Add ClassificationFeatures struct
- `src/defaults.rs` - Update orchestrator prompt

## Priority
VERY HIGH - Fundamental architecture change

## Dependencies
- Task 010 (Entropy-Based Flexibility) - Related
- Task 011 (Iterative Refinement) - Independent
