# Task 775: Correct Stale Complexity Levels in config/defaults

## Type
Cleanup / Documentation

## Severity
Medium

## Scope
Configuration

## Problem
`config/defaults/complexity_assessor.toml` and other related configuration files still reference `INVESTIGATE` and `OPEN_ENDED` complexity levels. These levels were explicitly removed in Task 766 in favor of a two-gate `DIRECT` and `MULTISTEP` system.

## Root Cause
The configuration files were not updated when the core complexity logic was refactored in Task 766.

## Proposed Solution
Sync all configuration files with the modern two-gate complexity system.

- Phase 1: Update `config/defaults/complexity_assessor.toml` to remove `INVESTIGATE` and `OPEN_ENDED`.
- Phase 2: Update all model-specific `complexity_assessor.toml` files (e.g., in `config/gemma-*/`, `config/llama-*/`).
- Phase 3: Audit all prompts that mention these stale levels and update them to use `DIRECT` or `MULTISTEP`.
- Phase 4: Update any remaining documentation that refers to the old 4-level system.

## Acceptance Criteria
- [ ] No configuration file or system prompt references `INVESTIGATE` or `OPEN_ENDED` as valid complexity levels.
- [ ] All complexity assessment results are either `DIRECT` or `MULTISTEP`.

## Verification Plan
- Workspace audit: `grep -r "INVESTIGATE" config/` should return no matches in relevant fields.
- Integration test: Run complexity assessment and verify that only the new levels are produced.

## Dependencies
Task 771 (Remove Word-Based Routing from Complexity Assessor)

## Notes
- This is a "Truthfulness & Accuracy" (Dimension F) task.
