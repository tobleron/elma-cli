# Task 737: Adaptive Context Retention And Budget-Aware Turn Summaries

## Type

Reliability / Context Management / Semantic Continuity

## Severity

Critical

## Scope

`src/app_chat_loop.rs`, `src/effective_history.rs`, `src/auto_compact.rs`, `src/session_write.rs`, token budgeting, turn summaries, compaction notices

## Problem

Elma currently spawns a turn summarizer after every turn and applies the latest pending summary at the start of the next turn. The apply path marks earlier messages as summarized and removes them from the effective model history even when the active endpoint has a large context window. This means conversation context can shrink on every turn despite 128k to 256k tokens being available.

This can reduce accuracy because the model receives a summary instead of the user's exact earlier wording, tool details, intermediate constraints, and instruction nuance. Turn summaries are useful for resume and emergency compaction, but they must not destructively replace raw context unless token pressure actually requires it.

Task 740, which improves token counting, is necessary but not sufficient. Accurate counts tell Elma how full the window is; this task decides when raw context may be compacted.

## Current Evidence

- `src/app_chat_loop.rs` applies `load_pending_turn_summary()` at the start of a turn and calls `mark_summarized()` on messages before the boundary.
- `src/effective_history.rs` excludes every message with `summarized = true`.
- `src/app_chat_loop.rs` spawns a background `TurnSummaryUnit` after each turn regardless of context pressure.
- The summary is injected as a compact system message, which replaces exact prior turns in the model's effective history.

## Proposed Solution

Separate **summary generation** from **context replacement**.

1. Continue saving turn summaries as optional session artifacts for resume, traceability, and future compaction.
2. Do not mark raw messages as summarized unless the context budget policy decides compaction is required.
3. Base the compaction decision on the detected model context length, accurate token counts, projected next-turn budget, and preserved safety margin.
4. Preserve raw messages by default on large contexts. For endpoints with 64k+ tokens, keep exact raw turns until real pressure exists.
5. When compaction is required, compact the oldest eligible span first while always preserving:
   - The latest user request exactly.
   - Active user constraints and explicit "must" requirements.
   - Current work graph or goal state.
   - Recent tool failures, stop reasons, and retry evidence.
   - File paths, line numbers, commands, and artifact paths needed for continuity.
6. Add a configurable policy in runtime settings:
   - `summary_generation_enabled`: true by default.
   - `destructive_compaction_enabled`: true by default.
   - `compact_prepare_threshold_pct`: default 65.
   - `compact_apply_threshold_pct`: default 80.
   - `compact_hard_threshold_pct`: default 92.
   - `min_raw_recent_turns`: default 6 for large contexts, 3 for small contexts.
7. Manual `/compact` should still force compaction, but it must produce a visible transcript row explaining what was compacted.
8. If helper LLM is disabled, use the main model or deterministic extractive fallback for compaction only when required; never trigger an auxiliary timeout UI just to create optional summaries.

## Acceptance Criteria

- [ ] Turn summaries may be written to `sessions/<id>/summaries/`, but raw messages remain in effective history while context usage is below the apply threshold.
- [ ] `mark_summarized()` is called only from an explicit compaction path, not from routine next-turn summary application.
- [ ] Large-context endpoints preserve raw prior turns until token pressure or `/compact` requires compaction.
- [ ] Compaction policy uses the detected context window and accurate token counts from Task 740.
- [ ] A visible collapsible transcript row is emitted whenever destructive compaction happens.
- [ ] `session.json` records why compaction happened, tokens before/after, preserved turn count, and the summary artifact path.
- [ ] Active user instructions are not lost after compaction.
- [ ] Helper-LLM-disabled mode does not show stale summary-timeout UI for optional turn summaries.

## Verification Plan

- Unit test: with 128k context and 10 short turns, no raw messages are marked summarized.
- Unit test: with 8k context and projected overflow, oldest eligible messages are summarized and latest constraints remain raw.
- Integration test: user gives a multi-turn task with constraints in turn 1; after several turns, final answer still obeys turn 1 constraints without relying only on summary wording.
- Session artifact test: compaction writes before/after token counts and a transcript row.
- Manual test: `/compact` compacts and explains the action in the transcript.

## Notes

Recommendation for the user-reported issue: the current every-turn destructive summary behavior is too aggressive for large local context windows. Summaries should be available as memory artifacts, but exact raw context should be preferred until there is measurable token pressure.
