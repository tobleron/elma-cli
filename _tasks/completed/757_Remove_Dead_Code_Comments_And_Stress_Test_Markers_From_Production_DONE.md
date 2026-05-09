# Task 757: Remove Dead Code Comments And Stress-Test Markers From Production

## Type

Code Quality / Reliability / Maintainability

## Severity

Medium

## Scope

All source files with dead code comments

## Problem

The codebase contains numerous dead code blocks, commented-out functions, and stress-test markers that clutter the code and mislead developers:

1. **`src/app_chat_patterns.rs`**:
   - Lines 81-123: large blocks of commented-out stress-test functions
   - Lines 138-148: more commented-out functions
   - Comment says "These functions kept for potential recipe migration but not called in production"

2. **`src/app_chat_loop.rs`**:
   - Lines 62-91: `apply_policy_fallback` contains dead code paths for `ExecutionLevel::Plan` and `ExecutionLevel::MasterPlan` that always return `None`
   - Lines 496-498: `apply_shape_fallbacks` is an empty function
   - Dozens of inline comments like `// Task 453 Category 1: Remove stress-test fallback policies`
   - Comments like `// Task 380: Create continuity tracker` on code that has been live for months

3. **`src/shell_preflight.rs`**:
   - Comments referencing tasks 116, 118, 119, 120 on code that is long since complete

4. **Throughout `src/`**:
   - `// Task XXX:` comments on code that is complete
   - `// Legacy constants absorbed into StopPolicy` in `tool_loop.rs`
   - `// Task 453 Category 1: Delete stress-test shape fallbacks` on empty functions

These markers:
- Clutter the code and reduce readability
- Mislead new developers into thinking the code is temporary or experimental
- Violate Rule 13: "If logic has been 'repaired' 3 times, it is architecturally unsound"
- Violate DEVELOPMENT_GUIDELINES: "Delete-First Policy: Prefer removing obsolete abstractions over patching failing ones"

## Root Cause

Tasks were implemented with inline comments marking the change. When tasks were completed, the comments were not removed. Stress-test code was commented out rather than deleted.

## Proposed Solution

### Phase 1 — Audit all files for dead markers

Search for patterns:
```bash
grep -rn "Task 453\|stress-test\|Category 1\|kept for potential" src/
grep -rn "^\s*//\s*pub(crate) fn\|^\s*/\*\s*pub(crate) fn" src/
grep -rn "Legacy constants\|absorbed into" src/
```

### Phase 2 — Delete commented-out code

1. In `app_chat_patterns.rs`: delete all commented-out functions (lines 81-123, 138-148)
2. In `app_chat_loop.rs`: delete empty `apply_shape_fallbacks`, dead `apply_policy_fallback` branches
3. In `tool_loop.rs`: delete legacy constant comments
4. Anywhere else: delete commented-out code blocks

### Phase 3 — Clean up task comments

For completed tasks:
1. Remove `// Task XXX:` comments that describe completed work
2. Keep `// Task XXX:` comments only for:
   - Active or recently completed tasks (last 30 days)
   - Tasks that describe ongoing behavior (not implementation)
   - Tasks that link to complex decisions that need context

For example:
- KEEP: `// Task 380: Semantic continuity tracking` (describes ongoing system behavior)
- DELETE: `// Task 453 Category 1: Remove stress-test fallback policies` (describes a completed deletion)
- DELETE: `// Task 114: Auto-Compact` on `mod auto_compact` (module declaration is self-evident)

### Phase 4 — Add lint or policy

In `DEVELOPMENT_GUIDELINES.md`, add:

```markdown
## Code Comment Policy

- Do not leave commented-out code in production files
- Delete code rather than commenting it out
- Task comments should be removed when the task is archived
- Use `// TODO(#task_number): description` for active work only
- Empty functions with "placeholder" comments should be deleted
```

## Acceptance Criteria

- [ ] No commented-out function definitions remain in `src/`
- [ ] No "stress-test" or "Category 1" markers remain
- [ ] No empty functions with placeholder comments remain
- [ ] Task comments describe ongoing behavior, not completed work
- [ ] `cargo build && cargo test` passes
- [ ] `cargo fmt` passes

## Verification Plan

- `grep -rn "^\s*//\s*pub(crate) fn\|^\s*/\*\s*pub(crate) fn" src/` → no matches
- `grep -rn "stress-test\|Category 1: Remove" src/` → no matches
- `grep -rn "Legacy constants absorbed" src/` → no matches
- `cargo build` → success

## Dependencies

- All source files (mechanical cleanup)
- `docs/DEVELOPMENT_GUIDELINES.md` (policy addition)

## Notes

This is a **mechanical cleanup** task. The risk is low because we're only deleting comments and commented-out code, not changing behavior.

Use `sed` or a script for bulk deletion of common patterns.

After cleanup, the codebase should be readable without wading through historical artifacts. The git history preserves the task context; inline comments are not needed.
