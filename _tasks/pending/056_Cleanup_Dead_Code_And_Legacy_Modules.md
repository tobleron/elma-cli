# ⏸️ POSTPONED

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

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
