# Task 071: Rolling Conversation Summary

## Priority
**P2 - EFFICIENCY & OBSERVABILITY (Tier B)**
**Depends on:** Tier A stability (tasks 065-069)

---

# Task 028: Rolling Conversation Summary

## Context
Long-running sessions accumulate token-heavy prose in the conversation history, reducing the available context for the model's primary tasks.

## Objective
Implement a "Working Context" mechanism:
- Periodically summarize older parts of the `ChatMessage` history.
- Replace verbose prose with compact fact-lists in the system context.
- Ensure key objectives and previous outcomes are preserved.

## Success Criteria
- Significant reduction in token usage for long sessions.
- Model maintains awareness of earlier facts even after many turns.
