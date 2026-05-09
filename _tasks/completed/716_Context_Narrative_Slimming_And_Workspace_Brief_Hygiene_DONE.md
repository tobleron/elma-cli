# Task 716: Context Narrative Slimming And Workspace Brief Hygiene

## Type

Context Architecture / Runtime Efficiency

## Severity

High

## Evidence

Round 6 session workspace briefs include large low-signal areas before task-specific work begins:

- old `project_tmp/elma_round*_terminal.out` files
- old backup directories
- copied session folders
- broad `_knowledge_base` entries
- entire config/model directory listings

Example files:

- `project_tmp/round6_sessions/prompt_01_s_1778140353_875100000/workspace_brief.txt`
- `project_tmp/round6_sessions/prompt_08_s_1778140807_118803000/workspace_brief.txt`

This makes the context narrative longer and more complex than necessary for small and dense coder models.

## Problem

The initial context narrative is workspace-shaped rather than objective-shaped. It exposes generated artifacts, historical test outputs, and broad archive folders that often do not help the current request. This increases token load and makes the model more likely to search irrelevant places.

## Requirements

- Build a minimal turn context packet that separates:
  - user objective
  - current workspace root
  - active project guidance summary
  - task-relevant file map
  - generated/history folders to avoid unless explicitly needed
- Exclude or collapse by default:
  - `project_tmp/round*_sessions`
  - `project_tmp/elma_round*_terminal.out`
  - backup folders
  - `target/`
  - dependency/vendor folders
  - old session dumps
- Keep `_knowledge_base` available but collapsed unless the prompt asks for knowledge-base comparison or agent-source-code research.
- Adapt the brief for source-code tasks versus document/data-analysis tasks without keyword-trigger routing.
- Add trace rows showing why a directory was included, collapsed, or excluded.
- Keep the packet simple and principle-first; do not add large example blocks.

## Acceptance Criteria

- [ ] New sessions no longer show old round outputs and backup folders in the default workspace brief.
- [ ] Source-code prompts start with `src/`, tests, config, docs, and task files as compact sections.
- [ ] Document/data prompts surface document indexes and citation support when appropriate.
- [ ] The session trace records context packet size and inclusion decisions.
- [ ] Tests cover generated-folder exclusion and `_knowledge_base` collapsed-by-default behavior.

