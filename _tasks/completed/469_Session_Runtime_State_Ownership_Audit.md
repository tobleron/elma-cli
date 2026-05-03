# Task 469: Session Runtime State Ownership Audit

**Status:** pending
**Priority:** MEDIUM
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 436, Task 470, completed Task 430, completed Task 285

## Summary

Audit session state writers and readers so runtime state, transcript, artifacts, summaries, event log, and SQLite store have clear ownership.

## Evidence From Audit

- Session code spans `session_paths`, `session_write`, `session_flush`, `session_display`, `session_store`, `session_index`, `session_gc`, `session_hierarchy`, and `session_error`.
- Completed Task 430 introduced slim session layout, while legacy files and folders remain as fallback paths.
- completed Task 436 added resume behavior based on `session.json` and `session.md`; this audit should verify the implementation did not reintroduce duplicate legacy storage.
- Task 470 wants an action-observation event log without duplicating full transcript content.

## User Decision Gate

Ask the user which stores are canonical:

- Markdown transcript plus JSON session state.
- SQLite as index/cache only.
- SQLite as canonical store with markdown export.

Also ask when legacy folders should stop being read.

## Implementation Plan

1. Inventory every session writer and reader.
2. Define canonical ownership for transcript, tool artifacts, summaries, event log, index, and errors.
3. Present migration/compatibility choices to the user.
4. Update Task 470 assumptions and create a follow-up for completed Task 436 behavior if the audit finds drift.
5. Add tests that normal operation does not create duplicate legacy state.

## Success Criteria

- [ ] Canonical session-state ownership is documented.
- [ ] Resume behavior has one clear source of truth.
- [ ] Legacy reads are explicit migration fallbacks.
- [ ] Duplicate transcript/final-answer artifacts are prevented.

## Anti-Patterns To Avoid

- Do not delete user session history without backup.
- Do not duplicate full transcript text into every event.
- Do not make resume depend on trace-only state.
