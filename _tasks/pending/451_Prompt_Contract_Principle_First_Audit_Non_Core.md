# Task 451: Prompt Contract Principle-First Audit Non-Core

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 305, pending Task 431

## Summary

Audit managed prompts outside `src/prompt_core.rs` for principle-first compliance, small-model decomposition, and schema contract consistency.

## Evidence From Audit

- `src/defaults_router.rs` and `src/defaults_evidence.rs` contain many prompt constants.
- Some prompts include long example-like distinctions and command-word guidance.
- Some units still request multi-field JSON where `_tasks/_objectives.md` prefers focused, narrow decisions for weak models.
- `src/prompt_core.rs` is protected and must not be changed without explicit approval.

## User Decision Gate

Ask the user before changing any managed prompt. For each prompt family, present:

- Current role and output contract.
- Risk of changing it.
- Proposed split or rewrite.
- Scenario tests to run before accepting the change.

## Implementation Plan

1. Inventory non-core prompts and classify by intel unit role.
2. Flag prompts that are example-heavy, multi-job, or schema-inconsistent.
3. Propose decompositions rather than prompt bloat.
4. Run scenario tests for any user-approved prompt change.
5. Update prompt hashes/tuning metadata only through the approved process.

## Success Criteria

- [ ] Non-core prompt inventory exists.
- [ ] Each approved rewrite has scenario evidence.
- [ ] No `src/prompt_core.rs` changes are made without explicit approval.
- [ ] Small-model JSON contracts are simpler, not larger.

## Anti-Patterns To Avoid

- Do not stuff more examples into prompts.
- Do not blame small models for failures.
- Do not change canonical prompt core in this task.
