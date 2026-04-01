# Task 055: Refine Drag Formula Weights

## Objective
Analyze and refine the mathematical "Drag" formula in the `_dev-system` to better reflect the cognitive load of Rust modules.

## Context
The current Drag formula (v2.0) uses weights for Nesting, Logic Density, and State Density. Based on project analysis, **State Density** is the most significant contributor to "context fog" for both human developers and AI agents.

## Technical Details
- **File**: `_dev-system/config/efficiency.json` (or where the weights are defined).
- **Current Weights** (from `ARCHITECTURE.md`):
    - Nesting: 0.6
    - Density: 1.0
    - StateDensity: 8.0
- **Proposed Change**: 
    - Evaluate increasing `StateDensity` weight to 10.0 or 12.0.
    - Re-evaluate the `FailurePenalty` cap to ensure it correctly triggers refactor tasks for files with repeated regression issues.
- **Goal**: Ensure the analyzer more aggressively flags state-heavy modules for surgical refactoring.

## Verification
- Run the `_dev-system/analyzer`.
- Observe if it generates new refactor tasks for the most complex files (e.g., `src/app_bootstrap_profiles.rs` or `src/intel_units.rs`) that were previously just below the threshold.
