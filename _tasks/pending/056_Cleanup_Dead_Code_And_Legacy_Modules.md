# Task 056: Cleanup Dead Code and Legacy Modules

## Priority
**P0 - FIRST IMPLEMENTATION TASK UNDER TASK 058**

## Status
**IN PROGRESS UNDER TASK 058**

## Masterplan Alignment
Task 058 reclassified this task as the first implementation target because reducing dead code and legacy paths lowers the risk and cost of the remaining stabilization work.

## Objective
Perform a systematic cleanup of unreachable code, legacy modules, and redundant logic identified during the architectural audit.

## Context
As the project has evolved (especially with the `_dev-system` guidance), several modules have been split or replaced, leaving behind "orphaned" code or redundant wrappers.

## Technical Details
- **Targets**:
    - **Legacy Wrappers**: Functions in `src/intel.rs` that simply call new `IntelUnit` implementations.
    - **Redundant Logic**: Compare `src/execution_steps_shell.rs` with `src/execution_steps_shell_exec.rs` for overlapping logic.
    - **Unused Types**: Audit `src/types_core.rs` for structs/enums that are no longer referenced in the new orchestration loop.
- **Requirements**:
    - Use `cargo check` and `unused_code` warnings as a starting point.
    - Manually verify that "Compatibility" layers (like `src/execution_steps_compat.rs`) are still necessary before removing.

## Verification
- `cargo build` (Zero warnings).
- Ensure no regression in existing tests or scenarios.
- Quantifiable reduction in total project LOC without loss of functionality.

## Progress Notes
- Managed `status_message_generator` is now loaded and synchronized like other canonical intel-unit profiles.
- Removed the inline shell-step prompt/profile construction so runtime execution no longer bypasses the canonical prompt registry for status messages.
- Removed the now-unused `default_status_message_generator_config` fallback constructor after migrating status-message loading to managed profile seeding and startup sync.
- Migrated the active `src/intel.rs` compatibility layer so its production calls now execute through trait-based intel units instead of raw ad hoc model-call helpers.
- Expanded `IntelContext` with structured extras and shared profile-request helpers so trait units can preserve legacy payload richness while converging on one execution model.
- Migrated direct runtime callers in planning, result presentation, selection, compaction, artifact classification, command repair, evidence-mode selection, and shell status generation onto trait units.
- Deleted `src/intel.rs` after confirming it was no longer on the live call graph; neutral compatibility context now lives in `src/intel_trait.rs`.
- Verified after this slice with `cargo build`, `cargo test`, and `./run_intention_scenarios.sh`.
