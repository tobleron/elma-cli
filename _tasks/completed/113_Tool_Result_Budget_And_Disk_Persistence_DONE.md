# Task 113: Tool Result Budget & Disk Persistence

## Priority
**P0 — Critical for long conversations**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** None (builds on tool-calling from Phase 2)

## Problem

When tool results are large (e.g., `ls -laR` on a big project, `cat` on a long file), they consume the context window and push out earlier conversation history. On a 3B model with ~8K context window, a single large file read can consume 50%+ of available tokens.

Claude Code solves this with **tool result budgeting**: results above a threshold are persisted to disk, and the model sees a `<persisted-output>` wrapper with a 2KB preview and file path.

## Scope

### 1. Per-Tool Result Threshold
- Default threshold: 50,000 characters (~20K tokens)
- Configurable via `tool_result_max_chars` in profile config
- Per-tool overrides (e.g., `shell` can be higher, `read` lower)

### 2. Disk Persistence
- Store large results in `sessions/{id}/tool-results/{tool_call_id}.txt`
- Atomic writes (write to .tmp, rename)
- Include metadata file with tool name, timestamp, original size

### 3. Model-Facing Format
When a result is persisted:
```
[persisted-output]
Tool: shell
Original size: 45,230 chars
Preview: (first 2KB of content...)
Full output saved to: sessions/s_xxx/tool-results/tc_123.txt
Use `read` tool to examine the full output if needed.
[/persisted-output]
```

### 4. Aggregate Budget
- Enforce per-message total: `MAX_TOOL_RESULTS_PER_MESSAGE_CHARS` (default 150K)
- When exceeded, replace largest results first with persisted-output wrappers
- Track which tool_call_ids have been replaced (to avoid re-persisting)

### 5. Integration Points
- `src/tool_calling.rs` — check threshold when returning ToolExecutionResult
- `src/tool_loop.rs` — build aggregated budget, apply persistence before sending messages
- `src/types_api.rs` — add `persisted_output_path` field to ToolExecutionResult
- `src/session.rs` or new `src/tool_result_storage.rs` — persistence logic

## Design Principles
- **Small-model-first:** The threshold defaults are tuned for 8K context windows, not 200K cloud models
- **Offline-first:** Persistence is local disk, no network required
- **Principle-driven:** Persist when cost exceeds value of keeping inline; don't truncate silently
- **No death spirals:** If persistence fails, fall back to truncated inline (not crash)

## Verification
1. `cargo build` clean, zero warnings
2. `cargo test` — unit tests for threshold logic, persistence, aggregate budget
3. Real CLI: `ls -laR /usr/local` — verify large output is persisted, model sees preview
4. Real CLI: multi-turn conversation with large outputs — verify context stays usable
5. Verify model can `read` the persisted file if asked for details

## Acceptance Criteria
- [ ] Tool results >50K chars are persisted to disk automatically
- [ ] Model sees preview + persisted-output wrapper, not full content
- [ ] Aggregate budget enforced per message (150K default)
- [ ] Model can use `read` tool to examine persisted output
- [ ] No regression on small tool results (they stay inline)
- [ ] Session directory is clean: tool-results/ subdirectory organized
