# Task 611: Continuity retry uses hardcoded 0.85 threshold instead of model-adaptive

## Type

Bug (Model Robustness)

## Severity

Medium

## Scope

System-wide (`app_chat_loop.rs` continuity retry block)

## Session Evidence

**Session:** `s_1777840173_315323000`, turn 2
**Model:** Huihui-Qwen3.5-4B (< 7B params)
**Adaptive model threshold:** 0.65 (from `apply_model_threshold`)
**Hardcoded retry threshold:** 0.85

From `trace_debug.log`:
```
trace: continuity_score=0.78 needs_fallback=false last_stage=finalization
[HTTP_START] timeout=Some(120)s    ← continuity retry triggered despite needs_fallback=false
trace: continuity_retry_rejected: retry response was non-text/too-short (0 chars), keeping original (16 chars)
```

The system correctly computed `needs_fallback=false` (0.78 > 0.65 adaptive threshold), but the continuity retry at `app_chat_loop.rs:1035` uses a hardcoded `alignment_score < 0.85` check that ignores the model-adaptive threshold.

From `reasoning_audit.jsonl` line 2 — the wasted retry model call:
```json
{"final_text":"<think>...</think>\n<tool_call>\nname: \"date\"\nargs: {}\n</tool_call>", "model":"Huihui-Qwen3.5-4B-Claude-4.6-Opus-abliterated.Q6_K.gguf"}
```
The 4B model couldn't produce a meaningful improvement and instead tried to propose new tool calls.

## Problem

`apply_model_threshold` at `continuity.rs:17-31` intelligently adjusts the threshold based on model size:
- Models < 7B → threshold 0.65
- Models 7-20B → threshold 0.72
- Models > 20B → threshold 0.80

This acknowledges that small models produce lower-confidence outputs even when correct. However, the continuity **retry** at `app_chat_loop.rs:1035` ignores this adaptive threshold and uses a hardcoded `0.85`:

```rust
if !is_direct
    && continuity_tracker.alignment_score < 0.85  // ← hardcoded, should be tracker.threshold
    && !already_retried
```

This means:
1. For a 4B model with threshold 0.65, score 0.78 passes `needs_fallback()` but still triggers retry
2. The retry wastes a model call (120s timeout, compute, context window)
3. Small models rarely produce useful improvements — they tend to propose more tools instead
4. Combined with Task 610 (evidence ledger cleared), the retry rate is artificially high

## Root Cause Hypothesis

**Confirmed:** Hardcoded `0.85` at `app_chat_loop.rs:1035` does not use `continuity_tracker.threshold` (set by `apply_model_threshold`). The retry should respect the same adaptive threshold that `needs_fallback()` uses.

## Proposed Solution

In `src/app_chat_loop.rs`, change line 1035 from:
```rust
&& continuity_tracker.alignment_score < 0.85
```
to:
```rust
&& continuity_tracker.needs_fallback()
```
This delegates the decision to the model-adaptive threshold already configured.

Alternative: use `continuity_tracker.threshold` directly:
```rust
&& continuity_tracker.alignment_score < continuity_tracker.threshold
```

The first approach (`needs_fallback()`) is cleaner — it uses the same check as the rest of the system. However, note that for a 4B model, threshold 0.65 means retry would NOT fire for score 0.78 (which is correct — the answer IS correct).

## Acceptance Criteria

- [ ] Continuity retry at `app_chat_loop.rs:1035` uses `continuity_tracker.needs_fallback()` (or `continuity_tracker.threshold`)
- [ ] For a 4B model with score 0.78, no continuity retry is triggered (0.78 > 0.65)
- [ ] For a large model with score 0.78, retry IS triggered (0.78 < 0.80)
- [ ] `trace_debug.log` no longer shows continuity retry for correct short answers from small models

## Verification Plan

- Unit test: Create `ContinuityTracker` with model threshold 0.65, set alignment_score to 0.78, verify `needs_fallback()` returns `false` and retry is skipped
- Integration test: Run a simple factual question with a 4B model, verify no continuity retry is triggered
- Replay session `s_1777840173_315323000` — verify no 120s timeout call appears in the trace

## Dependencies

- Task 610 (evidence ledger not cleared) — fixes the root cause of false low scores, but this task prevents the symptom even when scores are legitimately low for small models

## Notes

This is purely a defensive fix. If Task 610 is fixed first, scores will be correct (1.0 for correct answers), and the retry won't fire regardless of threshold. However, fixing the hardcoded threshold is still valuable for cases where the score IS legitimately low due to other reasons — the retry should still be adaptive to model capability.
