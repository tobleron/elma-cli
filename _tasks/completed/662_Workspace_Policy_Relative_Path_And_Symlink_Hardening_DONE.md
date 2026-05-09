# Task 662: Workspace Policy Relative Path And Symlink Hardening

**Status:** pending
**Priority:** CRITICAL
**Type:** Security / Reliability
**Scope:** `src/workspace_policy.rs`, `src/program_utils.rs`, `src/execution_steps_*`, `elma-tools/src/tools/*`
**Source:** AGENTS.md workspace-only rule; agent `_knowledge_base` Roo ignore/protect audit

## Summary

Enforce workspace-relative core tool paths, robust ignore/protect matching, and symlink escape prevention.

## Evidence And Gap

- AGENTS.md says core file tools operate on workspace-relative paths only and absolute paths are rejected with clear alternatives.
- The audit found `program_utils.rs` accepts absolute paths inside the workspace in some cases.
- `workspace_policy.rs` matching is simple and should support full relative path semantics.

## Implementation Plan

1. Create a single path normalization/policy API used by every core file tool.
2. Reject absolute paths for core tools with a suggested workspace-relative path when possible.
3. Resolve symlinks safely and block workspace escapes.
4. Implement `.elmaignore` and `.elmaprotect` matching against normalized relative paths.
5. Keep FileScout as the explicit read-only whole-system exception.

## Acceptance Criteria

- [ ] Absolute paths are rejected by core tools even when inside the workspace.
- [ ] Symlink escapes are blocked for reads and writes.
- [ ] Nested ignore/protect globs behave predictably.
- [ ] All file tools use the same policy layer.

## Verification Plan

Run path policy tests for absolute paths, `..`, symlinks, nested globs, protected files, and FileScout exceptions.

