# Task 462: Native Git Inspection And Worktree Tool

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-4 days
**Dependencies:** Task 387, Task 441
**References:** source-agent parity: normal coding-agent git awareness

## Objective

Add a rust-first git inspection tool suite so Elma can inspect repository state without defaulting to shell `git` commands.

## Scope

Initial read-only tools:

- git status summary
- current branch and upstream
- changed files
- diff stat
- recent commits

Write operations such as commit, checkout, reset, and rebase are out of scope for the first task.

## Implementation Plan

1. Choose a Rust git implementation strategy (`git2`, `gix`, or existing command fallback with structured wrapping).
2. Add `git_inspect` declaration with mode enum and path scope.
3. Enforce workspace policy and protected paths.
4. Return structured output that can feed evidence and final summaries.
5. Fall back to shell `git` only when native support is unavailable, with transcript-visible reason.

## Verification

```bash
cargo test git
cargo test tool_calling
cargo build
```

## Done Criteria

- Common git inspection is available without shell.
- Worktree state is summarized safely and clearly.
- Shell fallback is explicit and tested.

