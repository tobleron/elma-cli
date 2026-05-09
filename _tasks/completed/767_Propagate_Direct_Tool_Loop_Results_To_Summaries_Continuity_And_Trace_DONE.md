# Task 767: Propagate Direct Tool Loop Results To Summaries Continuity And Trace

## Type

Observability / Context Hygiene / Session Forensics

## Severity

High

## Scope

Direct tool loop, orchestration core, turn summarizer, session artifacts

## Problem

The direct tool loop now collects `ToolLoopSummary`, but `run_tool_calling_pipeline` still returns only answer, iterations, tool call count, and stopped flag. The chat loop therefore feeds empty `step_results` to summaries. In the latest session, summaries repeatedly said no tools were executed even when the transcript clearly shows tool calls.

## Root Cause

Direct tool-calling metadata is created inside `tool_loop` and then discarded at the orchestration boundary. Summaries, continuity, and trace reducers cannot see tool failures, duplicate suppressions, stop reason, or coverage.

## Proposed Solution

- Return a structured `ToolCallingPipelineResult` from `run_tool_calling_pipeline`.
- Include answer, iterations, tool call count, stop outcome, loop summary, evidence summary, and required artifacts/scope state.
- Convert the loop summary into `StepResult`-like summarizer input or update the summarizer contract directly.
- Ensure summaries state failed/partial turns accurately.

## Acceptance Criteria

- [ ] Turn summaries for direct tool-calling turns list tools executed.
- [ ] Summaries include failed operations and stop reason.
- [ ] Session JSON contains structured direct-loop metadata.
- [ ] Continuity verifier can use direct-loop metadata instead of only evidence-count heuristics.

## Verification Plan

- Replay latest session and inspect summary markdown files.
- Unit test conversion from `ToolLoopSummary` to summarizer input.
- Confirm no summary says "no tools executed" when tools ran.

## Dependencies

Do before Task 769.

## Notes

This is required for reliable debugging. If the session summary lies, later context and replay analysis become unreliable.

