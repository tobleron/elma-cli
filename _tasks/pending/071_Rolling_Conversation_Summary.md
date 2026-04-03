# Task 071: Rolling Conversation Summary

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

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
