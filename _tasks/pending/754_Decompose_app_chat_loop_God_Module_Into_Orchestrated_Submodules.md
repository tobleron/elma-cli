# Task 754: Decompose app_chat_loop.rs God Module Into Orchestrated Submodules

## Type

Architecture / De-bloating / Maintainability

## Severity

High

## Scope

Chat loop, orchestration, command handling, finalization

## Problem

`src/app_chat_loop.rs` is a **1354-line god module** that handles:

1. Chat command parsing (`/exit`, `/clear`, `/tasks`, `/help`, etc.)
2. Session picker UI (`open_session_picker`)
3. Workspace discovery (`try_workspace_discovery`)
4. Tool execution wrapper (`execute_tool`)
5. Intent annotation and classification (`annotate_and_classify`)
6. Policy fallbacks (`apply_policy_fallback`)
7. Shape fallbacks (`apply_shape_fallbacks`)
8. Reflection loops (`run_reflection_loop`)
9. Program building (`build_program` calls)
10. Tool loop orchestration (`orchestrate_with_retries`)
11. Final answer resolution (`resolve_final_text`)
12. Continuity checking and retry
13. Evidence ledger cleanup
14. Token estimation and status bar updates
15. Turn summary spawning
16. Goal state management

The DEVELOPMENT_GUIDELINES.md explicitly warns:

> "`src/main.rs` has historically been oversized. Continue extracting logic into cohesive domain modules."
> "Treat these files with extra care — they are large and tightly coupled"

While `main.rs` was de-bloated, `app_chat_loop.rs` became the new concentration point. It violates:
- **Rule 3**: "One intel unit, one role, one narrow decision" — `app_chat_loop.rs` does everything
- **Rule 11**: Runtime behavior priorities — crashes in this file bring down the entire session
- **DEVELOPMENT_GUIDELINES de-bloating guidance**: large modules should be split into façades and sub-modules

## Root Cause

As features were added (tasks 380, 498, 597, 605, 606, 609, 611, etc.), they were added as inline blocks in `app_chat_loop.rs` rather than extracted into modules. The file accumulated ~50 Task XXX comments, indicating repeated patching without refactoring.

## Proposed Solution

Follow the established pattern: create a façade module (`app_chat_loop.rs`) and sub-modules.

### New module structure:

```
src/app_chat_loop/
  mod.rs              ← 150-line façade: re-exports, module declarations
  commands.rs         ← Chat command handlers (/exit, /clear, /tasks, etc.)
  discovery.rs        ← Workspace discovery, tool discovery
  planning.rs         ← Program building, reflection, shape/policy fallbacks
  execution.rs        ← Orchestration call, tool loop integration
  finalization.rs     ← Final answer resolution, continuity retry
  cleanup.rs          ← Evidence cleanup, turn summary spawn, state save
```

### Phase 1 — Extract chat commands

Move `handle_chat_command()`, `open_session_picker()`, and all command handlers to `src/app_chat_loop/commands.rs`.

### Phase 2 — Extract discovery

Move `try_workspace_discovery()`, `execute_tool()`, and tool discovery logic to `src/app_chat_loop/discovery.rs`.

### Phase 3 — Extract planning

Move `annotate_and_classify()` if it remains after Task 751, `apply_policy_fallback()`, `apply_shape_fallbacks()`, `run_reflection_loop()`, and program building to `src/app_chat_loop/planning.rs`.

### Phase 4 — Extract execution

Move the `orchestrate_with_retries()` call and `is_tool_calling_result` detection to `src/app_chat_loop/execution.rs`.

### Phase 5 — Extract finalization

Move `resolve_final_text()`, continuity retry logic, and `build_best_effort_answer()` to `src/app_chat_loop/finalization.rs`.

### Phase 6 — Extract cleanup

Move evidence ledger cleanup, token estimation, status bar updates, turn summary spawning, and goal state saving to `src/app_chat_loop/cleanup.rs`.

### Phase 7 — Simplify façade

The remaining `run_chat_loop()` in `mod.rs` should be ~200-300 lines that:
1. Initialize TUI
2. Loop over input
3. Call `commands::handle_chat_command()`
4. Call `discovery::try_workspace_discovery()`
5. Call `planning::build_program()`
6. Call `execution::run_orchestration()`
7. Call `finalization::resolve_final_answer()`
8. Call `cleanup::end_turn()`

## Acceptance Criteria

- [ ] `src/app_chat_loop.rs` is replaced by `src/app_chat_loop/mod.rs` (~150 lines)
- [ ] Each submodule is ≤ 400 lines
- [ ] `run_chat_loop()` in `mod.rs` is ≤ 300 lines
- [ ] No functionality is lost or changed
- [ ] All Task XXX comments are preserved (or migrated with the relevant code)
- [ ] `cargo build && cargo test` passes
- [ ] `cargo fmt` passes

## Verification Plan

- `wc -l src/app_chat_loop/mod.rs` → ≤ 150
- `wc -l src/app_chat_loop/*.rs` → each ≤ 400
- `grep -c "async fn\|fn " src/app_chat_loop/mod.rs` → ≤ 10 functions
- Integration test: full chat session (input → planning → execution → finalization → cleanup)
- Regression test: all existing `/` commands work

## Dependencies

- Task 751 (delete or narrow dead routing) should land first to reduce `app_chat_loop.rs` size
- Task 748 (delete app_chat_patterns) reduces planning complexity
- `src/app_chat_handlers.rs` (existing command handlers — may be merged into commands.rs)
- `src/app_chat_helpers.rs` (existing helpers — may be merged)

## Notes

This is a **pure refactor** — no behavior changes. The goal is to make the chat loop maintainable.

Do not use this refactor as an opportunity to "improve" behavior. Extract the code as-is, then create follow-up tasks for improvements.

The `app_chat_loop.rs` file currently has 50+ Task XXX inline comments. These are evidence of the delete-first patching policy violation (Rule 13). After decomposition, audit these comments — many may refer to completed work that can be removed.
