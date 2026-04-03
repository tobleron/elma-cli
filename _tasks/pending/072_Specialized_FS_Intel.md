# Task 072: Specialized Filesystem Intel

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 035: Specialized "File-System" Intel Units

## Context
Shell-based inspection is generic. Specialized parsers for project-specific files (Cargo.toml, package.json, etc.) would be more efficient.

## Objective
Add "Structured Observation" units:
- Implement lightweight Rust parsers for common config formats.
- Provide these structured facts to the planner instead of raw `cat` or `grep` output.
- Reduce token usage and increase accuracy for project-discovery tasks.

## Success Criteria
- Faster and more accurate project summaries.
- Reduced context noise from config file content.
