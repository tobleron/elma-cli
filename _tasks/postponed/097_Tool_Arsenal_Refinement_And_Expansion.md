# Task 097: Tool Arsenal Refinement & Expansion

## Backlog Reconciliation (2026-05-02)

Split across Task 445 declaration/executor parity, Task 447 tool-context budgeting, and Task 457 rust-first file operations.


## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Priority
**P0 - FOUNDATIONAL INFRASTRUCTURE**

## Objective
Ensure every step type in Elma's program corresponds to a real tool or function call, so the maestro's plain English instructions can be reliably transformed into concrete actions.

## Current Arsenal
| Step Type | Mechanism | Status |
|-----------|-----------|--------|
| `Shell` | `handle_shell_step` — executes command via `std::process::Command` | ✅ Working |
| `Read` | `handle_read_step` — reads file content from workspace | ✅ Working |
| `Search` | `handle_search_step` — runs ripgrep search | ✅ Working |
| `Select` | `handle_select_step` — LLM picks items from list | ✅ Working |
| `Respond` | `handle_respond_step` — stores as final reply, no tool call | ✅ Working |
| `Reply` | `handle_reply_step` — stores as final reply, no tool call | ✅ Working |
| `Explore` | `handle_explore_step` — placeholder, stores objective | ⚠️ Stub |
| `Summarize` | `handle_summarize_step` — LLM summarizes text | ✅ Working |
| `Edit` | `handle_edit_step` — structured file edit | ✅ Working |
| `Plan` | `handle_plan_step` — LLM creates plan | ✅ Working |
| `MasterPlan` | `handle_master_plan_step` — LLM creates phased plan | ✅ Working |
| `Decide` | `handle_decide_step` — LLM makes decision | ✅ Working |
| `Write` | `handle_write_step` — writes file content | ✅ Working |
| `Delete` | `handle_delete_step` — removes file/dir | ✅ Working |

## TODO
- [ ] Implement `Explore` step as actual exploration loop (read → search → reason chain)
- [ ] Add `Move` step type — rename/relocate files
- [ ] Add `Mkdir` step type — create directories
- [ ] Add `Copy` step type — duplicate files
- [ ] Consider adding `Diff` step type — compare file versions before/after edits
- [ ] Ensure every step type has proper `depends_on` wiring for tool chaining
- [ ] Add tool registry awareness so the orchestrator knows what tools are available before generating steps

## Acceptance Criteria
- Every step type either calls a tool (shell/read/search/edit) or a function (respond/reply/plan/decide)
- No step type is a dead-end stub
- Tool availability is checked before step generation
- `cargo build` clean, `cargo test` all green
