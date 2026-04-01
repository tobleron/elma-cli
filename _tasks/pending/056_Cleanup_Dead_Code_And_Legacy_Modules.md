# Task 056: Cleanup Dead Code and Legacy Modules

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
