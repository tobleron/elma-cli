# Task 601: Verify Final Answer Is Consistent With Tool Output Evidence

## Type

Model Robustness

## Severity

Critical

## Scope

System-wide

## Session Evidence

Session `s_1777831852_914805000` artifacts:

- `0002_tool_shell_success.txt`: `Sun May  3 21:11:14 EEST 2026`
- `0003_thinking.txt`: "The date command shows it's Sunday, May 3rd, 2026 at 21:11:14 EEST."
- `tool_glob_66JNBEG4...txt`: Found `project_tmp/GEMINI.md` and `_knowledge_base/.../gemini.md`
- `0005_final_answer.txt`: "I don't have access to the real-time system clock in my context" and "GEMINI.md should be located at: docs/GEMINI.md"

Both claims in the final answer contradict the tool outputs:
- date command returned current time (Sunday) → model claimed no clock access
- glob returned `project_tmp/GEMINI.md` → model guessed `docs/GEMINI.md`

## Problem

The small model's final answer ignores tool outputs it correctly processed in intermediate thinking. The model regresses to guessing from workspace context rather than using concrete evidence from tool execution. This produces a provably wrong final answer that wastes the entire session.

There is no system-level guard that detects contradictions between final answers and gathered evidence.

## Root Cause Hypothesis

**Confirmed**: Small model context fragility. The model has two context sources:
1. The workspace context (injected as system message: workspace.txt, workspace_brief.txt)
2. The tool outputs (shell date result, glob result)

The model's intermediate reasoning (thinking artifacts) correctly uses source 2. But when generating the final answer, it falls back to source 1 (workspace brief) which is more familiar/salient. The final answer prompt does not explicitly remind the model to use tool outputs over initial context.

**Possible**: The tool loop's model call context may place the tool outputs far back in the conversation window, and the small 4B model loses access to them by the time it generates the final answer.

**Possible**: The initial system prompt contains workspace info (macOS version) which the model latches onto as the "real" answer source.

## Proposed Solution

### Option A: Evidence finalizer always runs (Recommended)

The tool loop already has `finalize_from_evidence_or_fallback` which constructs a clean context with only the user prompt + evidence block + "Answer using only the evidence above" instruction. This function is currently only called in stop-policy paths (max iterations, stagnation, repeated failures). It should also be called when the model voluntarily stops calling tools.

Implementation in `tool_loop.rs` at the end of the main loop (when `turn.tool_calls.is_empty()`):

```rust
if turn.tool_calls.is_empty() {
    let evidence_block = build_evidence_block(&messages, &tool_outcomes);
    let clean_prompt = format!(
        "{}\n\n--- Evidence gathered ---\n{}\n--- End evidence ---\n\nAnswer concisely using ONLY the evidence above. \
         Do NOT use your general knowledge or the initial workspace context. \
         Cite specific file paths and values.",
        original_user_request,
        evidence_block
    );
    // Make a quick non-streaming call with clean context
    let final_content = request_clean_final_answer(client, chat_url, model_id, &clean_prompt).await;
    return Ok(ToolLoopResult {
        final_answer: final_content,
        ...
    });
}
```

### Option B: Post-hoc evidence consistency check

After the fetch the final answer from the tool loop, run a lightweight LLM call or rule-based check:

```rust
// Check if final answer contradicts key evidence
fn check_evidence_consistency(final_answer: &str, evidence: &[EvidenceEntry]) -> bool {
    // For each key evidence item, check if answer contradicts or ignores it
}
```

But this is weaker than Option A since it can only detect problems and re-try.

### Option C: Inject evidence into the finalization prompt

Modify the existing request in `tool_loop.rs` around line 1658 or create a new hook point between `turn.tool_calls.is_empty()` and returning `ToolLoopResult`. When no tool calls are produced, route through `finalize_from_evidence_or_fallback` instead of using the raw model output.

This is the lightest change with the highest impact.

## Acceptance Criteria

- [ ] When the model stops calling tools voluntarily, the final answer is generated from a clean context containing only the user prompt + evidence block
- [ ] The evidence block includes ALL tool outputs from the session, not just a summary
- [ ] The clean-context prompt explicitly tells the model to use only evidence, not initial workspace context
- [ ] The model's final answer correctly reflects tool outputs for the session's tools
- [ ] Existing tool-loop paths (stop policy, stagnation, etc.) continue to work

## Verification Plan

1. Create a test fixture where shell `date` returns `Sun May 3` but model answers "I don't know the day"
2. Run the fixture through the tool loop with the fix
3. Verify the final answer contains the day of the week
4. Create a test fixture where glob finds `project_tmp/GEMINI.md` but model guesses `docs/GEMINI.md`
5. Verify the final answer contains `project_tmp/GEMINI.md`

## Dependencies

None. This is a new guard covering a gap in the existing tool loop.

## Notes

The existing `finalize_from_evidence_or_fallback` at `tool_loop.rs:598` already builds the clean evidence context and does the right thing. The key change is to call IT (or a variant) when the model stops producing tools, rather than using the raw model output.

The `request_final_answer_from_evidence` function at `tool_loop.rs:470` already:
1. Strips tool calls from message history
2. Inserts the evidence block
3. Adds a "use only evidence" instruction
4. Streams the result to TUI

We just need to use it in the "model stopped producing tools" path.
