# 003 — Split Monolithic tool_calling.rs into Per-Tool Executor Modules

- **Priority**: Critical
- **Category**: Refactoring
- **Depends on**: None
- **Blocks**: 004, 008, 012, 016

## Problem Statement

`src/tool_calling.rs` is 3,345 lines and contains:
- The `ToolExecutionResult` type definition
- A 30-branch `match` statement in `execute_tool_call()` (lines 87-146)
- All 30+ tool executor functions in the same file
- UI event emission intertwined with execution logic
- Internal helper types (`LsEntry`) and functions (`collect_entries`, `is_ignored`, `format_time`)

This makes it impossible to:
- Add tool-specific validation without touching the monolith
- Test individual tool executors in isolation
- Reason about the safety properties of any single tool
- Add new tools without growing an already enormous file
- Review tool changes without scrolling through thousands of lines

## Why This Matters for Small Local LLMs

- Each tool executor should have its own validation layer tuned to the specific failure modes of small models for that tool
- A monolith encourages copy-paste patterns instead of shared validation infrastructure
- Small models produce more edge-case arguments — each tool needs focused defensive parsing that's easier to maintain in a dedicated module

## Current Behavior

```rust
// tool_calling.rs:87-146
match tool_name.as_str() {
    "ls" => exec_ls(&args_value, workdir, &call_id, tui),
    "observe" => exec_observe(&args_value, workdir, &call_id, tui),
    "tool_search" => exec_tool_search(&args_value, &call_id, tui),
    "shell" => exec_shell(args, &args_value, workdir, session, &call_id, tui).await,
    "read" => exec_read(&args_value, workdir, &call_id, tui),
    // ... 25 more branches
}
```

All executor functions are defined in the same file, with no shared validation infrastructure beyond ad-hoc path checks.

## Recommended Target Behavior

Split into:
```
src/
  tools/
    mod.rs              — pub mod declarations, shared ToolExecutionResult, execute_tool_call dispatcher
    exec_ls.rs          — exec_ls, LsEntry, collect_entries, is_ignored, format_time
    exec_observe.rs     — exec_observe
    exec_shell.rs       — exec_shell (largest; may need sub-modules)
    exec_read.rs        — exec_read
    exec_glob.rs        — exec_glob
    exec_patch.rs       — exec_patch, verify_syntax integration
    exec_edit.rs        — exec_edit
    exec_write.rs       — exec_write
    exec_search.rs      — exec_search
    exec_stat.rs        — exec_stat
    exec_copy.rs        — exec_copy
    exec_move.rs        — exec_move
    exec_mkdir.rs       — exec_mkdir
    exec_trash.rs       — exec_trash
    exec_touch.rs       — exec_touch
    exec_file_size.rs   — exec_file_size
    exec_exists.rs      — exec_exists
    exec_workspace_info.rs
    exec_repo_map.rs    — exec_repo_map
    exec_git_inspect.rs — exec_git_inspect
    exec_interpreter.rs — exec_run_python, exec_run_node
    exec_job.rs         — exec_job_start, exec_job_status, exec_job_output, exec_job_stop
    exec_fetch.rs       — exec_fetch
    exec_respond.rs     — exec_respond, exec_summary
    exec_update_todo_list.rs
    exec_tool_search.rs — exec_tool_search
    validation.rs       — shared path validation, argument sanitization helpers
    prelude.rs          — common imports for executor modules
```

## Source Files That Need Modification

- `src/tool_calling.rs` — Split into `src/tools/` directory; keep `ToolExecutionResult` and `execute_tool_call` dispatcher
- `src/main.rs` — Update `mod tool_calling` to point to new module structure
- `src/tool_loop.rs` — Update import for `ToolExecutionResult`
- Any file importing from `crate::tool_calling::` — update as needed

## New Files/Modules

- `src/tools/mod.rs` — Module declarations, re-exports, dispatcher
- `src/tools/validation.rs` — Shared argument validation infrastructure
- `src/tools/prelude.rs` — Common imports (`use crate::*`, `use super::*`)
- 25+ per-tool executor files as listed above

## Step-by-Step Implementation Plan

1. Create `src/tools/` directory
2. Move `ToolExecutionResult` type to `src/tools/types.rs` or keep in `mod.rs`
3. For each tool executor:
   a. Extract the function into its own file
   b. Move any private helper functions (e.g., `LsEntry`, `collect_entries`) with the tool
   c. Make shared helpers (`emit_tool_start`, `emit_tool_result`, `format_time`) accessible via `pub(crate)` in a shared module
4. Move `is_tool_call_markup`, `tool_signal`, `normalize_shell_signal` to a shared utilities module
5. Create the `execute_tool_call` dispatcher as a thin routing function in `mod.rs`
6. Update `main.rs` module declarations
7. Run `cargo check` after each tool move to catch import issues
8. Run full test suite
9. Verify tool registry parity test still passes

## Recommended Crates

None new — this is purely a reorganization.

## Validation/Sanitization Strategy

- Each tool executor must explicitly import the validation helpers it uses
- No tool executor should reach across into another tool's module
- Shared path validation lives in `src/tools/validation.rs`
- The dispatcher must still route unknown tools to the same error path

## Testing Plan

1. Run existing test suite: `cargo test`
2. The `test_tool_executor_parity` test in `tool_registry.rs` must still pass
3. Smoke-test each tool category: read, write, shell, file ops, metadata
4. Verify that command blocking (preflight, permission gate) still works for shell
5. Run scenario tests

## Acceptance Criteria

- `src/tool_calling.rs` is ≤500 lines (dispatcher + shared types only)
- Each tool executor is in its own file ≤400 lines
- All existing tests pass
- No tool executor imports from another tool executor's module
- The `execute_tool_call` dispatcher uses a clean match or lookup table

## Risks and Migration Notes

- **Risk**: Breaking imports across the codebase. Use `pub(crate) use` re-exports in `src/tools/mod.rs` to maintain backward compatibility during migration.
- **Risk**: Moving `verify_syntax` (used by `exec_edit`, `exec_write`, `exec_patch`) requires careful extraction.
- **Risk**: `exec_shell` is complex (400+ lines) and depends on many subsystems; extract it last after patterns are established.
- **Strategy**: Do this incrementally — move one tool per commit, verify with `cargo check` each time.
