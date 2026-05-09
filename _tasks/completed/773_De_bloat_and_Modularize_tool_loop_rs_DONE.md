# Task 773: De-bloat and Modularize tool_loop.rs

## Type
Refactor / Hardening

## Severity
High

## Scope
System-wide / Module

## Problem
`src/tool_loop.rs` has grown to over 4000 lines. It contains a mix of high-level orchestration, streaming model interaction, tool execution logic, finalization logic, and compaction management. This violates the project's "Module De-Bloating" guideline and makes the core execution path difficult to maintain and test.

Furthermore, it contains blocking filesystem operations (e.g., `std::fs::write`) inside `async` functions, which can cause stalls in the async runtime.

## Root Cause
Incremental expansion of the tool loop without periodic refactoring. Logic that should belong to separate modules (like `finalization`, `compaction`, or `streaming_client`) has been accumulated in the main loop.

## Proposed Solution
Decompose `src/tool_loop.rs` into a modular hierarchy.

- Phase 1: Extract streaming model turn logic into `src/tool_loop_streaming.rs`.
- Phase 2: Extract finalization and "evidence-to-answer" logic into `src/tool_loop_finalization.rs`.
- Phase 3: Extract iteration budget and stop policy enforcement into a more robust `src/stop_policy.rs` (expanding on existing logic).
- Phase 4: Modularize the main loop in `src/tool_loop/mod.rs` (or similar structure) using a façade pattern.
- Phase 5: Audit all `std::fs` calls and replace them with `tokio::fs` or wrap in `spawn_blocking`.

## Acceptance Criteria
- [ ] `src/tool_loop.rs` is reduced to < 1000 lines (target 500 lines for the main dispatcher).
- [ ] Logic is split into clear, single-responsibility sub-modules.
- [ ] No blocking I/O calls remain in the async path.
- [ ] All existing functionality (streaming, tool calling, finalization) is preserved and verified.

## Verification Plan
- Unit test: `cargo test tool_loop` (verify all sub-modules).
- Integration test: Run a multi-step task and verify the loop behaves identically to the pre-refactor state.
- Performance: Verify that the async runtime is not blocked (via trace logs or metrics).

## Dependencies
None

## Notes
- Follow the pattern established in the recent `tool_calling.rs` refactor.
- Ensure `AppRuntime` access is clean across the new modules.
