# 006 — Create or Remove Missing Module Declarations

- **Priority**: High
- **Category**: Architecture
- **Depends on**: 001
- **Blocks**: None

## Problem Statement

`src/main.rs` declares four modules that have no corresponding source files:

```rust
mod orchestration_loop_helpers;  // line 119 — no file exists
mod orchestration_loop_reviewers; // line 120 — no file exists
mod orchestration_loop_verdicts;  // line 121 — no file exists
mod orchestration_retry_tests;    // this one ALSO doesn't exist
```

These are dead declarations that:
1. Will cause compilation errors if anyone tries to reference these modules
2. Indicate incomplete refactoring or abandoned feature work
3. Add noise to the module graph

Additionally, `orchestration_retry_tests.rs` was declared separately from `orchestration_retry.rs` but never created, suggesting a plan to split tests that was never completed.

## Why This Matters for Small Local LLMs

Dead module declarations don't directly affect the model, but they indicate architectural drift. A clean module graph makes it easier to understand and modify the system — which is critical for a codebase that needs frequent tuning for small-model behavior.

## Current Behavior

The `mod` declarations compile because Rust allows module declarations without corresponding files as long as no code references them. They're invisible at runtime but visible in the source tree.

## Recommended Target Behavior

Option A (if Task 001 removes legacy orchestration):
- Remove all four `mod` declarations as part of the legacy cleanup

Option B (if keeping orchestration):
- Create the missing files with appropriate content
- `orchestration_loop_helpers.rs` — helper functions extracted from `orchestration_loop.rs`
- `orchestration_loop_reviewers.rs` — reviewer/critic logic extracted from loop
- `orchestration_loop_verdicts.rs` — verdict types extracted from loop
- `orchestration_retry_tests.rs` — tests extracted from `orchestration_retry.rs`

## Source Files That Need Modification

- `src/main.rs:119-121` — Remove or implement `mod` declarations
- `src/orchestration_loop.rs` — Extract helpers if creating the files
- `src/orchestration_retry.rs` — Extract tests if creating test file

## New Files/Modules (if choosing Option B)

- `src/orchestration_loop_helpers.rs`
- `src/orchestration_loop_reviewers.rs`
- `src/orchestration_loop_verdicts.rs`
- `src/orchestration_retry_tests.rs`

## Step-by-Step Implementation Plan

### Option A (recommended — pairs with Task 001)
1. Verify the four modules are not referenced anywhere (grep for `orchestration_loop_helpers`, etc.)
2. Remove the four `mod` declarations from `main.rs`
3. Run `cargo check` to verify no breakage
4. Done

### Option B
1. Analyze `orchestration_loop.rs` and `orchestration_retry.rs` to identify extractable helpers, reviewers, verdicts, and tests
2. Create each file with appropriate content
3. Update imports in original files
4. Run `cargo check` and `cargo test`

## Validation/Sanitization Strategy

- Before removal: `rg "orchestration_loop_helpers|orchestration_loop_reviewers|orchestration_loop_verdicts" src/` to find all references
- After removal: `cargo check` must succeed

## Testing Plan

1. Run `cargo test` before and after changes
2. No test failures introduced

## Acceptance Criteria

- No dead `mod` declarations in `main.rs`
- `cargo check` passes
- `cargo test` passes

## Risks and Migration Notes

- **Low risk**: Dead declarations have no runtime effect
- If choosing Option A, pair with Task 001 to avoid leaving orphaned functions
- If choosing Option B, ensure the extracted modules are actually used somewhere
