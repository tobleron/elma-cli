# Task 448: Task Ledger Drift And Completed Claim Reconciliation

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 356, pending Task 433

## Summary

Reconcile task ledger drift where active/pending/completed files or completed claims no longer match the current codebase.

## Evidence From Audit

- `_tasks/active/435_Config_Architecture_Global_Essentials_Source_Defaults_And_Model_Overrides.md` exists while `_tasks/completed/435_Config_Architecture_Global_Essentials_Source_Defaults_And_Model_Overrides_DONE.md` also exists.
- completed Task 203 claims `extract_djvu()` and `extract_mobi()` exist, but current source does not contain those functions.
- completed Task 251 says EPUB metadata/spine extraction is implemented, while current `extract_epub()` reports full implementation pending.
- Deferred/postponed superseded task files remain useful historical context but are hard to distinguish from actionable work.

## User Decision Gate

Ask the user whether historical completed task files may be edited with reconciliation notes, or whether new reconciliation notes should live only in this task and docs.

Also ask whether duplicate active/completed task 435 should be archived, deleted, or merged.

## Implementation Plan

1. Build a task ledger consistency scanner for duplicate numeric prefixes across lifecycle folders.
2. Compare completed task claims for named functions/files against current source evidence.
3. Present a drift report to the user.
4. Apply approved ledger moves/renames/notes.
5. Add a lightweight task-ledger check for duplicate active/completed numbers.

## Success Criteria

- [ ] No task exists as both active and completed unless explicitly marked as a rollback artifact.
- [ ] Completed claims known to be inaccurate have reconciliation notes.
- [ ] Pending work that revives deferred tasks references the older task clearly.
- [ ] A local check can detect duplicate lifecycle state.

## Anti-Patterns To Avoid

- Do not rewrite history without user approval.
- Do not mark code complete because a completed task says it is complete.
- Do not move task files without preserving context needed by future implementers.
