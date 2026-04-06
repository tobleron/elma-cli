# Task 115: Streaming Tool Execution

## Priority
**P2 — Quality of life improvement**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** None (parallel to Tasks 113/114)

## Problem

Currently, Elma waits for the full model response before executing tools. If the model generates 500 tokens of reasoning before a tool call, the user sees nothing until all 500 tokens arrive, then the tool executes, then more waiting.

Claude Code uses **streaming tool execution**: tools start running as soon as their `tool_use` block arrives in the stream, not after the full response. This dramatically improves perceived speed.

## Scope

### 1. Streaming Response Parsing
- Parse `tool_use` blocks from streaming chunks as they arrive
- Detect when a tool_use block is complete (has id, name, arguments)
- Don't wait for full response — start executing immediately

### 2. Parallel Tool Execution
- When multiple tool_calls arrive, check `is_concurrency_safe`:
  - `read`, `search`, `respond` = safe (read-only or non-mutating)
  - `shell` = NOT safe (may mutate state, race conditions)
- Safe tools run in parallel (max 3 concurrent)
- Unsafe tools run serially

### 3. Order Preservation
- Results must be fed back to the model in the order tool_calls appeared
- If tool #2 finishes before tool #1, wait for #1 before sending #2's result
- Progress messages visible immediately (see shell running)

### 4. Progress Visibility
- Show `→ executing shell` as soon as tool starts (not after completion)
- For long-running shell commands, show elapsed time every 5s
- Cancel indicator if user interrupts mid-execution

### 5. Error Cascading (for parallel shell)
- If a `shell` tool errors in a parallel batch, abort sibling shell calls
- Non-shell tools (read, search) do NOT cascade — they're independent
- Prevents wasted API calls on doomed sibling operations

### 6. Integration Points
- `src/tool_loop.rs` — replace sequential tool execution with streaming executor
- `src/streaming_tool_executor.rs` (new) — the streaming executor
- `src/ui_trace.rs` — progress messages for long-running tools
- `src/tool_calling.rs` — add `is_concurrency_safe` method

## Design Principles
- **Reliability before speed:** If streaming introduces instability, fall back to sequential
- **Small-model-friendly:** Streaming helps small models that take long to generate tool calls
- **No keyword routing:** Tool safety classification based on tool type, not user intent words

## Verification
1. `cargo build` clean
2. `cargo test` — executor logic, concurrency safety, order preservation, error cascading
3. Real CLI: parallel `read` + `search` — verify both run simultaneously
4. Real CLI: `shell` + `shell` — verify serial execution
5. Measure: time from user input to first tool execution starts (should be < model generation time for tool_use block)

## Acceptance Criteria
- [ ] Tools execute as soon as their tool_use block is complete in the stream
- [ ] Safe tools run in parallel (up to 3 concurrent)
- [ ] Unsafe tools (shell) run serially
- [ ] Results fed back in original order
- [ ] Progress visible immediately when tool starts
- [ ] Shell error cascading works (siblings aborted)
- [ ] Fallback to sequential if streaming fails
