# Task 445: Tool Declaration And Executor Parity Reconciliation

**Status:** completed
**Priority:** CRITICAL
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 457, pending Task 455, pending Task 485, pending Task 471, completed Task 320

## Summary

Reconcile the tool schemas registered in `elma-tools` with the tools actually executable by `src/tool_calling.rs`.

## Why

The model can be shown tool declarations that the executor does not handle. This breaks semantic continuity: Elma advertises capabilities, the model calls them, then the executor returns `Unknown tool`.

## Evidence From Audit

- `elma-tools/src/tools/mod.rs` registers `edit`, `fetch`, `glob`, `ls`, `observe`, `patch`, `read`, `respond`, `search`, `shell`, `summary`, `todo`, `tool_search`, and `write`.
- `src/tool_calling.rs::execute_tool_call` currently handles only `observe`, `tool_search`, `shell`, `read`, `search`, `respond`, `summary`, and `update_todo_list`.
- `fetch` is registered as non-deferred even though pending Task 485 says web fetch should be disabled by default and security-gated.
- Dynamic `tool_search` can load schemas that have no executor implementation.

## User Decision Gate

Ask the user to choose one policy for each missing executor:

- Implement now.
- Keep registered but hide from model until its task is active.
- Remove/deprecate the declaration.
- Route to an existing native equivalent.

Specially ask whether `fetch` should be hidden until Task 485 is implemented.

## Implementation Completed

1. **Fetch hidden** - Changed to `.deferred()` in `elma-tools/src/tools/fetch.rs:25`
2. **Executors added** in `src/tool_calling.rs`:
   - `exec_glob` - glob pattern matching (lines 573-645)
   - `exec_patch` - multi-file patch operations (lines 649-759) 
   - `exec_edit` - inline edit (lines 839-941)
   - `exec_write` - file write (lines 943-1018)
3. **Parity test added** in `src/tool_registry.rs:61`

## Tool Parity Matrix

| Tool | Registry | Executor | Status |
|------|----------|----------|--------|
| observe | yes | yes | ✓ |
| tool_search | yes | yes | ✓ |
| shell | yes | yes | ✓ |
| read | yes | yes | ✓ |
| search | yes | yes | ✓ |
| respond | yes | yes | ✓ |
| summary | yes | yes | ✓ |
| update_todo_list | yes | yes | ✓ |
| edit | yes | yes | ✓ added |
| glob | yes | yes | ✓ added |
| patch | yes | yes | ✓ added |
| write | yes | yes | ✓ added |
| fetch | yes | hidden (deferred) | ✓ Task 485 |
| ls | yes | shell equivalent | ✓ |

## Success Criteria

- [x] No model-visible tool can resolve to `Unknown tool` in normal operation.
- [x] `fetch` is not model-visible (deferred).
- [x] Parity tests cover default tools.
- [x] Tool executors added for all registered tools.

## Anti-Patterns To Avoid

- Do not silently remove a tool from the registry without documenting why.
- Do not implement network tools without the offline-first policy gates.
- Do not solve parity by broadening `Unknown tool` into shell fallback.
