# Task 429: Fix Hallucinated Factual Answers And Evidence Gate Gaps

**Status:** Complete
**Priority:** HIGH (directly addressed session hallucination bug)

## Problem

In the session `s_1777669577_889988000`, Elma answered "what time is it now?" with `Sun Dec 21 15:30:42 UTC 2025` — a hallucinated date 4 months in the past. The correct date was May 2, 2026.

Root causes:
1. **Route misclassification**: "what time is it now?" → 100% CHAT route (zero probability for SHELL). No shell tools available.
2. **Evidence gate gap**: With zero evidence entries (model never called tools), the gate didn't fire despite factual fabrication.
3. **Trace-only verdicts**: Evidence grounding checks were only logged to trace — invisible to the user.

## What Was Built

### Route Override (routing_infer.rs)
- Added factual query detection for time/date patterns ("what time is it", "current time", etc.)
- When detected, overrides CHAT short-circuit to SHELL route with `evidence_required: true`
- Source set to `chat_factual_override` for debugging visibility

### Evidence Gate V2 (tool_loop.rs)
- When zero evidence entries exist, checks respond content for datetime fabrication patterns
- `has_factual_content()` — narrowly scoped to datetime only (avoids blocking benign chat)
- Blocks respond with an ungrounded-claims correction message
- Fires EVIDENCE transcript event

### Chat Tools (registry.rs)
- Added `tool_search` and `update_todo_list` to CHAT context tools  
- Model can now discover `shell` via tool_search when needed in CHAT mode

### Transcript Visibility
- EVIDENCE verdicts now `push_meta_event("EVIDENCE", msg)` instead of hidden trace logs
- SHELL transcript events: `push_meta_event("SHELL", command)` on every shell call

## Verification

```bash
cargo test  # 816 tests pass
```

Manual probes:
- `what time is it now?` → routes to SHELL, calls `date`, returns correct time
  - Output: `Sat May  2 00:30:14 EEST 2026` ✅
- `hi` → routes to CHAT, responds with greeting ✅
  - No false evidence gate triggers for benign self-introductions

## Unfixed Issues

- Summary intel unit still fails with "URL error: relative URL without a base" — this is a profile config issue, not addressed by Wave 3 tasks
- Model still occasionally gets stuck in thinking loops when evidence_required gate fires — the gate needs a retry timeout
