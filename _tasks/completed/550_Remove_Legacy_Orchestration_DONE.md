# 001 — Remove or Fully Deprecate Legacy Maestro Orchestration Pipeline

- **Priority**: Critical
- **Category**: Architecture
- **Depends on**: None
- **Blocks**: 009, 012, 017

## Problem Statement

The codebase maintains two parallel orchestration systems:

1. **New tool-calling pipeline** (`orchestration_core::run_tool_calling_pipeline` → `tool_loop::run_tool_loop`): Direct model → tool execution, uses `prompt_core::TOOL_CALLING_SYSTEM_PROMPT`, is the active path.

2. **Legacy Maestro pipeline** (`orchestration_core::build_program_from_maestro` → `orchestrate_instruction_once`): Calls a "Maestro" intel unit to generate text instructions, then calls an "Orchestrator" intel unit to transform each instruction into JSON `Step` objects, then executes those steps via a separate `Program` execution path.

The legacy pipeline (~600 lines in `orchestration_core.rs`, plus `orchestration_planning.rs` ~1034 lines, `orchestration_loop.rs` ~623 lines, and `orchestration_retry.rs` ~1079 lines) pulls in:
- Hardcoded English capability strings (lines 186-199 in `orchestration_core.rs`) that haven't been updated with newer tools
- Fixed JSON schemas for `Step` types that differ from the tool calling schema
- Separate retry logic in `orchestration_retry.rs`
- Separate critic/final-answer generation in `orchestration_loop_*.rs`

## Why This Matters for Small Local LLMs

- Each additional orchestration path doubles the surface area for model confusion
- The legacy path's hardcoded capability strings don't match current tool schemas, so if the system ever falls back to this path, small models will get contradictory tool information
- Code divergence means fixes applied to one pipeline aren't applied to the other
- Small models benefit from a single, consistent execution model

## Current Behavior

The legacy pipeline is accessible via `build_program_from_maestro()` and `orchestrate_instruction_once()` but the active call path in `app_chat_orchestrator.rs` appears to use the new pipeline. The old functions remain compiled and available as public API.

## Recommended Target Behavior

One of three options (choose one):
1. **Full removal**: Delete `build_program_from_maestro`, `orchestrate_instruction_once`, `orchestrate_program_once`, `recover_program_once`, `run_critic_once`, `generate_final_answer_once`, `judge_final_answer_once`, and all supporting types from `orchestration_core.rs`. Remove `orchestration_planning.rs`, `orchestration_loop.rs`, `orchestration_retry.rs`, `orchestration_loop_reviewers.rs` (dead declaration), `orchestration_loop_verdicts.rs` (dead declaration), `orchestration_retry_tests.rs` (dead declaration).
2. **Feature-gated deprecation**: Wrap all legacy code behind a `#[cfg(feature = "legacy_orchestration")]` and remove from default compilation.
3. **Refactor to compatibility layer**: Keep only the `Program` / `Step` / `StepResult` types (which are used elsewhere) and remove the orchestration code.

## Source Files That Need Modification

- `src/orchestration_core.rs` — Remove legacy functions (lines 163-end)
- `src/orchestration_planning.rs` — Remove entire file or feature-gate
- `src/orchestration_loop.rs` — Remove entire file or feature-gate
- `src/orchestration_retry.rs` — Remove entire file or feature-gate
- `src/orchestration_helpers/` — Audit for legacy-only functions
- `src/main.rs` — Remove dead `mod` declarations for `orchestration_loop_helpers`, `orchestration_loop_reviewers`, `orchestration_loop_verdicts`
- `src/types_core.rs` — Keep `Program`, `Step`, `StepResult`, `StepCommon` types; remove legacy-only types
- `src/program.rs` — Audit for legacy-only functions
- `src/program_policy.rs` — Audit for legacy-only functions
- `src/program_steps.rs` — Audit for legacy-only functions
- `src/program_utils.rs` — Keep `run_shell_persistent_sync` (used by shell_preflight)
- `src/execution.rs` — Audit for legacy execution path
- `Cargo.toml` — Remove any dependencies used ONLY by legacy path

## Step-by-Step Implementation Plan

1. Map all call sites of `build_program_from_maestro`, `orchestrate_instruction_once`, `orchestrate_program_once`, `recover_program_once`, `run_critic_once`, `generate_final_answer_once`, `judge_final_answer_once`
2. Verify none are on any active code path (add temporary `unreachable!()` calls to confirm)
3. Extract and preserve the `Step`, `Program`, `StepResult` type definitions if needed elsewhere
4. Remove the legacy orchestration functions from `orchestration_core.rs`
5. Remove `orchestration_planning.rs`, `orchestration_loop.rs`, `orchestration_retry.rs`
6. Remove dead `mod` declarations from `main.rs`
7. Audit `orchestration_helpers/` and remove legacy-only helpers
8. Run full test suite to verify no breakage
9. Run scenario tests to verify tool-calling pipeline still works
10. Remove any dependencies that become unused

## Recommended Crates

None — this is a removal task.

## Validation/Sanitization Strategy

- Before removing each function, grep for all references across the entire codebase
- Add `#[deprecated]` annotations for one commit cycle before removal
- Verify with `cargo check` and `cargo test` after each removal step

## Testing Plan

1. Run existing test suite: `cargo test`
2. Run scenario tests in `scenarios/` directory
3. Verify shell execution pipeline still works
4. Verify tool calling pipeline still works
5. Check that no warnings about dead code remain

## Acceptance Criteria

- Legacy orchestration functions are either removed or feature-gated behind `legacy_orchestration`
- Dead `mod` declarations are removed from `main.rs`
- All existing tests pass
- The tool-calling pipeline executes correctly in manual smoke test
- No compilation warnings about unused code from removed modules

## Risks and Migration Notes

- **Risk**: Some legacy types (`Program`, `Step`) may be used by the task persistence system (`task_persistence.rs`, `runtime_task.rs`). Audit carefully before removing type definitions.
- **Risk**: The `generate_final_answer_once` function may be used by the evaluation/calibration system. Check `evaluation.rs`, `evaluation_workflow.rs`.
- **Mitigation**: Do this in stages — first remove the orchestration functions, keep types, then clean up types in a follow-up PR.
- Verify that `orchestration_helpers::request_response_advice_via_unit` and `present_result_via_unit` are used by active paths before touching `orchestration_helpers/`.
