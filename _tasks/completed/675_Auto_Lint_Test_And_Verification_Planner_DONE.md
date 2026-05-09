# Task 675: Auto Lint Test And Verification Planner

**Status:** pending
**Priority:** MEDIUM
**Type:** Tooling / Verification
**Scope:** `_scripts/`, `src/evaluation*`, `src/task_steward.rs`, `_tasks/_masterplan.md`
**Source:** deferred task 479

## Summary

Create a local verification planner that maps changed files and task types to the smallest meaningful test/lint/build commands.

## Evidence And Gap

- The docs define a verification ladder, but task implementers must choose commands manually.
- Large Rust test suites can be expensive; verification should scale with risk and blast radius.

## Implementation Plan

1. Map source globs to required checks in a config file.
2. Add a script/command that reads `git diff --name-only` and suggests/runs targeted verification.
3. Include special gates for UI snapshots, provider fixtures, task/session persistence, and shell safety.
4. Persist verification results in task/session artifacts when run by Elma.

## Acceptance Criteria

- [ ] Planner recommends focused commands before broad `cargo test`.
- [ ] High-risk surfaces still require full checks.
- [ ] Output is clear enough to paste into task completion notes.
- [ ] Planner does not require internet.

## Verification Plan

Run planner against synthetic changed-file sets and compare expected command lists.

