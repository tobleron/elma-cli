# Task 758: Deferred Wire Or Delete Unwired Recipe And Formula Pattern Dead Code

## Deferred Status

Deferred during pending queue review on 2026-05-07.

Reason: this task's delete recommendation is not currently safe enough for the active pending queue. Code search shows `FormulaPattern` is imported by `orchestration_core.rs` and `tuning_support.rs`, and recipe-related UI events still exist. The task may still represent real documentation or dead-code drift, but deleting recipes/formula patterns as written could remove useful tuning or compatibility surfaces before the live path is fully proven. Revisit after Task 751 and Task 756 establish the actual routing/formula architecture.

## Type

Architecture / Dead Code / Reliability

## Severity

High

## Scope

Recipe system, formula system, skill selection

## Problem

The codebase contains two unwired subsystems that are maintained but never executed:

### 1. Formula Patterns (`src/formulas/patterns.rs`)

`docs/SKILL_SYSTEM.md` explicitly states:

> "The abstract FormulaPattern definitions in `src/formulas/patterns.rs` capture intent and use-case heuristics but are **not currently wired into the selection path**. They serve as reference and may be used for future optimization."

This is **dead code** — compiled, maintained, and potentially confusing, but never used in the live path.

### 2. Recipe System (`src/recipes/`)

The recipe system (Task 451) provides "versioned external workflow patterns without Rust code changes." The formula-to-recipe bridge (`formula_to_recipe_id()`) maps 10 formulas to recipes. But:

- `docs/SKILL_SYSTEM.md` says recipes are used "when a formula matches a recipe-eligible pattern"
- There is no evidence in `app_chat_loop.rs` or `tool_loop.rs` that recipes are ever loaded or executed
- The recipe schema is well-defined but unwired
- `src/recipes/mod.rs` and `src/recipes/loader.rs` exist but are not called from the live path

This dead code violates:
- **Rule 2**: reliability over speed — dead code increases compile times and binary size
- **Rule 13**: delete-first patching — "Always prioritize the removal of dead code over patching regressions"
- **DEVELOPMENT_GUIDELINES**: "Do not perform broad refactors unless they directly serve an active reliability goal" — but removing dead code IS a reliability goal

## Root Cause

The recipe and formula pattern systems were designed for future extensibility but never integrated. They were kept "for future use" rather than deleted.

## Proposed Solution

### Decision: Wire or Delete?

Evaluate whether the recipe system provides value:

**Wire it if:**
- The recipe system can replace hardcoded formula logic in `app_chat_loop.rs`
- Recipes can be loaded from `_elma-tasks/` or `config/recipes/`
- The recipe system reduces Rust code changes for new workflows

**Delete it if:**
- The tool-calling pipeline makes recipes unnecessary (the model chooses tools directly)
- Formula selection is already simple enough (reply_only vs inspect_reply)
- Maintaining recipes adds complexity without benefit

Given that routing is hardcoded to SHELL and formula selection is trivial (`reply_only` vs `inspect_reply`), the recipe system is likely unnecessary. **Recommendation: delete.**

### Phase 1 — Delete formula patterns

1. Delete `src/formulas/patterns.rs`
2. Keep `src/formulas/mod.rs` and `src/formulas/scores.rs` if they are used in live formula selection
3. Verify `scores.rs` is actually called from `skills.rs` or `app_chat_loop.rs`

### Phase 2 — Delete recipe system

1. Delete `src/recipes/mod.rs`
2. Delete `src/recipes/loader.rs`
3. Delete `src/recipes/tests.rs`
4. Delete the `recipes/` directory
5. Remove `pub mod recipes` from `main.rs`
6. Remove all `formula_to_recipe_id()` references

### Phase 3 — Update docs

1. Remove recipe system references from `docs/ARCHITECTURE.md`
2. Remove recipe system references from `docs/SKILL_SYSTEM.md`
3. Update the "End-to-End Flow" diagram

### Phase 4 — Verify formula selection still works

1. Ensure `skills.rs` can still select formulas without recipes
2. Ensure `app_chat_loop.rs` can still construct `FormulaSelection` without recipe references

## Acceptance Criteria

- [ ] `src/formulas/patterns.rs` is deleted (if unwired)
- [ ] `src/recipes/` directory is deleted
- [ ] `formula_to_recipe_id()` is deleted
- [ ] `main.rs` has no `recipes` module declaration
- [ ] `cargo build && cargo test` passes
- [ ] Formula selection still works (reply_only / inspect_reply)
- [ ] Docs are updated

## Verification Plan

- `find src -name "*recipe*"` → no files
- `find src -name "patterns.rs"` → no file in formulas/
- `grep -r "recipe" src/` → no matches (except in comments about deletion)
- `grep -r "formula_to_recipe" src/` → no matches
- Integration test: simple request → formula `reply_only` selected
- Integration test: complex request → formula `inspect_reply` selected

## Dependencies

- `src/skills.rs` (formula selection)
- `src/formulas/` (formula scoring)
- `src/app_chat_loop.rs` (formula construction)

## Notes

If the recipe system is deemed worth keeping, create a separate task to wire it into the live path. But do not keep dead code "just in case."

The docs say:

> "Always prioritize the removal of dead code over patching regressions."

If recipes are ever needed again, they can be rebuilt from the recipe schema (which is documented in `docs/SKILL_SYSTEM.md` and can be preserved in `docs/` even if the code is deleted).

Formula patterns, however, are less valuable — they are essentially heuristic maps that violate Rule 1 (no word-based routing). They should be deleted regardless.
