# Task 608: Empty final answer guard on budget exhaustion

## Type

Bug (Finalization)

## Severity

Critical

## Scope

System-wide (orchestration finalization path)

## Session Evidence

**Session:** `s_1777837069_544875000`, turn 2
**User request:** "read GEMINI.md and summarize it"
**Evidence gathered:** `glob` found file at `project_tmp/GEMINI.md` (iteration 6)
**Pipeline answer:** 598 chars (from `tool_calling_pipeline: answer_len=598`)
**Final answer delivered:** EMPTY STRING (from `session.json` final_answer_prepared event with no output)

From `session.json`:
```json
{"final_answer_prepared": {
    "event_type": "final_answer_prepared",
    "turn_id": "turn_7"
}}
```
No `output` field present. The answer was empty.

From `session.md`: No ELMA: response line appears after the glob output. The session ends with:
```
> **notice:** StopReason Budget limit: iteration_limit_reached
```

From `trace_debug.log`:
```
trace: tool_calling_pipeline: answer_len=598 iterations=7 tool_calls=6 stopped=true
trace: continuity_score=0.78 needs_fallback=false last_stage=finalization
```

## Problem

When budget is exhausted, the system returns an empty final answer despite:
1. The pipeline having produced a 598-character answer
2. Valid evidence having been gathered (glob found the file)
3. The last successful tool calls providing actionable data

The user asked "read GEMINI.md and summarize it." The system found the file but returned NOTHING. This is a silent failure — worse than a wrong answer because the user gets no feedback at all.

The cascade is: duplicate waste (607) → budget exhaustion → continuity retry on imperfect answer → retry destroys answer → empty string returned.

## Root Cause Hypothesis

**Likely:** The continuity retry mechanism (line 1023 in `app_chat_loop.rs`) re-prompts the model with the conversation context. When the continuity retry model response doesn't parse as valid text (e.g., it proposes new tool calls in thinking tags), the code path discards the original pipeline answer and leaves the final answer empty.

**Secondary:** There is no "last resort" best-effort answer generation when budget is exhausted. If all else fails, the system should at minimum say "I found GEMINI.md at project_tmp/GEMINI.md but ran out of iterations before I could read it."

## Proposed Solution

### Part A: Guard continuity retry from destroying the answer

In `src/app_chat_loop.rs`, around the continuity retry block (line 1023+):

1. Before the continuity retry replaces `final_text`, SAVE the original pipeline answer
2. After the retry, if the new text is empty or contains only tool-call proposals (e.g., `search_files`, `<think>` without user text), **revert to the original**
3. Add a check: `if new_text.len() < 10 || new_text.starts_with("<")` → keep original

### Part B: Best-effort finalization fallback

In `src/final_answer.rs` or the orchestration finalization path:

Create a `build_best_effort_answer()` function that triggers when:
- The final answer is empty or < 10 chars
- Evidence exists in the session evidence ledger
- Budget has been exhausted

The function should:
1. Collect the last N evidence entries (glob results, ls output, shell output)
2. Template them into a transparent message: "I ran out of iterations, but here's what I found: ..."
3. Log that a best-effort answer was used (for observability)

## Acceptance Criteria

- [ ] When continuity retry produces empty or garbage text, the original pipeline answer is preserved
- [ ] If the final answer is empty after all processing, a best-effort answer is generated from existing evidence
- [ ] The best-effort answer is clearly marked as such (e.g., "Note: I exhausted my iteration budget before completing this task.")
- [ ] Session trace logs when best-effort fallback is triggered
- [ ] Replaying session `s_1777837069_544875000` produces a non-empty final answer

## Verification Plan

- Unit test: `final_answer.rs` — create empty answer + evidence ledger entries → assert non-empty best-effort answer
- Integration test: simulate budget exhaustion mid-task → verify answer is non-empty and transparent
- Replay session: `s_1777837069_544875000` should produce "I found GEMINI.md at project_tmp/GEMINI.md but ran out of iterations"

## Dependencies

- Task 607 (duplicate suppression) would prevent some but not all budget exhaustion cases

## Notes

The "empty string when exhausted" pattern defeats the whole purpose of having an autonomous agent. A user asking "read X and summarize it" getting nothing back is the worst possible UX outcome. Best-effort partial answers with transparent disclosure of limitations are always better than silence.
