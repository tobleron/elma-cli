# Task 456: File Context Tracker And Stale Read Gate

**Status:** Pending
**Priority:** HIGH
**Promotion reason:** required for rust-native mutation tools and source-agent editing parity.
**Source patterns:** Roo-Code `FileContextTracker`, Crush file tracker, Aider read-before-edit discipline
**Revives:** `_tasks/postponed/022_Session_Based_File_Tracker.md`
**Depends on:** completed Task 326 (edit robustness), Task 455 (patch tool)

## Summary

Track files read, mentioned, edited, and externally modified during a session. Mark file context stale when the file changes after Elma read it, and require a fresh read before edit, write, delete, or patch operations.

## Why

The strongest editing agents guard against stale context. Elma has rich read and edit tooling, but the current architecture does not have one canonical file-context ledger shared by read, search, shell, edit, patch, and finalization.

## Implementation Plan

1. Add a `file_context_tracker` module with file path, last read hash, last read timestamp, last edit timestamp, source of mention, and stale state.
2. Update `read`, `search`, shell output file mentions, edit, write, and patch paths to record file context.
3. Add stale checks before write-like operations.
4. Surface stale file warnings as collapsible transcript rows.
5. Persist enough tracker state to support session resume.

## Success Criteria

- [ ] Edits are blocked or require explicit reread when the file changed after last read.
- [ ] External modifications are detected by mtime/hash comparison.
- [ ] Mentioned-only files are distinguished from actually read files.
- [ ] Tracker memory is capped with LRU or session-scoped limits.
- [ ] Tests cover stale reads, external edits, duplicate paths, symlinks, and resume.

## Anti-Patterns To Avoid

- Do not trust model claims that it read a file.
- Do not rely only on timestamps when a cheap hash is available.
- Do not bury stale-context warnings in trace-only logs.
