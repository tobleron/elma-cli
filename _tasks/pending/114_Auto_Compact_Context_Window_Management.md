# Task 114: Auto-Compact (Context Window Management)

## Priority
**P1 — Needed for multi-turn conversations**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 113 (Tool Result Budget) — compact needs to know what's already persisted

## Problem

Elma has no context window management. In a multi-turn conversation:
- Each turn adds: user message + system prompt + tool results + assistant response
- After ~5-10 turns with tool usage, the prompt exceeds the model's context window
- The model either errors (prompt_too_long) or silently forgets earlier context

Claude Code uses a **layered defense**: microcompact → history snip → auto-compact → reactive compact. We need at minimum auto-compact.

## Scope

### 1. Token Counting
- Track approximate token count per message (chars / 3.5 for English text)
- Track running total across the conversation
- Model context window read from profile config or detected at startup

### 2. Auto-Compact Trigger
- Fire when: `token_count >= context_window - BUFFER_TOKENS`
- Default buffer: 3,000 tokens (room for model response + tool calls)
- Configurable via `compact_buffer_tokens` in profile config

### 3. Compaction Strategy
**Option A: Inline Summary (preferred for 3B models)**
- Elma generates a summary itself (no forked agent needed — too expensive)
- Replace early conversation messages with: `[Earlier conversation summary: ...]`
- Keep last 3 turns intact for continuity

**Option B: Forked Summarizer Agent**
- Separate API call to summarize conversation
- More accurate but costs extra tokens per compact
- Only use if model is too weak to self-summarize

### 4. Compact Content Selection
**What to summarize (oldest first):**
1. Old tool results (already handled by Task 113 persistence)
2. Old user messages (replace with summary)
3. Old assistant responses (replace with summary)

**What to always preserve:**
- System prompt (identity + tool definitions)
- Last 2 user messages + responses
- Any active tool results the model is still working with

### 5. Circuit Breaker
- Max 3 consecutive compact failures
- After hitting limit, stop compacting and warn user
- Track: `compact_failure_count`, `last_compact_success_unix_s`

### 6. Integration Points
- `src/tool_loop.rs` — check token budget before each API call, compact if needed
- `src/context_compact.rs` (new) — compaction logic
- `src/types_api.rs` — add token counting helpers
- Profile config: add `compact_buffer_tokens`, `compact_max_failures`

## Design Principles
- **Small-model-first:** Inline summary, not forked agent. 3B models can summarize their own conversation.
- **No death spirals:** Circuit breaker prevents infinite compact → fail → compact loops
- **Principle-first:** Summarize substance, don't just truncate. Preserve active work context.
- **Offline-first:** No network needed for compaction

## Verification
1. `cargo build` clean
2. `cargo test` — token counting, compact trigger, content selection, circuit breaker
3. Real CLI: 15+ turn conversation — verify compact fires, conversation continues
4. Real CLI: verify model retains context after compact (ask about earlier work)
5. Verify compact doesn't fire for short conversations (no unnecessary summarization)

## Acceptance Criteria
- [ ] Token counting tracks approximate usage per message
- [ ] Auto-compact fires when approaching context window limit
- [ ] Conversation continues naturally after compact (model knows what happened)
- [ ] Circuit breaker prevents death spirals (max 3 failures)
- [ ] System prompt and recent messages always preserved
- [ ] User notified when compact fires (subtle trace, not interrupting)
