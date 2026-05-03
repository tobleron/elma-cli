# Task 071: Rolling Conversation Summary

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Priority
**P2 - EFFICIENCY & OBSERVABILITY (Tier B)**
**Depends on:** Tier A stability (tasks 065-069)

---

# Task 549: Rolling Conversation Summary

## Context
Long-running sessions accumulate token-heavy prose in the conversation history, reducing the available context for the model's primary tasks.

**Recent Incident Context:** In session `s_1777807006_86051000`, the system crashed with an `error decoding response body` partly because the budget boundaries were not tightly enforced. By rolling the conversation summary forward, we ensure that deep investigation tasks always have the maximum possible context room for reading files and holding evidence, rather than wasting space on greetings and old tool calls.

## Objective
Implement a "Working Context" mechanism:
- Periodically summarize older parts of the `ChatMessage` history.
- Replace verbose prose with compact fact-lists in the system context.
- Ensure key objectives and previous outcomes are preserved.

## Success Criteria
- Significant reduction in token usage for long sessions.
- Model maintains awareness of earlier facts even after many turns.
