# 573 — Implement Structured Logging with Session-Scoped Tracing

- **Priority**: Medium
- **Category**: Logging/Observability
- **Depends on**: 554 (session-scoped state)
- **Blocks**: None

## Problem Statement

Logging uses ad-hoc `trace()` calls and `append_trace_log_line()` rather than structured `tracing` spans. The `tracing` and `tracing-subscriber` crates are declared as dependencies but used minimally. There's no session-scoped log isolation — all sessions write to the same log output.

## Why This Matters for Small Local LLMs

Debugging small model behavior requires rich, structured logs that capture:
- What the model was asked to do
- What the model responded with
- What tool calls were made and their results
- What errors occurred and how they were handled

Without structured logging, debugging model behavior requires reading through flat trace files.

## Recommended Target Behavior

1. Use `tracing` spans for structured, hierarchical logging:
   ```rust
   #[tracing::instrument(skip(args, tui))]
   async fn run_tool_loop(...) -> Result<ToolLoopResult> {
       tracing::info!(iteration = 1, "Starting tool loop");
       // ...
   }
   ```
2. Session-scoped log files: each session gets its own log file
3. Structured events: tool calls, model responses, errors as typed events
4. Configurable log levels per module

## Source Files That Need Modification

- `src/logging.rs` — Replace ad-hoc logger with tracing subscriber
- `src/tool_loop.rs` — Add tracing spans
- `src/tool_calling.rs` — Add tracing spans
- `src/orchestration_core.rs` — Add tracing spans
- `src/session_paths.rs` — Add session log file path

## Acceptance Criteria

- All major operations have tracing spans with structured fields
- Session log files are isolated per session
- Log level configurable per module via RUST_LOG
- No performance regression in release builds (spans compiled out at trace level)

## Risks and Migration Notes

- Tracing adds overhead if too many spans. Use `trace!` level for high-frequency events.
- Existing `trace()` function and `append_trace_log_line()` must be replaced or adapted.
