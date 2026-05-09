# Task 797: Stagnation-Forced Strategy Change

## Status
- **Priority:** Critical
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
When the system detects stagnation (repeated tool failures, struggle signals, circuit breakers opening), force a hard strategy change rather than just emitting advisory hints. In the session, `[STRUGGLE]` fired 10+ times and `[TOOL_CIRCUIT_OPEN]` fired, but the agent continued the same failing patterns (glob → fail → glob → fail).

## Root Cause
The stagnation detection system emits hints (`"Decomposition recommended"`, `"Strategy Retry Detected"`) but does not enforce a strategy change. The model ignores the hints.

## Requirements
- After N consecutive `[STRUGGLE]` signals (e.g., 3), inject a mandatory "stop and reassess" checkpoint:
  - Pause the tool loop
  - Summarize what's been tried and what failed
  - Enumerate 3 alternative approaches
  - Force the model to pick a NEW approach before continuing
- When a tool circuit breaker opens (`[TOOL_CIRCUIT_OPEN]`), immediately block that tool for the remainder of the turn.
- Track "strategy retry" count per strategy type and escalate:
  - Count 1-2: advisory hint
  - Count 3-4: hard block + require new strategy
  - Count 5+: terminate turn with structured failure report

## Failure Mode Fixed
- Endless loops despite detected stagnation
- Ignored circuit breaker warnings
- Wasted API calls on doomed strategies
