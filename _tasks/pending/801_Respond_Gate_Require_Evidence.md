# Task 801: Respond Gate — Require Evidence Before Answering

## Status
- **Priority:** Critical
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Prevent the `respond` tool from being called without at least one successful evidence-gathering tool call. In the session, the agent called `respond` 5+ times with fabricated/assumed information, triggering `respond_abuse` stops.

## Root Cause
The model can call `respond` at any time, even with zero evidence. It fabricates answers rather than admitting it hasn't gathered any information.

## Requirements
- Implement a gate: track a counter of "successful non-respond tool calls" per turn.
- If counter == 0, block `respond` and inject: "You haven't gathered any evidence yet. Run at least one tool (read, ls, glob, search, shell) before answering."
- If all tool calls failed, `respond` should be allowed but must include an honest admission: "I attempted X, Y, Z but all failed. Here's what I know..."
- Add a `coverage` check: if coverage is below a threshold (e.g., < 5% of expected), warn before allowing respond.

## Failure Mode Fixed
- Respond abuse (fabricated answers with no evidence)
- Premature termination with no useful output
