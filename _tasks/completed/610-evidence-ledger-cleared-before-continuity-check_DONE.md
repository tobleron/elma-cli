# Task 610: Evidence ledger cleared before continuity check causes false low score

## Type

Bug

## Severity

Critical

## Scope

System-wide (orchestration + continuity)

## Session Evidence

**Session:** `s_1777840173_315323000`, turn 2 ("what day of the week is today?")
**Model:** Huihui-Qwen3.5-4B

From `trace_debug.log`:
```
trace: tool_call: shell command=date +%A
trace: [audit] command=date +%A success=true output_len=6
trace: tool_calling_pipeline: answer_len=16 iterations=2 tool_calls=1 stopped=false
trace: continuity_score=0.78 needs_fallback=false last_stage=finalization
```

The shell tool successfully gathered evidence ("Sunday" from `date +%A`). The evidence finalizer produced the correct answer "Today is Sunday." (16 chars). But the continuity score is 0.78 instead of 1.0.

From `reasoning_audit.jsonl` line 2 — the continuity retry model call:
```json
{"final_text":"<think>...</think>\n<tool_call>\nname: \"date\"\nargs: {}\n</tool_call>", ...}
```
The retry model responded with tool proposals instead of an improved answer. A 120s timeout model call was wasted.

The session from the prior analysis (`s_1777839764_891723000`) exhibits the same pattern: continuity_score=0.78, retry triggers, hallucinated answer replaces correct answer.

## Problem

The continuity score is incorrectly computed as 0.78 for a perfectly correct answer. This happens because `has_evidence` is `false` when `check_final_answer` runs, even though evidence WAS gathered by the tool loop.

The score breakdown:
- initialization: Aligned (1.0 × weight 1.0)
- routing: Aligned (1.0 × weight 1.5)
- finalization: Drifted "no supporting evidence" (0.5 × weight 2.0)
- Total: 3.5 / 4.5 = 0.78

This Drifted checkpoint fires at `continuity.rs:234-247` because `has_evidence=false` and the original request has more than 3 words.

The false low score triggers an unnecessary continuity retry (hardcoded 0.85 threshold), wasting a 120s model call. For small models, the retry often produces garbage (tool proposals, hallucinations) that can overwrite the correct answer.

## Root Cause Hypothesis

**Confirmed:** `clear_session_ledger()` is called in `orchestration_core.rs:158` (end of `run_tool_calling_pipeline`) BEFORE `app_chat_loop.rs:989` checks `has_evidence`:

```
run_tool_calling_pipeline() {
    run_tool_loop()              // initializes ledger, adds evidence
    clear_session_ledger()       // ← CLEARS ALL EVIDENCE
    return (answer, ...)         //
}                                //
                                 //
app_chat_loop {                  //
    has_evidence = get_session_ledger()  // ← RETURNS None (ledger was cleared)
        .map(|l| l.entries_count() > 0)
        .unwrap_or(false);       // false
    check_final_answer(text, false); // has_evidence=false → Drifted
}
```

The evidence ledger was correctly populated during the tool loop, but `clear_session_ledger()` wipes it before the continuity check can use it.

## Proposed Solution

Move `clear_session_ledger()` from `orchestration_core.rs:158` to `app_chat_loop.rs`, after BOTH:
1. The continuity check (`check_final_answer` at line 998)
2. The evidence contradiction correction (`correct_evidence_contradictions` at line 1104, if it uses the ledger)

Files to change:
- `src/orchestration_core.rs`: Remove `clear_session_ledger()` call at line 158
- `src/app_chat_loop.rs`: Add `clear_session_ledger()` after the final answer display processing (after line 1145 or at the end of the turn processing)

## Acceptance Criteria

- [ ] `clear_session_ledger()` is removed from `orchestration_core.rs`
- [ ] `clear_session_ledger()` is called in `app_chat_loop.rs` after continuity and evidence checks complete
- [ ] Replaying session `s_1777840173_315323000` shows `continuity_score=1.0` (not 0.78)
- [ ] No continuity retry is triggered for "what day of the week is today?" (correct simple answer)
- [ ] Evidence ledger is still properly cleared between turns (no cross-turn evidence poisoning)

## Verification Plan

- Unit test: verify that `check_final_answer("Today is Sunday.", true)` with `original_intent = "what day of the week is today?"` produces `Aligned` verdict and score near 1.0
- Integration test: run a tool-calling turn that gathers evidence, verify `has_evidence=true` after `run_tool_calling_pipeline` returns
- Replay session `s_1777840173_315323000` — verify continuity_score=1.0 and no retry triggered

## Dependencies

- Task 611 (continuity retry threshold) — complementary but independent fix

## Notes

This is the primary cause of the "2 replies" and "Thinking proces..." issue the user reported. The sequence was:
1. Correct answer streamed to TUI
2. Evidence ledger cleared
3. Continuity score 0.78 triggers retry
4. Retry model produces thinking content + garbage tool proposals
5. Thinking content leaks due to Task _fix (already fixed by stateful think-block processing)
6. Retry response is non-text, rejected by Task 609 fix
7. Original answer preserved — but retry was still wasteful

With this fix: step 2-3 are eliminated, no retry at all for correct answers.
