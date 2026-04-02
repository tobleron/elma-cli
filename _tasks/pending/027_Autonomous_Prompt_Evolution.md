# ⏸️ POSTPONED

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 031: Autonomous Prompt Evolution

## Context
The `sync_and_upgrade_profiles` mechanism currently uses hardcoded patches. The agent should be able to improve its own prompts based on feedback.

## Objective
Implement a mechanism for autonomous prompt refinement:
- When a `critic` or `outcome_verifier` identifies a recurring reasoning failure, propose a prompt tweak.
- Allow the user to review and "commit" prompt upgrades to their local configuration.

## Success Criteria
- System improves its own performance over time through self-correction.
- Reduced need for manual prompt engineering by developers.
