# Task 590: Cross-Cycle Evidence Injection via ToolLoopResult

## Session Evidence
Session `s_1777822834_658323000`: Cycle 1 exhausted 11 iterations (workspace_info → ls docs → 8× read fail → stagnation finalization). Cycle 2 started from scratch with the same workspace_info → ls docs → read fail loop. The `evidence_progress_summary` field on `ToolLoopResult` exists but is not yet wired into the caller that restarts the tool loop.

Trace shows cycle 2 repeated all the steps from cycle 1:
```
trace: tool_loop: starting max_iterations=20  (cycle 2)
trace: tool_loop: iteration 1/20 → workspace_info
trace: tool_loop: iteration 2/20 → ls docs
trace: tool_loop: iteration 3/20 → read (fail)  // cycle 1 already did these!
```

## Problem
The `ToolLoopResult` now has `evidence_progress_summary: Option<String>` (implemented in Task 558) but it's never consumed. When `run_tool_calling_pipeline` restarts the tool loop for a continuity retry, the prior cycle's evidence summary is lost.

## Solution
Wire the cross-cycle evidence injection into `orchestration_core::run_tool_calling_pipeline`:

1. Add an optional `prior_evidence: Option<String>` parameter to `run_tool_calling_pipeline`
2. If provided, prepend it to the `user_message` before starting the tool loop
3. In `app_chat_orchestrator.rs` (the caller at line 50), store the prior `ToolLoopResult.evidence_progress_summary` in `AppRuntime`
4. On continuity retry, pass it to `run_tool_calling_pipeline`

Format of the injected message: 
```
[Previously gathered in a prior attempt]
You already know: <evidence_summary>
Do NOT repeat workspace_info or ls. Continue from: reading the documentation files.
```
