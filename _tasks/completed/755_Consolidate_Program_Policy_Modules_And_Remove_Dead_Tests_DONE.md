# Task 755: Consolidate Program Policy Modules And Remove Dead Tests

## Type

Architecture / De-bloating / Reliability

## Severity

Medium

## Scope

Program policy, step validation, execution

## Problem

The program policy system is fragmented across three modules:

1. **`src/program_policy.rs`** — listed in DEVELOPMENT_GUIDELINES as a "High-Risk Concentration Area"
2. **`src/program_policy_level.rs`** — policy level enforcement
3. **`src/program_policy_tests.rs`** — tests for program policy

Additionally, `src/types_core.rs` (also listed as high-risk) contains the `Step` enum and related types.

This fragmentation:
- Makes it hard to understand the full policy surface
- Causes policy logic to diverge (updates in one file may not reach others)
- Violates the de-bloating guidance: "create a sub-module directory with a mod.rs re-export"
- The tests in `program_policy_tests.rs` may test behavior that is no longer live

## Root Cause

Incremental development. Each new policy concern (allow/deny lists, level enforcement, tests) got its own file rather than being organized into a cohesive module.

## Proposed Solution

### Phase 1 — Audit all three modules

1. Read `program_policy.rs`, `program_policy_level.rs`, and `program_policy_tests.rs`
2. Map every function to its responsibility
3. Identify duplicates or overlapping logic
4. Identify dead tests (tests for functions no longer called)

### Phase 2 — Create `src/program_policy/` directory

```
src/program_policy/
  mod.rs              ← re-exports, 20 lines
  policy.rs           ← main policy logic (from program_policy.rs)
  levels.rs           ← level enforcement (from program_policy_level.rs)
  tests.rs            ← consolidated tests
```

### Phase 3 — Merge and delete old files

1. Move `program_policy.rs` content to `program_policy/policy.rs`
2. Move `program_policy_level.rs` content to `program_policy/levels.rs`
3. Move `program_policy_tests.rs` content to `program_policy/tests.rs`
4. Delete old files
5. Update `main.rs` module declarations

### Phase 4 — Audit `types_core.rs`

`types_core.rs` is 789 lines (not the 789 bytes I might have thought — wait, `wc -l` showed 789 lines). This is large for a types file. Evaluate:

1. Are all types in `types_core.rs` actually "core"?
2. Can `Step` enum move to `src/program_policy/step.rs`?
3. Can `StepResult` move to `src/execution/`?
4. Can `ChatMessage` move to `src/types_api.rs`?

If yes, extract and reduce `types_core.rs` to under 400 lines.

## Acceptance Criteria

- [ ] `src/program_policy.rs`, `program_policy_level.rs`, `program_policy_tests.rs` are deleted
- [ ] `src/program_policy/` directory exists with `mod.rs`, `policy.rs`, `levels.rs`, `tests.rs`
- [ ] No functionality lost
- [ ] `types_core.rs` is ≤ 400 lines (if extraction is feasible)
- [ ] `cargo build && cargo test` passes
- [ ] No dead tests remain

## Verification Plan

- `find src -name "program_policy*"` → only directory `src/program_policy/`
- `wc -l src/program_policy/*.rs` → each ≤ 400 lines
- `wc -l src/types_core.rs` → ≤ 400 lines
- `cargo test program_policy` → all tests pass

## Dependencies

- `src/types_core.rs` (Step enum)
- `src/execution_steps.rs` (step execution)
- `src/program.rs` (Program type)

## Notes

This is a pure refactor. Do not change policy behavior.

The `types_core.rs` extraction is optional — if it requires touching too many call sites, defer it to a follow-up task. The primary goal is consolidating the three policy files.

Check if `program_policy_tests.rs` contains tests for the keyword-based routing that Task 748 deletes. If so, those tests should be deleted, not migrated.
