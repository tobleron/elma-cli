# ⏸️ POSTPONED

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 030: Long-Term Tactical Memory (LTM) for Formulas

## Context
`formula_memory_matcher.toml` allows re-using successful programs. This can be expanded into a more robust learning system.

## Objective
Enhance the formula memory system:
- Save not just successful programs, but also "negative examples" (failed commands and their successful repairs).
- Store "lessons learned" about specific workspace patterns.
- Improve retrieval based on semantic similarity of the user request.

## Success Criteria
- System "learns" from previous mistakes (e.g., doesn't repeat a failed shell command).
- Faster planning for recurring task types.
