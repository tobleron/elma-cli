# 568 — Add Context Window Budget Accounting with User Visibility

- **Priority**: Medium
- **Category**: LLM I/O
- **Depends on**: 554 (session-scoped state)
- **Blocks**: None

## Problem Statement

Context window management is handled by `auto_compact.rs` and `CompactTracker` with heuristic token counting (`estimate_tokens()` in `ui_terminal.rs`). The context budget is tracked internally but not surfaced to the user in a usable way. Per AGENTS.md Rule 6, budgeting and compaction decisions must appear as collapsible transcript rows — currently they're mostly trace-only.

Additionally, the token counting uses heuristic estimation (`chars / 4` or similar) rather than actual tokenization, despite `tiktoken-rs` being a dependency.

## Why This Matters for Small Local LLMs

Small models typically have smaller context windows (4K-8K tokens). Budget visibility is critical because:
- Users need to know why the agent seems to "forget" earlier conversation
- Users need to understand why compaction is happening
- The model needs to know its context budget to avoid generating content that will be truncated

## Current Behavior

- Token counting: `TerminalUI::estimate_tokens(&m.content)` — heuristic (chars/4)
- Compaction triggers: `CompactTracker` with configurable thresholds
- User visibility: Budget notices pushed via `tui.push_budget_notice()` but may not persist in transcript
- No per-model token limit awareness: `model_capabilities.rs` has `ctx_max` but it's not consistently used

## Recommended Target Behavior

1. Wire `tiktoken-rs` for accurate token counting (replace heuristic)
2. Surface context budget in transcript rows:
   ```
   ┌ Context budget: 3,200/8,192 tokens (39%) — 4 turns, 12 tool results
   ├ Compaction triggered: auto-compact freed 1,200 tokens
   └ Budget approaching limit: 7,500/8,192 tokens (91%)
   ```
3. Add `context_budget` field to `SessionState` with:
   - `total_tokens: u64`
   - `system_prompt_tokens: u64`
   - `conversation_tokens: u64`
   - `tool_results_tokens: u64`
   - `max_tokens: u64` (from model capabilities)
4. Add budget warning when approaching limit (configurable threshold, default 70%)

## Source Files That Need Modification

- `src/auto_compact.rs` — Update tracker to use accurate token counts, emit transcript events
- `src/token_counter.rs` — Wire tiktoken-rs, replace heuristic
- `src/model_capabilities.rs` — Ensure ctx_max flows to budget tracker
- `src/tool_loop.rs` — Add budget transcript events
- `src/ui_terminal.rs` — Replace `estimate_tokens` with accurate counter

## Step-by-Step Implementation Plan

1. Wire `tiktoken-rs` in `token_counter.rs`:
   ```rust
   pub fn count_tokens(text: &str, model: &str) -> u64 {
       // Use tiktoken-rs with appropriate encoding
   }
   ```
2. Create `ContextBudget` struct in `SessionState`
3. Update `CompactTracker` to use accurate counts
4. Add transcript event emission for budget milestones
5. Add `push_context_budget_event()` to TUI
6. Benchmark: ensure token counting doesn't add noticeable latency

## Recommended Crates

- `tiktoken-rs` — already a dependency (Cargo.toml:77)

## Acceptance Criteria

- Token counting uses tiktoken-rs (not heuristic)
- Budget milestones appear as transcript rows
- Compaction events appear as transcript rows
- Budget warning at 70% usage
- Context budget resets per user turn

## Risks and Migration Notes

- tiktoken-rs may have different counts than the actual model's tokenizer. Document the discrepancy.
- Token counting on every message may be expensive. Cache counts per message.
