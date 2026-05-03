# Task 304: Add Feature Flagged Classification System For Safe Migration

## Problem Analysis
When replacing hardcoded routing logic with model-confidence-based decisions, we need a safe migration strategy that allows:
1. Gradual rollout of new classification systems
2. Easy rollback if issues are detected
3. A/B testing capabilities to compare old vs new approaches
4. Configuration-based enabling/disabling without code changes

This follows the pattern seen in stress testing repositories like Claude Code, which uses feature flags to safely enable/disable classification systems.

## Solution Approach
Implement a feature flag system for classification routing decisions that:
1. Allows enabling/disabling new classification logic via configuration
2. Provides fallback to existing hardcoded logic when new system is disabled
3. Supports gradual rollout percentages for A/B testing
4. Includes monitoring and logging capabilities to track usage

## Implementation Plan

### Phase 1: Create Feature Flag Infrastructure
- [ ] Add feature flag definitions in config/ or constants/
- [ ] Create `FeatureFlags` struct with routing-related flags:
  * `use_structured_classifier`: boolean for new classification system
  * `use_confidence_based_fast_paths`: boolean for fast path changes
  * `classifier_rollout_percentage`: u8 for gradual rollout (0-100)
- [ ] Implement feature flag retrieval from configuration/environment
- [ ] Add logging for feature flag evaluations

### Phase 2: Implement Feature-Gated Classification Functions
- [ ] Wrap new structured classifier functions with feature flag checks:
  ```rust
  if feature_flags.use_structured_classifier {
      // Use new structured classification
  } else {
      // Fallback to existing logic
  }
  ```
- [ ] Implement percentage-based rollout for testing:
  ```rust
  if should_use_new_classifier(user_id, feature_flags.rollout_percentage) {
      // Use new system
  } else {
      // Use old system
  }
  ```
- [ ] Ensure all new classification work from Tasks 301-303 respects feature flags

### Phase 3: Update Configuration System
- [ ] Add routing feature flags to config template files
- [ ] Document available flags and their effects
- [ ] Ensure defaults maintain existing behavior (flags disabled by default)
- [ ] Add validation for flag values

### Phase 4: Integrate with Existing Routing Logic
- [ ] Modify `app_chat_fast_paths.rs` to use feature flags for fast path decisions
- [ ] Update `routing_infer.rs` to use feature flags for classification decisions
- [ ] Ensure all routing decision points can leverage the feature flag system
- [ ] Maintain backward compatibility when flags are disabled

### Phase 5: Testing, Monitoring, and Validation
- [ ] Create tests for feature flag functionality
- [ ] Add tests for percentage-based rollout logic
- [ ] Verify that disabling flags results in identical behavior to baseline
- [ ] Test with real CLI validation using various flag configurations
- [ ] Add monitoring/logging to track feature flag usage in production
- [ ] Ensure no performance degradation from feature flag checks

## Success Criteria
- [ ] Feature flag system implemented and configurable
- [ ] New classification logic can be enabled/disabled without code changes
- [ ] Percentage-based rollout works correctly for A/B testing
- [ ] All existing tests pass with flags disabled (backward compatibility)
- [ ] New functionality testable with flags enabled
- [ ] Real CLI validation shows no regressions when flags disabled
- [ ] Task can be marked as complete and moved to _tasks/completed/

## References
- AGENTS.md: Local-First, Offline-First principle (configuration should work offline)
- Stress testing analysis: Claude Code's feature flag usage in utils/
- Existing patterns: Feature usage in claude-code utils/classifierApprovals.ts
- Configuration patterns: Existing config/ directory and defaults_*.rs files