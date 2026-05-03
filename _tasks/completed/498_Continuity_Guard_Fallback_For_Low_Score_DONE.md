# Task 498: Add Continuity-Guard Fallback When Score Drops Below Threshold

**Status:** pending
**Priority:** MEDIUM
**Source:** Session s_1777735825_94786000 deep trace analysis (2026-05-02)
**Related:** semantic continuity system in `src/intel_units/intel_units_continuity.rs`

## Evidence From Session

Trace shows continuity scores:
```
turn 0: continuity_score=1.00 needs_fallback=false
turn 1: continuity_score=0.78 needs_fallback=false  
turn 2: continuity_score=0.78 needs_fallback=false
```

Turn 1 scored 0.78 because the model responded with time/day/name which partially matched the user request. Turn 2 scored 0.78 because the model discussed documentation assessment conceptually, which was tangentially related to the user request.

However, at 0.78 the system determined `needs_fallback=false` and accepted the answer as-is. The final answer for turn 2 was a structural analysis of what information was missing rather than an actual comparison of docs vs code. While the continuity score correctly identified the answer was partially off-target, no corrective action was taken.

## Problem

The continuity score detected degradation but the system didn't act on it. The score crossed a qualitative boundary (from 1.0 to 0.78 is a 22% drop) but there is no threshold-based re-response mechanism.

The `ContinuityUnit` only produces a verdict, it doesn't trigger corrective action. The orchestration layer doesn't check the continuity score to decide whether to re-ask the model.

## Fix

### Phase 1: Add continuity threshold guard
- In `orchestration_core.rs` or `tool_loop.rs`, after receiving the final answer and continuity verdict:
  - If `continuity_score < 0.85` AND the model has tool calls remaining, send a re-prompt asking the model to address the gap
  - The re-prompt should include the original user request and the continuity gap explanation
  - Limit re-prompts to 1 per turn to avoid loops

### Phase 2: Make `needs_fallback` more sensitive
- In `ContinuityUnit::post_flight()` or in the fallback logic:
  - Lower the threshold for `needs_fallback` from the current (unknown) to 0.80
  - When `needs_fallback=true`, trigger a clean-context re-ask with the original prompt

### Phase 3: Add gap-explanation to continuity output
- Update `ContinuityVerdictOutput` to include a `gap: String` field explaining what was missing
- Use this gap in re-prompts: "You addressed X but the user also asked about Y. Please address Y."

## Implementation Plan

1. Check current `ContinuityUnit` for `needs_fallback` threshold logic
2. Add a `gap` field to `ContinuityVerdictOutput`
3. In the orchestration loop, after finalization check continuity:
   ```
   if continuity < 0.85 && !already_retried {
       re_prompt_with_gap(original_request, gap_text)
   }
   ```
4. Add a re-prompt counter to prevent infinite loops
5. Add tests for continuity guard triggering and suppression

## Success Criteria

- [ ] When continuity < 0.85, the model is re-prompted with the gap explanation
- [ ] Re-prompt is limited to 1 per turn
- [ ] Continuity output includes a `gap` field
- [ ] Tests verify guard triggers and suppresses correctly
