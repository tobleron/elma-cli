# Task 362: Parallel Read/Search Tool Execution

**Status:** pending
**Priority:** medium-high
**Primary surfaces:** `src/streaming_tool_executor.rs`, `src/tool_loop.rs`, `src/tool_calling.rs`, `src/tool_registry.rs`
**Depends on:** Task 339 (tool metadata policy)
**Related tasks:** Task 338 (event log), Task 340 (native search execution), Task 335 (LSP diagnostics)

## Objective

Use metadata-backed parallel execution for independent read-only tool calls while preserving deterministic transcript order, evidence order, stop-policy behavior, and safety gates.

## Current Code Reality

- `src/streaming_tool_executor.rs` already exists and can run batches.
- Its safety classifier is currently hardcoded by tool name: `read`, `search`, `respond`.
- `src/tool_loop.rs` currently executes each tool call serially in its own loop and performs important post-processing after each call:
  - transcript flush
  - evidence ledger entry
  - stop policy update
  - respond abuse guard
  - tool message insertion
  - compaction/evidence updates
- The existing batch executor passes `None` for TUI during parallel execution.
- Parallel execution must not skip the existing per-tool post-processing semantics.

## Design Requirements

### Metadata-Backed Scheduling

Do not use tool names directly to decide concurrency. Use Task 339 tool metadata.

Initial parallel-safe candidates:

- `read`
- native `search` once Task 340 removes shell-string execution
- `lsp_diagnostics` only if the LSP manager declares the specific request safe

Serial by default:

- `respond`
- `summary`
- `update_todo_list`
- `shell`
- `edit`
- `write`
- `patch`
- `fetch`
- `browser_observe`
- unknown or declaration-only tools

### Batch Planning

Add a pure planner function that can be tested without executing tools:

```rust
pub(crate) enum ToolBatch {
    ParallelReadOnly(Vec<ToolCall>),
    Serial(ToolCall),
}

pub(crate) fn plan_tool_batches(
    calls: &[ToolCall],
    registry: &DynamicToolRegistry,
    limits: ParallelToolLimits,
) -> Vec<ToolBatch>
```

Rules:

- keep original call order in final results
- group adjacent independent read-only calls up to the configured limit
- do not move a read-only call across a serial call
- do not parallelize calls that require permission prompts
- do not parallelize calls that touch the same mutable state

### Tool Loop Integration

Refactor `src/tool_loop.rs` so execution and post-processing are separate:

```rust
async fn execute_one_or_batch(...) -> Vec<ToolExecutionResult>;
fn record_tool_result_in_order(...);
```

The post-processing must still run in original tool-call order even if execution completed out of order.

Preserve existing behavior for:

- evidence-required respond gate
- consecutive respond guard
- stop policy counters
- session flush
- evidence ledger entries
- tool result messages sent back to the model

### UI/Event Handling

Do not share `&mut TerminalUI` across parallel tasks.

Acceptable approaches:

- execute parallel calls without direct TUI mutation and emit ordered tool events afterward
- or send tool events through a channel consumed by the main loop

Do not allow parallel execution to reorder visible transcript rows.

### Limits And Failure Semantics

Add config/defaults:

- default max parallel read-only tools: 3
- hard max: 8
- per-tool timeout remains tool-specific
- if one parallel tool fails, record the failure but keep other results
- if planner is uncertain, choose serial

## Implementation Steps

1. Complete Task 339 metadata needed for concurrency decisions.
2. Add `ParallelToolLimits` and `plan_tool_batches` in `src/streaming_tool_executor.rs`.
3. Replace `is_concurrency_safe(tool_name)` with metadata lookup.
4. Refactor tool-loop post-processing into a reusable ordered function.
5. Integrate batch execution into `src/tool_loop.rs`.
6. Add fake executor tests so parallel behavior is deterministic and does not depend on filesystem timing.
7. Add real read/search integration tests with temp files.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test streaming_tool_executor
cargo test tool_loop
cargo test tool_registry
cargo test tool_calling
cargo build
```

Required coverage:

- planner groups adjacent read-only calls
- planner does not move calls across serial operations
- planner respects max parallel limit
- unknown tools are serial
- declaration-only tools are serial or blocked
- respond is serial
- update_todo_list is serial
- shell is serial
- edit/write/patch are serial
- fetch/browser are serial by default
- parallel results are returned in original call order
- failure in one parallel call does not drop successful sibling results
- evidence ledger entries are recorded in original order
- transcript flush happens once per result in original order
- respond abuse guard behavior is unchanged
- TUI/event rows are deterministic

Optional performance probe:

```bash
cargo test parallel_read_search_smoke -- --ignored --nocapture
```

This probe may use artificial delayed fake tools and should show parallel elapsed time below serial elapsed time.

## Done Criteria

- All required verification commands pass.
- No name-only concurrency policy remains outside compatibility tests.
- Tool-loop semantics are preserved for serial and parallel batches.
- Parallelism is bounded and disabled on uncertainty.
- No prompt-core changes are included.

## Anti-Patterns

- Do not parallelize by hardcoded tool name.
- Do not parallelize any write-capable tool.
- Do not allow parallel tool completion order to become transcript order.
- Do not skip evidence or stop-policy bookkeeping for batched tools.
- Do not share mutable TUI state across spawned tasks.
