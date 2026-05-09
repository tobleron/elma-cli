# Task 765: Add Workspace Path Resolution And Failed Path Recovery

## Type

Tool Robustness / Workspace Intelligence / Recovery

## Severity

High

## Scope

Path tools, workspace discovery, tool repair, stop policy

## Problem

The user asked about `tasks/completed`, but the real path is `_tasks/completed`. Elma tried the missing path, searched broadly, scanned irrelevant `_knowledge_base` results, read `src/project_init.rs`, and still failed to resolve the path. This caused iteration exhaustion and a wrong final answer.

## Root Cause

Failed path recovery relies on model self-correction and broad search output. There is no workspace-aware resolver that can rank likely path candidates after a path miss.

## Proposed Solution

- Add a workspace path resolver that runs after path-not-found failures.
- Use current workspace tree, ignored directories, path suffix similarity, directory basename similarity, and unique candidate detection.
- When exactly one strong candidate exists, inject a structured recovery suggestion or auto-retry the tool with the resolved path.
- When multiple candidates exist, ask the model to choose from a bounded list.
- Exclude generated/vendor/reference trees by default unless the current objective explicitly scopes into them.

## Acceptance Criteria

- [ ] `tasks/completed` resolves to `_tasks/completed` in this repo.
- [ ] Missing path recovery does not search `_knowledge_base` unless relevant.
- [ ] Repeated missing-path failures change strategy before iteration limit.
- [ ] Trace shows original path, candidate paths, selected recovery, and confidence.

## Verification Plan

- Unit tests for path candidate ranking.
- Integration fixture with `foo/bar` requested and `_foo/bar` existing.
- Replay latest completed-task prompt and assert it reads `_tasks/completed`.

## Dependencies

Coordinate with Task 763.

## Notes

This is deterministic filesystem recovery, not request keyword routing. It should operate on path evidence and workspace structure.

