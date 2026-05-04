# Task 609: Continuity retry must preserve original answer when retry response is non-text

## Type

Bug (Finalization)

## Severity

Critical

## Scope

System-wide (continuity.rs + app_chat_loop.rs)

## Session Evidence

**Session:** `s_1777837069_544875000`, turn 2
**Pipeline answer:** 598 chars (confirmed by trace)
**Continuity score:** 0.78 (below 0.85 threshold)
**Continuity retry response:** model output contains only `<think>` block suggesting to search for the file — no user-facing text
**Final answer:** EMPTY

From `reasoning_audit.jsonl` line 2 (the continuity retry response):
```json
{
  "final_text": "<think>\nThe user wants me to read GEMINI.md and summarize it. I need to find this file in the workspace and extract its contents for summarization.\n\nLet me search for GEMINI.md in the workspace first.\n</think>\n\n<search_files query=\"GEMINI.md\">",
  "has_reasoning": false,
  "reasoning_text": ""
}
```

The model's response to the continuity retry asks for `search_files` — it does NOT produce an improved answer. It wants to call more tools. The `<think>` block is stripped, leaving only `<search_files query="GEMINI.md">` which is a tool-like tag, not user-facing text.

From `trace_debug.log`:
```
trace: continuity_score=0.78 needs_fallback=false last_stage=finalization
[HTTP_START] ... timeout=Some(120)s
[HTTP_RESPONSE] status=200 OK
[HTTP_BODY] received 932 bytes
[HTTP_SUCCESS] parsed response successfully
trace: memory_gate_status=skip reason=missing_workspace_evidence
[HTTP_START] ... timeout=Some(15)s
```

The 120s timeout model call is the continuity retry. After it, there's a 15s timeout call which appears to be another finalization attempt or turn summary.

From `session.json`:
```json
{"final_answer_prepared": {"event_type": "final_answer_prepared", "turn_id": "turn_7"}}
```
The event has no output — answer is empty.

## Problem

The continuity retry mechanism (Task 498/597) re-prompts the model with `[continuity_retry]` prefix and conversation context. But small models often respond to "expand/enhance" by proposing MORE work rather than producing text:

1. Model proposes new tool calls in pseudo-markdown (`<search_files query="...">`)
2. The system strips thinking tags, leaving only tool-like text
3. The tool-like text is treated as the "answer"
4. The original 598-char pipeline answer is **discarded and replaced**
5. Final answer is empty or garbage

The continuity retry actively **destroys** the pipeline's answer when the retry model response is non-text.

## Root Cause Hypothesis

**Confirmed:** In `app_chat_loop.rs` around line 1023, the continuity retry overwrites `final_text` with the retry model's response without validating that the response is actually a coherent text answer. The code assumes the retry model will produce improved text, but small models frequently respond to "expand" prompts with tool proposals instead.

The flow is:
```
1. answer = pipeline.run()         // 598 chars, valid
2. score = continuity.check(answer)  // 0.78, below 0.85
3. retry_text = model.call("[continuity_retry]...")  // produces tool proposal
4. answer = retry_text             // OVERWRITES with garbage
5. return answer                    // empty/garbage
```

Step 4 is the bug — it unconditionally replaces the answer.

## Proposed Solution

In `src/app_chat_loop.rs`, in the continuity retry block:

```rust
if continuity_tracker.alignment_score < 0.85 && !already_retried {
    let retry_result = continuity_retry();
    
    // Guard: only accept retry text if it's valid user-facing content
    let retry_text = retry_result.cleaned_text();
    let is_valid_answer = retry_text.len() >= 20
        && !retry_text.contains("<search_files")
        && !retry_text.contains("<read ")
        && !retry_text.contains("<write ")
        && !retry_text.contains("<glob ")
        && !retry_text.contains("<shell")
        && !retry_text.contains("<bash")
        && !retry_text.starts_with("<");
    
    if is_valid_answer {
        final_text = retry_text;
        retry_happened = true;
    } else {
        // Keep original pipeline answer; log that retry was rejected
        trace("continuity_retry_rejected: retry response was non-text/too-short, keeping original");
    }
}
```

Also add a guard in `src/continuity.rs`:
- If the continuity score is below threshold but the original answer contains specific markers of being a valid answer (contains actual English sentences, not just tool proposals), raise the threshold or skip retry

## Acceptance Criteria

- [ ] When continuity retry model response is < 20 chars, original answer is preserved
- [ ] When continuity retry model response contains tool pseudo-tags (`<search_files`, `<shell`, etc.), original answer is preserved
- [ ] A trace message is emitted when retry is rejected (for observability)
- [ ] Replaying session `s_1777837069_544875000` does NOT produce an empty final answer
- [ ] The 598-char pipeline answer is delivered to the user

## Verification Plan

- Unit test in `continuity.rs`: mock retry response containing `<search_files>` — verify original is kept
- Unit test in `continuity.rs`: mock retry response containing only `<think>` block — verify original is kept
- Integration test: run session where continuity retry proposes tools → verify final answer is non-empty
- Replay session `s_1777837069_544875000` — verify user gets a non-empty answer

## Dependencies

Related to Task 608 (empty answer guard), but addresses the specific root cause of the continuity retry destruction pattern.

## Notes

This is the direct fix for the cascading failure observed in session `s_1777837069_544875000`. The continuity retry's purpose is to improve answers — but the current implementation assumes the retry will always produce better text. For small models, this assumption is dangerously wrong. The retry must validate its output before accepting it as a replacement.
