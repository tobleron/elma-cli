# Task 666: Session Forensics Runner From Fix Task

**Status:** pending
**Priority:** HIGH
**Type:** Observability / Test Coverage
**Scope:** `_tasks/_fix.md`, `src/session_*`, `src/event_log.rs`, `_scripts/`
**Source:** user request to process `_fix.md` after running complex prompts

## Summary

Turn `_tasks/_fix.md` into a repeatable local session-forensics runner that audits latest or specified sessions and creates evidence-backed tasks.

## Evidence And Gap

- `_tasks/_fix.md` defines a thorough manual procedure for session reconstruction and task generation.
- The procedure still references DSL terminology and is not automated.
- User asked to run complex prompts and create tasks for issues found by processing `_fix.md`.

## Implementation Plan

1. Update `_fix.md` terminology to strict JSON/tool-call architecture.
2. Add a script or command that locates latest session, reconstructs timeline, checks required artifacts, and emits a report.
3. Make task creation optional/dry-run first; never create speculative tasks without evidence.
4. Deduplicate against existing pending/active/completed tasks.

## Acceptance Criteria

- [ ] The forensics workflow can run on any session directory.
- [ ] Missing artifacts become explicit observability findings.
- [ ] Generated task proposals include evidence, root cause hypothesis, affected files, and verification.
- [ ] DSL references are removed or marked as stale history.

## Verification Plan

Run the runner against the complex prompt sessions created during this audit and inspect generated findings.

