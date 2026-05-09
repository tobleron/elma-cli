# Task 779: Eliminate chars/4 Token Estimation Heuristics

## Type
Truthfulness / Bug

## Severity
High

## Scope
UI / Observability

## Problem

The codebase has a proper `token_counter.rs` module using `tiktoken-rs` cl100k_base encoding (Task 499), but several UI and runtime paths still use the discredited `chars / 4` heuristic. This means:

1. **The status bar lies.** Token counts displayed to the user are approximate, not truthful.
2. **Budget decisions may be wrong.** If the streaming token estimate diverges from the real count, compaction triggers at the wrong time.

**Specific violations (file:line):**

- `src/claude_ui/claude_render.rs:455` — `add_output_tokens()`: `chars / 4 + 1`
- `src/claude_ui/claude_render.rs:756` — `append_thinking()`: `text.len() / 4 + 1`
- `src/claude_ui/claude_render.rs:801` — `append_content()`: `text.len() / 4 + 1`
- `src/claude_ui/claude_render.rs:463` — `inc_output_tokens()`: `delta_chars / 3`
- `src/claude_ui/claude_render.rs:635` — `UserSubmitted`: `content.len() / 2`
- `src/ui/ui_terminal.rs:929-930` — `draw_claude()`: `streaming.thinking.len() / 4 + streaming.content.len() / 4`
- `src/app_chat_loop.rs:1208-1218` — Comment says `~4 chars per token` but then calls `TerminalUI::estimate_tokens()` which does use tiktoken. However the streaming path (above) does not.

## Root Cause

Task 499 added `token_counter.rs` but did not replace every estimation site. The UI streaming path continues to use `len() / 4` because calling tiktoken per-delta would be expensive during streaming.

## Proposed Solution

Phase 1: Replace `len() / 4` in `add_output_tokens()`, `append_thinking()`, `append_content()`, and `inc_output_tokens()` with calls to `crate::token_counter::count_tokens()`.

Phase 2: For the streaming hot-path (`append_thinking`, `append_content`), accumulate text in a buffer and batch-count tokens every 500ms or every 1KB of text, whichever comes first. This avoids per-delta tokenizer calls while still being accurate.

Phase 3: Remove the `UserSubmitted` `content.len() / 2` heuristic — use `count_tokens` directly.

Phase 4: Fix the `draw_claude()` streaming token estimate at line 929-930.

## Acceptance Criteria
- [ ] No production code path uses `len() / 4` or `len() / 3` or `len() / 2` for token estimation
- [ ] All status bar token counts use tiktoken-rs
- [ ] Streaming performance is not materially degraded (batch counting, not per-delta)
- [ ] `cargo test` passes

## Verification Plan
- Unit test: Verify `count_tokens("Hello world")` matches tiktoken expected output
- Integration test: Run the CLI, observe status bar token counts match API-reported values
- Regression test: `cargo build && cargo test`

## Dependencies
- `src/token_counter.rs` must be functional (already is, Task 499)

## Notes
- **Architectural Rule violated:** Rule 13 (Observability) — token counts must be truthful
- **AGENTS.md violated:** "Accurate reporting: token counts, line counts, and status metrics are truthful, not approximate"
- Dimension F: Truthfulness — "Token estimates using `len / 4` instead of actual tokenizers"
