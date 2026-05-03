# Surface Hidden Stop Reasons In Transcript

## Problem
In session `s_1777807006_86051000`, the agent failed silently due to internal exhaustion. The `trace_debug.log` clearly showed:
- `trace: tool_loop: stopping reason=iteration_limit_reached`
- `trace: finalization_failed_nonfatal stage=evidence error=error decoding response body`

However, the user transcript merely printed a broken, truncated message:
```
Based on the evidence gathered:
- ## Workspace Root
- docs/  (49 item(s))
- [persisted-output]
```

This is a direct violation of **Rule 6 (Prefer Transcript-Native Operational Visibility)**: "Budgeting, routing/formula choice, compaction, stop reasons, and hidden processes must surface as collapsible transcript rows."

## Required Actions
1. **Expose Stop Reasons:** Update the `TerminalUI` event pipeline to capture `iteration_limit_reached` and rendering it as a user-visible system notice. Call `tui.push_stop_notice()` in `tool_loop.rs` when stop policy triggers.
2. **Expose Finalization Errors:** If finalization fails due to a model crash or decoding error, print a clear system error row in the transcript rather than silently returning whatever partial output survived.
3. **Expose Operational Events:** Ensure routing decisions, formula selections, and provider switches are surfaced by calling `tui.push_meta_event("ROUTE", ...)`, `tui.push_meta_event("FORMULA", ...)`, etc.
4. **Graceful Degradation:** The final answer should explicitly state that it ran out of budget/context rather than trying to construct a half-baked answer.
