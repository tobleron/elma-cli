# Task 310: Deferred Pre-Turn Summary — Post-Turn Intel Unit with Effective History Compaction

**Status**: Pending  
**Priority**: High  
**Depends on**: None (additive, no breaking changes)  
**Elma Philosophy**: One intel unit = one job, local-first, small-model-friendly, principle-first prompts, semantic continuity

## Problem

Elma accumulates every raw message in `runtime.messages` across turns. A single tool-heavy turn adds 10-20 messages. Over 6-8 turns the context window fills even on 200K-token models. On small local models (8K-32K), it fills in 2-3 turns.

Current context management:
- `auto_compact.rs` compacts when the window nears the limit — reactive, not proactive
- The compaction summarizer (`apply_compact_with_summarizer`) is a stub that falls back to inline concatenation
- No per-turn summarization exists — raw tool output bloats every turn irreversibly

## Solution: Two-Phase Deferred Pre-Turn Summary

**Phase 1 — Post-Turn (fire-and-forget, zero blocking)**: After Elma displays her response, spawn a background task that calls a new `TurnSummaryUnit` intel unit. The unit reads the current turn's data and writes a summary to disk. Nothing blocks.

**Phase 2 — Pre-Turn (before the next LLM call)**: When the user sends their next message, before the LLM query is built: check if a pending summary exists, replace raw messages from the summarized turn with the single compact summary message.

## Architecture

```
Turn N completes → spawn tokio::task → TurnSummaryUnit → write sessions/<id>/summaries/turn_N_summary.json
User types next prompt → pre-turn check → inject summary as system msg → LLM sees 1 msg instead of 12+
```

### Summary JSON Schema

```json
{
  "turn_number": 0,
  "user_message_excerpt": "Find all unused dependencies",
  "summary_narrative": "User asked to find unused deps. Elma searched Cargo.toml + grep'd imports. Found serde_json, chrono unused. Result presented.",
  "status_category": "completed",
  "tools_used": ["read", "bash"],
  "tool_call_count": 4,
  "formula_used": "maestro",
  "noteworthy": false,
  "errors": [],
  "artifacts_created": ["Cargo.toml"]
}
```

## Implementation Plan

### Part 1: ChatMessage Tagging (`src/types_api.rs`)
- Add `summarized: bool` field (serde default, skip_serializing_if)
- Add `mark_summarized()` and `is_summarized()` helpers

### Part 2: TurnSummaryOutput + TurnSummaryUnit (`src/intel_units/intel_units_turn_summary.rs`)
- New struct `TurnSummaryOutput` with fields: `summary_narrative`, `status_category`, `noteworthy`, `tools_used`, `tool_call_count`, `errors`, `artifacts_created`
- `TurnSummaryUnit` implements `IntelUnit` trait
- `pre_flight`: requires at least `final_text` or `step_results`
- `execute`: builds narrative from context extras, calls LLM via `turn_summary_cfg` profile (max_tokens: 256, timeout: 15s)
- `post_flight`: validates `summary_narrative` and `status_category` present
- `fallback`: constructs minimal summary from available data with `status_category: "partial"`

### Part 3: Effective History Filter (`src/effective_history.rs`)
- `compute_effective_history(messages)` → excludes `summarized=true` messages
- `inject_turn_summary(messages, summary)` → inserts system message with summary narrative

### Part 4: Session Summary Persistence (`src/session_write.rs`)
- `save_turn_summary(session_root, turn_number, summary)` → writes `summaries/turn_N_summary.json`
- `load_pending_turn_summary(session_root)` → finds highest unapplied summary
- `mark_summary_applied(session_root, turn_number)` → tracks in `summaries/applied.json`

### Part 5: Profile Loading
- Add `turn_summary_cfg: Profile` to `LoadedProfiles` (`src/app.rs`)
- Load from `turn_summary.toml` with 3-tier fallback (`src/app_bootstrap_profiles.rs`)
- Add validation in healthcheck (`src/config_healthcheck.rs`)
- Create default `config/defaults/turn_summary.toml`

### Part 6: Module Registration
- `src/intel_units/mod.rs`: add `mod intel_units_turn_summary; pub(crate) use intel_units_turn_summary::*;`

### Part 7: Chat Loop Integration (`src/app_chat_loop.rs`)
- Phase 1: After `save_goal_state()` (~line 1050), spawn `tokio::task` that runs `TurnSummaryUnit`
- Phase 2: At start of loop iteration (~line 700-750), check for pending summary, apply if found

## Files to Create/Modify

| File | Action |
|------|--------|
| `src/intel_units/intel_units_turn_summary.rs` | CREATE |
| `src/effective_history.rs` | CREATE |
| `config/defaults/turn_summary.toml` | CREATE |
| `src/types_api.rs` | MODIFY |
| `src/session_write.rs` | MODIFY |
| `src/app.rs` | MODIFY |
| `src/app_bootstrap_profiles.rs` | MODIFY |
| `src/config_healthcheck.rs` | MODIFY |
| `src/intel_units/mod.rs` | MODIFY |
| `src/app_chat_loop.rs` | MODIFY |

## Verification

1. `cargo build` must pass
2. `cargo test` must pass
3. Unit tests for `TurnSummaryUnit` pre_flight, post_flight, fallback
4. Integration test: run multi-turn conversation, verify summary files created
5. Effective history test: inject summary, verify `compute_effective_history` excludes summarized messages
