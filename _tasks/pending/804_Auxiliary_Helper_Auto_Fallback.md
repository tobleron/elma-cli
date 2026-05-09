# Task 804: Auxiliary Helper Auto-Fallback for Small Models

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
When the primary model is small (e.g., 4B parameters) and the auxiliary helper is disabled, the system should either warn the user or auto-enable a fallback reasoning path. In the session, `auxiliary_helper_disabled` was logged on every iteration, and the 4B model was severely underpowered for autonomous multi-step work.

## Root Cause
The auxiliary helper LLM was disabled (`runtime.auxiliary.enabled = false`), forcing a tiny 4B model to do all reasoning, planning, and tool selection alone. The model doesn't receive any warning that it's operating without the helper.

## Requirements
- On startup, check the primary model's parameter count (from config or model metadata).
- If the model is small (< 7B) AND the auxiliary helper is disabled, emit a warning: "Primary model is small (X B parameters) and auxiliary helper is disabled. Multi-step tasks may fail. Consider enabling the auxiliary helper with `elma-cli config set runtime.auxiliary.enabled true`."
- Add a "reasoning budget" adjustment: for small models without auxiliary, reduce max iterations per turn and increase stagnation sensitivity.
- Alternatively, auto-enable a lightweight local reasoning path for small models.

## Failure Mode Fixed
- Small model attempting impossible multi-step tasks
- User unaware that auxiliary helper is needed for reliable operation
