# Task 303: Implement Structured Classifier Interface For Routing Decisions

## Problem Analysis
Elma's current routing system relies on direct string comparisons and hardcoded heuristics that violate architectural principles. To enable more nuanced, model-driven routing decisions, we need to implement a structured classifier interface that provides rich classification results instead of simple binary choices.

## Solution Approach
Implement a structured classifier interface that:
1. Returns rich classification results with confidence scores, reasoning, and metadata
2. Replaces simple string-based decisions with structured data
3. Enables more sophisticated routing logic based on model confidence
4. Follows patterns seen in stress testing repositories (Claude Code's classifier systems)

## Implementation Plan

### Phase 1: Define Structured Classification Types
- [ ] Create `ClassificationResult` struct with fields:
  * `matches: bool` - whether the classification matches the target
  * `confidence: ConfidenceLevel` - enum (High/Medium/Low/VeryLow)
  * `reason: String` - human-readable explanation
  * `entropy: f64` - uncertainty measure
  * `metadata: Option<HashMap<String, String>>` - additional context
- [ ] Define `ConfidenceLevel` enum with variants
- [ ] Implement utility functions for working with classification results

### Phase 2: Create Classification Functions for Key Routing Decisions
- [ ] Implement `classify_speech_act(input: &str) -> ClassificationResult`
- [ ] Implement `classify_workflow_intent(input: &str) -> ClassificationResult`
- [ ] Implement `classify_mode_preference(input: &str) -> ClassificationResult`
- [ ] Each function should leverage existing model outputs (entropy/margin) when available

### Phase 3: Update Existing Routing Logic to Use Structured Classification
- [ ] Modify `infer_digit_router` to work with structured classification results
- [ ] Update `infer_route_prior` to consume structured classifications
- [ ] Replace direct string comparisons with confidence-based assessments
- [ ] Maintain backward compatibility where needed

### Phase 4: Integrate with Existing Infrastructure
- [ ] Ensure compatibility with existing `ProbabilityDecision` types
- [ ] Provide conversion functions between old and new systems
- [ ] Update any dependent code in routing_calc.rs and routing_parse.rs
- [ ] Verify integration with tool calling and program building systems

### Phase 5: Testing and Validation
- [ ] Create unit tests for all new classification functions
- [ ] Ensure existing tests continue to pass (adapt as needed)
- [ ] Add integration tests verifying structured classification improves routing accuracy
- [ ] Validate with real CLI testing using stress testing scenarios
- [ ] Confirm no performance regressions

## Success Criteria
- [ ] Structured classifier interface implemented and functional
- [ ] All routing decisions use classification results instead of hardcoded checks
- [ ] Confidence scores properly influence routing decisions
- [ ] All existing tests pass
- [ ] Real CLI validation shows improved or maintained routing accuracy
- [ ] Task can be marked as complete and moved to _tasks/completed/

## References
- AGENTS.md: Use Decomposition To Help Small Models principle
- AGENTS.md: Grounded Answers Only requirement
- Stress testing analysis: Claude Code's classifier-based systems in utils/permissions/
- Existing patterns: entropy/margin usage in src/routing_infer.rs
- Pattern: Structured results from json_error_handler/schemas.rs