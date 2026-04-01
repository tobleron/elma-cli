# Task 052: Complete Intel Migration and Cleanup

## Objective
Finalize the migration of all reasoning logic from the legacy `src/intel.rs` to the new trait-based `IntelUnit` system in `src/intel_units.rs`. 

## Context
The project has successfully introduced a robust `IntelUnit` trait (in `src/intel_trait.rs`) which provides a standardized lifecycle (Pre-flight, Execution, Post-flight, Fallback). However, several legacy functions in `src/intel.rs` still exist, either as redundant implementations or as wrappers that haven't been fully refactored into the new system.

## Technical Details
- **Source**: `src/intel.rs`
- **Target**: `src/intel_units.rs`
- **Key Symbols to Migrate**:
    - `generate_status_message_once`
    - `assess_complexity_once`
    - `assess_evidence_needs_once`
    - `assess_workflow_plan_once`
    - Any remaining OODA-loop style logic that hasn't been unified.
- **Requirements**:
    - Each migrated unit must implement `IntelUnit`.
    - Ensure `IntelContext` is used consistently.
    - Maintain existing fallback behaviors for local model resilience.
    - Update `src/orchestration_planning.rs` and other callers to use the new trait-based units exclusively.

## Verification
- `cargo build` (Zero warnings)
- `./run_intention_scenarios.sh` to ensure reasoning quality is maintained or improved.
- Verify that `src/intel.rs` can be significantly reduced or removed entirely once all callers are updated.
