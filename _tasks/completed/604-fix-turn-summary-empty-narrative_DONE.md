# Task 604: Fix Turn Summary Empty Narrative After Continuity Retry

## Type

Bug

## Severity

Medium

## Scope

Session-specific

## Session Evidence

Session `s_1777833151_802415000` trace_debug.log:

```
[INTEL_VERBOSE] intel_turn_summary_postflight_failed error=Empty summary narrative
[INTEL_FALLBACK] unit=turn_summary error=post-flight: Empty summary narrative
```

The turn summarizer (runs in tokio::spawn fire-and-forget at `app_chat_loop.rs:1185`) produced an empty narrative. The summary file was created but its content is just the truncated final answer text, not a proper summary.

`summaries/2026-05-03_18-33-31.456680+00-00_summary_1.md`:
```
User asked: "hey there, who is this? ...". Outcome: ### Who is this?
I'm **Elma** — a local-first autonomous CLI agent...
From the project guidance in `AGENTS.md`, Elma's core philosophy includes:
- Reliability before speed
- Adaptive reasoning be
```

The summary is just a truncated copy of the final answer, not an actual summary. The summarizer LLM call likely returned empty content.

## Problem

The turn summarizer intel unit (`TurnSummaryUnit`) returns an empty narrative under some conditions. This wastes an HTTP call (trace shows it ran: 1739 bytes received but summarizer says "Empty summary narrative").

Possible causes:
1. The model's full final answer text is too long for the summarizer's context budget
2. The summarizer prompt doesn't work well with the 4B model
3. The summary content was empty after stripping thinking blocks
4. The post-flight validation rejects the summary as too short/invalid

The summary becomes useless — it's just a truncated copy of the answer, not a narrative summary.

## Root Cause Hypothesis

**Likely**: The `TurnSummaryUnit` post-flight validation (`intel_turn_summary_postflight_failed`) rejects the output because the LLM produced content that doesn't match the expected narrative format. The fallback then stores the truncated final answer text, which is not useful.

**Possible**: The LLM returned content but the extraction/filtering in the post-flight step stripped it to empty.

## Proposed Solution

Investigate `intel_units/intel_units_continuity.rs` (or wherever `TurnSummaryUnit` is defined) and the `post-flight` validation logic. Either:

1. Relax the post-flight validation to accept a wider range of summary formats
2. Fallback to a simple rule-based summary (concatenation of tool call names + first line of final answer) instead of a truncated final answer copy
3. Skip the summarizer LLM call for simple 1-turn factual questions (detectable from the intent annotation)

## Acceptance Criteria

- [ ] Turn summary is a meaningful narrative (not a truncated final answer)
- [ ] No "Empty summary narrative" error in logs
- [ ] Session summaries are useful for session resume context

## Verification Plan

Run a session with the same model and verify:
1. `trace_debug.log` contains no `postflight_failed` errors
2. `summaries/` directory contains a proper narrative summary
3. `session.json` turn_summaries entry contains a narrative, not truncated answer text

## Dependencies

None.

## Notes

The `TurnSummaryUnit` is defined in `src/intel_units/` — check `intel_units_continuity.rs` or nearby files. The post-flight validation likely rejects empty/too-short content.
