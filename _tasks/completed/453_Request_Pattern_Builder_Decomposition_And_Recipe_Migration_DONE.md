# Task 453: Request Pattern Builder Decomposition And Recipe Migration

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** deferred Task 382, pending Task 451, completed Task 301, completed Task 355

## Summary

Audit and migrate hardcoded user-request pattern builders that violate the principle-first routing philosophy or encode old stress-test scenarios into production logic.

## Evidence From Audit

- `src/app_chat_patterns.rs` contains many `lower.contains(...)` request recognizers.
- `src/app_chat_builders_advanced.rs` dispatches on exact request shapes such as architecture audits, logging standardization, documentation audits, and `_stress_testing/` paths.
- Some generated `Step::Shell` records for read-only commands are marked `is_destructive: true` and `is_read_only: false`.
- `src/guardrails.rs` contains objective keyword checks.

## User Decision Gate

Ask the user whether each pattern family should be:

- Deleted as obsolete stress harness scaffolding.
- Migrated into Task 451 recipes/subrecipes.
- Kept temporarily behind an explicit test-mode feature.
- Replaced by a focused intel unit or tool capability query.

Do not remove a pattern until its covered behavior has a user-approved replacement or retirement decision.

## Implementation Plan

1. Inventory every user-message keyword/pattern recognizer outside command-prefix parsing.
2. Classify each as routing, safety, recipe-like workflow, UI filtering, or test-only.
3. Present the inventory and recommended disposition to the user.
4. Move approved recipe-like flows out of production builders into recipes or fixtures.
5. Add regression scenarios proving current supported behaviors still work without keyword routing.

## Success Criteria

- [ ] Production routing no longer depends on brittle request-shape keywords.
- [ ] Stress-test-specific paths do not affect normal users.
- [ ] Recipe-worthy flows are represented as recipes or task fixtures.
- [ ] Safety checks remain robust and are not weakened.
- [ ] Audit output documents every retained keyword use and why it is acceptable.

## Anti-Patterns To Avoid

- Do not replace one keyword list with another keyword list.
- Do not delete safety preflight checks merely because they contain command patterns.
- Do not edit `src/prompt_core.rs` for this task.
