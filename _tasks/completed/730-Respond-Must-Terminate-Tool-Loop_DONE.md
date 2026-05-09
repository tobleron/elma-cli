# Task 730: Respond Must Terminate Tool Loop Immediately

## Type

Bug / Model Robustness

## Severity

High

## Scope

System-wide

## Session Evidence

Session `s_1778149997_285478000`, Turn 0 (user: `"hi"`):

**What the model actually generated** (from `tool_respond_0OqzAoW9.txt`):
```
Hello! I'm Elma, your AI assistant. How can I help you today?
```

**What the user actually received** (from `0003_final_answer.md`):
```
Hi there! I've received your message and reviewed the system logs.
Here is a quick summary of what was found:

- The respond tool was triggered.
- It returned a greeting from Elma, your AI assistant.
- Elma asked how she can help you today.

Everything looks good on my end. Ready when you are!
```

**Trace log:**
```
trace: tool_loop: iteration 1/3
trace: tool_loop: 1 tool call(s)
trace: tool_loop: iteration 2/3
trace: tool_loop: stagnation run 1 (no new tool signal)
trace: tool_loop: 1 tool call(s)
trace: tool_calling_pipeline: answer_len=304 iterations=2 tool_calls=2 stopped=false
```

## Problem

The model produced a clean, direct greeting via `respond`. But the tool loop did not stop after the `respond` call — it gave the model another iteration. By the time the budget was exhausted, the conversation context was polluted with tool execution frames. The evidence-grounded finalization pipeline then asked the model to produce a final answer from that degraded context, and the small (4B) model produced a third-person meta-summary:

> *"I've received your message and reviewed the system logs. The respond tool was triggered..."*

The model is describing the conversation instead of participating in it. The user sees a robotic meta-recap instead of the direct greeting the model originally produced.

This happens because:
1. `respond` does not terminate the tool loop — it's treated like any other tool call
2. After respond succeeds, the loop continues and context accumulates tool execution noise
3. Finalization sees the polluted context and produces a meta-summary instead of the respond content

For any task beyond a trivial greeting, this mechanism would similarly degrade the final answer by wrapping it in evidence-recap framing.

## Root Cause Hypothesis

Confirmed: `respond` is not treated as a terminal tool call. `tool_loop.rs` continues iterating after respond succeeds, giving the model another chance to call tools. The model (especially small models) then emits another respond or additional tool calls, polluting the context before finalization runs.

The `request_final_answer_from_evidence` function in `tool_loop.rs` constructs a compact packet with the user request + accumulated tool evidence and asks the model to produce a final answer. When the tool evidence includes respond calls themselves, the model summarizes the evidence rather than using the respond content directly.

## Proposed Solution

**After `respond` succeeds with non-empty content, terminate the tool loop immediately and return the respond content as the final answer.** Do not give the model another iteration.

Implementation plan:

- `src/tool_loop.rs` (around line 2100 where tool results are processed):
  - After executing a `respond` tool call, check if the result was successful and the content is non-empty (at least 3+ chars after trimming).
  - If so, break the iteration loop immediately and use the respond content as the final answer.
  - This should happen BEFORE pushing any additional messages to the conversation — the respond tool result should not be shown to the model again since it will spawn a new request.
  - Stop policy should NOT record stagnation on a respond-only turn.
  
  Specific changes:
  - After the tool result loop where `respond` is executed successfully, set a `respond_terminated = true` flag and `break` the tool call loop.
  - After the tool call loop, check `respond_terminated` and break the outer iteration loop.
  - Return the respond content as the final answer — do not run it through the evidence finalization pipeline.

- `src/tool_calling.rs` `exec_respond`: Ensure respond returns content that can be used directly as the final answer (the respond output IS the answer).

## Acceptance Criteria

- [ ] User says `"hi"` → sees direct greeting from Elma, not a meta-summary about what happened.
- [ ] User asks `"how many docs under docs?"` with a model-generated final answer → still works (respond occurs after tools, not instead).
- [ ] `respond` with empty content does NOT terminate (model still exploring/thinking).
- [ ] Budget is not wasted on extra iterations after respond succeeds.
- [ ] Stagnation detection does not flag respond-only turns.

## Verification Plan

- Manual test: `printf 'hi\n' | elma-cli --no-color` → should say something like "Hello! I'm Elma..." not "I've received your message and reviewed the system logs."
- Unit test: mock tool loop with respond tool → verify loop terminates on respond success.
- Unit test: mock tool loop with `ls` then respond → verify respond still terminates.
- Unit test: mock tool loop with empty respond → verify loop continues.
- Replay session `s_1778149997_285478000` Turn 0 → verify clean greeting returned.

## Dependencies

None.

## Notes

This is closely related to the previous Task 607/720 duplicate suppression work — both address model iteration waste. The core principle: when the model says "I'm done" via `respond`, believe it and stop the loop.
