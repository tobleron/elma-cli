# Task 458: Shell Mutation Snapshot And Rollback Coverage Revisit

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** postponed Task 094, pending Task 457, pending Task 459, completed Task 242, completed Task 325

## Summary

Close the recovery gap for shell-driven file mutations by deciding whether risky shell commands should create automatic snapshots before execution.

## Evidence From Audit

- Structured `Edit` steps create pre-edit snapshots.
- Shell execution can still mutate files through `mv`, `cp`, redirection, scripts, or generated commands.
- Shell preflight and permission gates classify risk, but rollback coverage is not guaranteed for shell mutations.
- Task 457 plans native file operations, but raw shell remains available.

## User Decision Gate

Ask the user which rollback policy they want:

- Snapshot before every caution/dangerous shell command.
- Snapshot only before shell commands likely to mutate workspace files.
- Prefer native mutation tools and block shell mutations unless explicitly approved.

## Implementation Plan

1. Inventory mutation-capable shell patterns currently allowed after preflight.
2. Define snapshot trigger policy from user choice.
3. Integrate snapshot creation before approved risky shell execution.
4. Ensure rollback metadata records the command and affected path estimate.
5. Add tests for redirection, `mv`, `cp`, script execution, and native edit comparison.

## Success Criteria

- [ ] Risky shell mutations have user-approved rollback coverage.
- [ ] Snapshot creation is visible in transcript.
- [ ] Rollback can restore shell-created or shell-modified files when feasible.
- [ ] Native mutation tools remain preferred where available.

## Anti-Patterns To Avoid

- Do not block all shell usage; inspection commands remain essential.
- Do not create huge snapshots for broad commands without user-visible cost.
- Do not weaken existing permission/preflight gates.
