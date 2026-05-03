# Task 607: Enforce duplicate tool call suppression (not just detect)

## Type

Bug (Model Robustness)

## Severity

High

## Scope

Tool-specific (tool_loop)

## Session Evidence

**Session:** `s_1777837069_544875000`, turn 2 ("read GEMINI.md and summarize it")
**Model:** Huihui-Qwen3.5-4B
**Budget:** 6 iterations for INVESTIGATE complexity

From `trace_debug.log`:

```
trace: tool_loop: iteration 1/6
[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing
trace: tool_loop: tool_failures=1

trace: tool_loop: iteration 2/6
trace: tool_loop: stagnation run 1 (tool: read) (no new tool signal)
trace: tool_loop: duplicate detected (previous failure) signal=read:GEMINI.md
[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing
trace: tool_loop: tool_failures=2

trace: tool_loop: iteration 3/6
trace: tool_call: shell command=cat GEMINI.md
trace: tool_loop: tool_failures=3

trace: tool_loop: iteration 4/6
trace: tool_loop: stagnation run 1 (tool: shell) (no new tool signal)
trace: tool_loop: duplicate detected (previous failure) signal=shell:cat GEMINI.md
trace: tool_call: shell command=cat GEMINI.md
trace: tool_loop: tool_failures=4
```

4 of 6 budget iterations wasted on duplicate failed calls. The stagnation detector logged "duplicate detected" but did NOT prevent the duplicate from executing.

From `session.md`:
```
> TOOL FAIL [read] id=KAzI8frk: Argument validation failed: filePath: required field 'filePath' is missing
> TOOL FAIL [read] id=MsK0frjv: Argument validation failed: filePath: required field 'filePath' is missing
> TOOL FAIL [shell] id=1pLx2X7j: Command failed: cat: GEMINI.md: No such file or directory
> TOOL FAIL [shell] id=9MXdSj1F: Command failed: cat: GEMINI.md: No such file or directory
```

## Problem

The tool loop detects duplicate calls to the same tool with the same signal but still executes them. This wastes iteration budget — 4 of 6 iterations in this turn were useless duplicates. When the model is small (4B), it cannot parse error messages to fix its calls, so it repeats them identically.

The cascade: duplicate waste → budget exhaustion → empty final answer.

## Root Cause Hypothesis

**Confirmed:** The stagnation/deduplication logic in `src/tool_loop.rs` (or wherever tool calls are scheduled) detects duplicates but does not suppress them. The detection is purely observational (logging `"duplicate detected (previous failure)"`) without enforcement.

The trace line `tool_loop: duplicate detected (previous failure) signal=read:GEMINI.md` proves the system knows this is a duplicate before executing it, yet proceeds anyway.

## Proposed Solution

In `tool_loop.rs`, when a tool call is identified as a duplicate of a previously failed call (same tool name + same arguments/signal within the same turn):

1. **Block execution** of the duplicate call entirely
2. **Inject a preemptive failure message** into the next model cycle: "Your previous call to `{tool}` with `{args}` failed. The error was: `{previous_error}`. You must change your arguments or use a different tool/approach."
3. **Increment stagnation counter** but skip actual tool execution
4. **Do NOT count this as an iteration** (or at minimum, don't double-charge the budget)

Files to change:
- `src/tool_loop.rs` — where tool calls are iterated and executed
- The check should compare: tool_name + canonicalized arguments + previous_failure flag

For small models that can't self-correct: after N consecutive duplicate attempts, inject explicit instruction with the EXACT correct JSON format. Example:
```
Your last 2 attempts to call `read` failed because you omitted the required `filePath` field.
Here is the correct format you MUST use:
{"filePath": "path/to/file", "limit": 50}
```

## Acceptance Criteria

- [ ] When a tool call is identical to a previously failed call in the same turn, it is NOT executed
- [ ] Instead, a synthetic error/instruction message is injected into the model's next cycle context
- [ ] The budget counter is not charged for suppressed duplicates (or charged once, not twice)
- [ ] After suppressed duplicates, the model receives explicit format correction guidance
- [ ] The existing stagnation timeout still applies (don't suppress forever)

## Verification Plan

- Write a unit test in `tool_loop.rs` that creates a tool loop with a previously failed `read` call and verifies the duplicate is detected AND suppressed
- Replay session `s_1777837069_544875000` with the fix — verify no duplicate tool calls are executed
- Verify budget is not wasted: 6 iterations should yield more useful work instead of 4 wasted cycles

## Dependencies

None.

## Notes

This is related to Task 559 (stagnation detection) but goes further: detection alone is insufficient for small models. Detection must lead to enforcement, with explicit error-format feedback injected into context. Small models cannot infer correct JSON structure from "required field missing" — they need the exact format shown in the error feedback.
