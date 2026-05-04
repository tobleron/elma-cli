# Task 620: Evidence finalizer HTTP decode failure → lossy fallback

## Type

Bug

## Severity

High

## Scope

System-wide (tool_loop finalization)

## Session Evidence

**Session:** `s_1777885188_226830000`, single turn
**Model:** Huihui-Qwen3.5-4B

From `trace_debug.log`:
```
trace: tool_loop: routing voluntary stop through evidence finalizer (Task 601)
trace: finalization_failed_nonfatal stage=evidence error=error decoding response body
trace: tool_calling_pipeline: answer_len=196 iterations=2 tool_calls=1 stopped=false
```

The evidence finalizer model call produced a response that couldn't be decoded ("error decoding response body" — likely a JSON parse failure or SSE parse failure). The `finalize_from_evidence_or_fallback` function caught the error and fell back to `build_fallback_from_recent_tool_evidence`.

The fallback produced: "Based on the evidence gathered: _knowledge_base/_source_code_agents/Roo-Code/apps/web-roo-code/public/logos/gemini.svg" — a file NOT in `project_tmp/`. The user asked about `project_tmp` specifically but got a result from a different directory.

The glob `**/GEMINI*` found MANY files across the workspace (including `project_tmp/GEMINI.md`). The evidence was there but the finalizer failed to parse it into a meaningful answer. The fallback just grabbed one line from the output — an unrepresentative one.

## Problem

When the evidence finalizer model call fails (HTTP error, bad response, decode error), the system falls back to `build_fallback_from_recent_tool_evidence` which produces a simplistic "Based on the evidence gathered: <first line of some tool output>". This is:
1. Often wrong/misleading (picks wrong evidence line)
2. Doesn't summarize or count — just dumps raw output
3. Provides no indication that the evidence finalizer failed
4. Gives the user no way to know the answer is unreliable

## Root Cause Hypothesis

**Likely:** The 4B model produced a response that the SSE parser couldn't decode (malformed JSON chunk, truncated stream, or rate limiting). The error is caught and the fallback runs, but the fallback is too primitive.

## Proposed Solution

### Part A: Retry evidence finalizer with simpler prompt
If the evidence finalizer fails, retry once with a NO-streaming, plain-text prompt: "Answer: <simple answer based on evidence>".

### Part B: Improve fallback aggregation
Instead of picking one random tool output line, aggregate all evidence:
- Count files that match the user's criteria (e.g., count files under `project_tmp/` with "GEMINI")
- Show the count + example files
- Flag which evidence was used

### Part C: Surface finalization failure to user
Add a notice: "⚠️ Evidence finalization failed; showing raw evidence." so the user knows the answer may be incomplete.

Files to change:
- `src/tool_loop.rs` — `finalize_from_evidence_or_fallback`
- `src/tool_loop.rs` — `build_fallback_from_recent_tool_evidence` (improve aggregation)

## Acceptance Criteria

- [ ] Evidence finalizer failure is retried once with simpler prompt
- [ ] Fallback answer aggregates evidence instead of picking one line
- [ ] User receives a notice when fallback is used

## Dependencies

None.
