# Task 541: Restore Accidentally Deleted Pending Task Files

**Status:** pending
**Priority:** HIGH (urgent — risk of permanent loss)
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P5 — Very High Confidence

## Summary

Five pending task files were deleted from `_tasks/pending/` and the deletion is unstaged and uncommitted. These are real, documented mid-priority tasks with evidence and implementation plans. They must be restored immediately before any `git clean` or workspace reset operation occurs.

## Files to Restore

```
_tasks/pending/469_Session_Runtime_State_Ownership_Audit.md
_tasks/pending/476_Cross_Platform_Portability_Gate.md
_tasks/pending/477_Cargo_Dependency_And_Feature_Hygiene_Audit.md
_tasks/pending/483_UI_Renderer_And_Module_Deprecation_Decision.md
_tasks/pending/484_Dead_Code_And_Deprecation_Decision_Audit.md
```

## Evidence

`git status` (from session):
```
deleted:    _tasks/pending/469_Session_Runtime_State_Ownership_Audit.md
deleted:    _tasks/pending/476_Cross_Platform_Portability_Gate.md
deleted:    _tasks/pending/477_Cargo_Dependency_And_Feature_Hygiene_Audit.md
deleted:    _tasks/pending/483_UI_Renderer_And_Module_Deprecation_Decision.md
deleted:    _tasks/pending/484_Dead_Code_And_Deprecation_Decision_Audit.md
```

All five were present in `HEAD` and contain valid task content with evidence, decision gates, and implementation plans.

## Implementation Plan

1. Run `git restore _tasks/pending/469_Session_Runtime_State_Ownership_Audit.md`
2. Run `git restore _tasks/pending/476_Cross_Platform_Portability_Gate.md`
3. Run `git restore _tasks/pending/477_Cargo_Dependency_And_Feature_Hygiene_Audit.md`
4. Run `git restore _tasks/pending/483_UI_Renderer_And_Module_Deprecation_Decision.md`
5. Run `git restore _tasks/pending/484_Dead_Code_And_Deprecation_Decision_Audit.md`
6. Verify: `git status` should show no pending deletions for these files
7. Commit: `git add _tasks/pending/ && git commit -m "restore: recover accidentally deleted pending task files 469 476 477 483 484"`

## Prevention

Investigate what caused the deletion — whether it was a prior Elma session, a manual command, or a tool. If Elma deleted these via a tool call without user approval, add a file-deletion guard that checks whether a file being deleted is a task file and requires explicit confirmation.

## Success Criteria

- [ ] All 5 task files restored and verified present in `_tasks/pending/`
- [ ] `git status` shows no unexpected deletions
- [ ] Changes committed to main branch
- [ ] Deletion source identified and prevention measure proposed

## Verification

```bash
git restore _tasks/pending/469_Session_Runtime_State_Ownership_Audit.md
git restore _tasks/pending/476_Cross_Platform_Portability_Gate.md
git restore _tasks/pending/477_Cargo_Dependency_And_Feature_Hygiene_Audit.md
git restore _tasks/pending/483_UI_Renderer_And_Module_Deprecation_Decision.md
git restore _tasks/pending/484_Dead_Code_And_Deprecation_Decision_Audit.md
git status
ls _tasks/pending/ | grep -E "469|476|477|483|484"
```
