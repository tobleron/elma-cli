# Task 493: File Watcher And AI Comment Workflow

**Status:** pending
**Source patterns:** Aider watch mode, Roo file watcher, Crush file tracker
**Depends on:** Task 456 (file context tracker)

## Summary

Add an optional file watcher that updates stale file context and can trigger scoped tasks from explicit AI comments in files.

## Why

Reference agents support tight edit loops where user edits and agent context stay synchronized. Elma should be able to notice when files change outside its own edits and optionally respond to explicit task comments without polling the whole workspace.

## Implementation Plan

1. Add a watcher service scoped to the workspace and ignore rules.
2. Feed file changes into the file context tracker.
3. Define an explicit AI-comment marker format that is disabled by default.
4. Add a queue for watcher-triggered tasks with transcript-visible notices.
5. Respect workspace policy files and permission gates.

## Success Criteria

- [ ] External file edits mark previous reads as stale.
- [ ] Watcher respects ignore/protected rules.
- [ ] AI-comment execution is opt-in and visible.
- [ ] Event log records watcher triggers.
- [ ] Tests cover change detection and ignored paths.

## Anti-Patterns To Avoid

- Do not run hidden tasks from arbitrary comments by default.
- Do not watch outside the workspace.
- Do not bypass stale-read gates.
