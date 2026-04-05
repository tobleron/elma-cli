# Task 097: Tool Arsenal Refinement & Expansion

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
