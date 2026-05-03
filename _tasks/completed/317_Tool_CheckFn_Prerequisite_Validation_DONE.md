# Task 317: Tool check_fn Prerequisite Validation (Proposal 008)

**Status:** pending  
**Proposal:** [docs/_proposals/008-tool-check-fn-prerequisite-validation.md](../../docs/_proposals/008-tool-check-fn-prerequisite-validation.md)  
**Depends on:** None  

## Summary

Add `check_fn: Option<Arc<dyn Fn() -> bool + Send + Sync>>` to `ToolDefinitionExt` in `src/tool_registry.rs`. Register prerequisite checks for shell and search tools. Filter tool availability in `build_current_tools()`.

## Why

Tools currently assume their prerequisites exist (shell binary, `rg`). If `rg` is missing, the `search` tool silently fails at runtime. Hermes Agent demonstrates that tools should self-report availability at registration time via `check_fn`. This is a permanent global solution — no new tool will ever silently assume binary availability.

## Implementation Steps

1. Add `check_fn: Option<Arc<dyn Fn() -> bool + Send + Sync>>` to `ToolDefinitionExt`
2. Add `.with_check_fn()` builder method that wraps the closure
3. Add `which` crate to `Cargo.toml` for binary lookup
4. Register checks for `shell` (sh or bash) and `search` (rg or grep)
5. Add `available_tools()` filter method to `DynamicToolRegistry`
6. Update `build_current_tools()` to filter by `check_fn`
7. Build and test

## Success Criteria

- [x] `ToolDefinitionExt` has `check_fn` field
- [x] `.with_check_fn()` builder method available  
- [x] Shell and search tools register prerequisite checks
- [x] `build_current_tools()` filters by availability
- [x] `which` crate dependency added
- [x] `cargo build` succeeds
