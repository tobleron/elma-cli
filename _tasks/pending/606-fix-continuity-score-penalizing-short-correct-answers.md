# Task 606: Fix Continuity Score Penalizing Short Correct Answers

## Type

Bug

## Severity

High

## Scope

System-wide

## Session Evidence

Session `s_1777834142_424713000` (2026-05-03 21:49):

- Tool: `date +%A` → `Sunday`
- Evidence finalizer produced 79-char answer
- `continuity_score=0.78` < 0.85 threshold → retry fired
- The 79-char answer was CORRECT: "Today is Sunday. Here's a joke: Why don't scientists trust atoms? Because they make up everything!"

The continuity score of 0.78 flagged a perfectly correct, concise answer as "incomplete" simply because it was short. This triggered the continuity retry which then hallucinated a long, detailed, but completely wrong answer.

## Problem

The `ContinuityTracker.alignment_score` calculation penalizes short answers even when they are complete and correct. A 79-char answer that correctly answers the user's question should NOT get a 0.78 score. The score threshold of 0.85 triggers retry on correct answers.

This is fundamentally a **scoring problem**: the continuity checker conflates "short" with "incomplete." For simple factual queries like "what day is it?" or "tell me a joke," the correct answer IS short.

## Root Cause Hypothesis

**Likely**: The `ContinuityTracker.check_final_answer()` method factors answer length into the alignment score (either directly or indirectly through the intel unit that assesses completeness). Short answers get lower scores regardless of correctness.

**Likely**: The `answer_continuity` intel unit (`src/intel_units/intel_units_continuity.rs`) evaluates whether the answer addresses the user request. For simple questions ("what day of week?"), a short answer IS complete, but the intel unit may not account for question complexity.

**Possible**: The continuity tracker's original task (498) was designed for complex multi-step tasks where short answers ARE a problem. It was not designed for simple factual queries.

## Proposed Solution

### Option A: Skip continuity retry for DIRECT complexity answers (Recommended)

If the task complexity is `DIRECT`, the answer is expected to be short. Skip the continuity retry entirely for DIRECT tasks.

```rust
if complexity != "DIRECT"
    && continuity_tracker.alignment_score < 0.85
    && !already_retried
{
    // continuity retry logic
}
```

### Option B: Add answer-length-aware scoring

Modify the continuity check to account for expected answer length based on intent complexity. For DIRECT tasks, a 50-char answer with tool evidence should get score >= 0.85.

### Option C: Lower the score threshold for short answers

If the answer is below 200 chars AND contains evidence-grounded facts, lower the threshold from 0.85 to 0.70.

## Acceptance Criteria

- [ ] 79-char correct answer to "what day of week?" gets score >= 0.85 (skipping retry)
- [ ] Complex multi-step answers below 0.85 still trigger retry
- [ ] No change to scoring for non-DIRECT tasks

## Verification Plan

1. Test fixture: DIRECT complexity, short correct answer, tool evidence present
2. Verify continuity score >= 0.85
3. Verify continuity retry does NOT fire
4. Test fixture: MULTISTEP complexity, long incomplete answer, no evidence
5. Verify continuity score < 0.85 and retry fires

## Dependencies

Task 605 — partially redundant. If Task 605 is implemented (skip retry after evidence finalizer), this task may not be needed for the specific session case. But this task addresses the general scoring bug that can affect any short correct answer, not just evidence-finalizer answers.

## Notes

The continuity tracker is implemented in `src/continuity.rs`. The `check_final_answer` method calls an intel unit (`answer_continuity`) that likely does the scoring. Additional investigation may be needed to understand exactly how the score is computed.
