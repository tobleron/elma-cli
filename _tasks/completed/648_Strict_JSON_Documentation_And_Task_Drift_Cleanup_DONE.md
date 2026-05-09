# Task 648: Strict JSON Documentation And Task Drift Cleanup

**Status:** pending
**Priority:** HIGH
**Type:** Documentation / Architecture
**Scope:** `docs/`, `_tasks/`, `config/`, `src/json_*`, `src/tool_loop.rs`
**Source:** user warning about stale DSL documentation; current strict JSON/tool-calling architecture

## Summary

Audit and update docs/tasks that still describe DSL-era architecture as active, while preserving historical completed-task records as history.

## Evidence And Gap

- `rg "DSL"` finds stale deferred/completed docs and `_tasks/_fix.md` references.
- Some docs still describe route decisions or tool architecture in ways that do not match strict JSON/tool-calling behavior.
- User explicitly said DSL related architecture should now be moved to strict JSON.

## Implementation Plan

1. Mark historical DSL docs as archived, not active architecture.
2. Update active docs to say model-facing execution uses strict JSON/tool calls and compact intel-unit JSON.
3. Remove DSL terminology from new pending tasks and session forensics instructions except as historical context.
4. Add a docs drift check that flags active docs claiming DSL is live.
5. Keep `src/prompt_core.rs` untouched unless the user explicitly approves a separate prompt task.

## Acceptance Criteria

- [ ] Active docs no longer instruct implementers to revive DSL action protocols.
- [ ] `_tasks/_fix.md` analyzes JSON/tool-call payload failures, not DSL migration.
- [ ] Historical completed tasks remain understandable as archived history.
- [ ] A grep/check script identifies new active DSL drift.

## Verification Plan

Run `rg "DSL" docs _tasks` and verify every hit is archived history, explicit non-goal, or a corrected strict JSON reference.

