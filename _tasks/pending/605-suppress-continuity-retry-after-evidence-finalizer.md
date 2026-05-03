# Task 605: Suppress Continuity Retry After Evidence Finalizer Answer

## Type

Bug

## Severity

Critical

## Scope

Session-specific

## Session Evidence

Session `s_1777834142_424713000` (2026-05-03 21:49):

- User: "what day of week today? Can you tell me a joke?"
- Tool: `date +%A` → `Sunday` ✓
- Model thinking #0006: "I've already answered the first question - it's Sunday." ✓
- Trace: `routing voluntary stop through evidence finalizer (Task 601)` — evidence finalizer runs
- Trace: `answer_len=79` — evidence finalizer produces 79-char answer (likely: "Today is Sunday. Joke: why don't scientists trust atoms? Because they make up everything!")
- `continuity_score=0.78` — retry fires
- Continuity retry HTTP (120s, 2312 bytes)
- Continuity retry output `0007_final_answer.txt`: **"Today is Wednesday, December 18, 2024"** — HALLUCINATED
- Continuity retry adds: "Since I couldn't call system tools to verify the exact current date/time, I used: The macOS version string from the workspace context, cross-referenced with known macOS build version mappings, Calendar calculation for December 2024"

The evidence finalizer (Task 601) produced a clean, correct, evidence-grounded answer. The continuity retry then:
1. Re-introduced the full conversation context (including workspace context)
2. Asked the model to "provide a more complete answer"
3. The 4B model expanded with hallucinated details (wrong date, fabricated verification method)
4. Replaced the correct answer with fabricated garbage

## Problem

The continuity retry is counterproductive when the answer came from the evidence finalizer. The evidence finalizer builds a clean context (question + evidence block only), strips workspace context, and produces a concise, evidence-grounded answer. The continuity retry then:

1. Takes the full conversation history (ALL messages including workspace brief with "macOS 26.4.1")
2. Asks "provide a more complete answer" 
3. The 4B model expands by fabricating details from the workspace context
4. Replaces the correct concise answer with a hallucinated verbose answer

This is a regression: fixing one problem (Task 601 - clean evidence context) created another (continuity retry re-introduces workspace context and triggers hallucination).

## Root Cause Hypothesis

**Confirmed**: The continuity retry uses `runtime.messages.clone()` (full conversation history) including workspace context, AGENTS.md, and all prior messages. The 4B model, when prompted to "expand" or "provide a more complete answer," leverages this context to fabricate plausibly-sounding details. The correct evidence-grounded answer gets replaced.

**Confirmed**: The evidence finalizer intentionally strips workspace context (line 489-498 in tool_loop.rs: creates clean_messages with only user question + evidence block). The continuity retry undoes this protection.

**Likely**: Short, correct answers (79 chars) get low continuity scores (0.78) even when they are complete. The continuity checker penalizes brevity.

## Proposed Solution

### Option A: Skip continuity retry for evidence-finalizer answers (Recommended)

In `app_chat_loop.rs`, add a flag indicating the answer came from the evidence finalizer. If so, skip the continuity retry entirely — the evidence-grounded answer is already the best we can do.

Implementation:
```rust
// In ToolLoopResult, add a field:
pub(crate) used_evidence_finalizer: bool,

// In tool_loop.rs Task 601 path, set it to true

// In app_chat_loop.rs, skip continuity retry when this field is true:
if !tool_loop_result.used_evidence_finalizer
    && continuity_tracker.alignment_score < 0.85
    && !already_retried
{
    // continuity retry logic
}
```

### Option B: Strip workspace context from continuity retry input

When building the continuity retry message, strip system messages containing workspace context and guidance. Only pass user messages, assistant messages, and tool results.

### Option C: Add minimum-length gate to continuity retry

Don't retry answers below a certain length. Short answers are either correct-and-concise or wrong — but the continuity retry can't fix wrong claims, it can only expand.

## Acceptance Criteria

- [ ] When the evidence finalizer produced the answer, continuity retry does NOT fire
- [ ] The evidence finalizer's 79-char answer is the final answer shown to user
- [ ] When the tool loop model produces the answer directly (no evidence finalizer), continuity retry still fires if score < 0.85
- [ ] No regression in the `ToolLoopResult` struct (backward compatible change)

## Verification Plan

1. Test fixture: model voluntary stop → evidence finalizer → answer (79 chars)
2. Verify continuity retry does NOT fire
3. Verify the evidence finalizer answer is the user-facing message
4. Test fixture: model produces answer directly (no tool evidence) → low score
5. Verify continuity retry still fires (unchanged behavior)

## Dependencies

Task 601 (evidence finalizer) — this task provides the `used_evidence_finalizer` flag.

## Notes

The `ToolLoopResult` struct is at `src/tool_loop.rs:264`. Adding a field here requires updating all `return Ok(ToolLoopResult { ... })` sites. The new field can default to `false` at all existing return sites except the Task 601 path.
