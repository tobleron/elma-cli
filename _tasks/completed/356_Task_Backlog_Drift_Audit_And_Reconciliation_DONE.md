# Task 356: Task Backlog Drift Audit And Reconciliation

**Status:** in_progress
**Source patterns:** Local project task procedures, completed/pending source inspection

## Summary

Audit `_tasks/` against the current codebase and reconcile tasks whose described behavior is already implemented, partially implemented, superseded, or now inconsistent with source.

## Why

The deep architecture review found pending tasks that appear partially or fully implemented in source, including evidence ledger behavior, respond-abuse guards, patch/fetch definitions, and structured step metadata. A stale backlog wastes engineering time and can cause duplicate or contradictory implementation.

## Implementation Plan

1. Scan pending tasks and map each to current source files and tests.
2. Mark tasks as still valid, partially implemented, implemented-but-unverified, duplicate, superseded, or needs rewrite.
3. Move truly completed tasks through the normal active/completed process only after verification.
4. Update task descriptions where the objective is still valid but the implementation plan is outdated.
5. Produce a short backlog health report.

## Success Criteria

- [ ] Every pending task has a current-state classification.
- [ ] Tasks that are already implemented are not left as misleading implementation plans.
- [ ] No source behavior is changed by the audit.
- [ ] The audit lists verification needed before archive.
- [ ] Future deep-analysis task creation checks existing tasks first.

## Anti-Patterns To Avoid

- Do not mark tasks complete without verifying source and tests.
- Do not delete historical task records.
- Do not create duplicate pending tasks for already covered work.
