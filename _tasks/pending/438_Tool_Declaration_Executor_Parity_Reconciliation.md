# Task 438: Tool Declaration And Executor Parity Reconciliation

**Status:** pending
**Priority:** CRITICAL
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 425, pending Task 394, pending Task 403, pending Task 433, completed Task 320

## Summary

Reconcile the tool schemas registered in `elma-tools` with the tools actually executable by `src/tool_calling.rs`.

## Why

The model can be shown tool declarations that the executor does not handle. This breaks semantic continuity: Elma advertises capabilities, the model calls them, then the executor returns `Unknown tool`.

## Evidence From Audit

- `elma-tools/src/tools/mod.rs` registers `edit`, `fetch`, `glob`, `ls`, `observe`, `patch`, `read`, `respond`, `search`, `shell`, `summary`, `todo`, `tool_search`, and `write`.
- `src/tool_calling.rs::execute_tool_call` currently handles only `observe`, `tool_search`, `shell`, `read`, `search`, `respond`, `summary`, and `update_todo_list`.
- `fetch` is registered as non-deferred even though pending Task 403 says web fetch should be disabled by default and security-gated.
- Dynamic `tool_search` can load schemas that have no executor implementation.

## User Decision Gate

Ask the user to choose one policy for each missing executor:

- Implement now.
- Keep registered but hide from model until its task is active.
- Remove/deprecate the declaration.
- Route to an existing native equivalent.

Specially ask whether `fetch` should be hidden until Task 403 is implemented.

## Implementation Plan

1. Generate a declaration/executor matrix for all `elma-tools` registrations.
2. Add tests that every default and dynamically discoverable tool has an executor or is explicitly hidden/deferred.
3. Apply the user-approved disposition for each missing executor.
4. Ensure `tool_search` cannot expose tools that fail parity.
5. Add transcript-visible policy messages when a tool is intentionally unavailable.

## Success Criteria

- [ ] No model-visible tool can resolve to `Unknown tool` in normal operation.
- [ ] `fetch` is not model-visible unless security-gated execution exists.
- [ ] Parity tests cover default tools and tool-search-loaded tools.
- [ ] Task 425/394/403 scopes are updated if this task delegates implementation.

## Anti-Patterns To Avoid

- Do not silently remove a tool from the registry without documenting why.
- Do not implement network tools without the offline-first policy gates.
- Do not solve parity by broadening `Unknown tool` into shell fallback.
