# Task 691: Workspace Scope And Output Volume Control

## Type

Performance

## Severity

High

## Scope

System-wide

## Session Evidence

Several prompts scanned outside the intended project scope or produced huge outputs:

- Prompt 03 `sessions/s_1778084708_633588000/session.md`: first glob `**/*.md` returned `.kilo/node_modules/...` before narrowing to `docs/**/*.md`.
- Prompt 04 `sessions/s_1778084857_555628000/session.md`: first glob `**/*.rs` returned `.trash/...` files before narrowing to `src/**/*.rs`.
- Prompt 06 `sessions/s_1778085073_737796000/trace_debug.log`: search results persisted a 12,480,008-character artifact.
- Prompt 07 `sessions/s_1778085464_714465000/session.md`: searching TODO comments matched `project_tmp/elma_test_02_terminal.out`, not just source code.
- Prompt 08 backup copied files from broad workspace paths into `project_tmp/backup_20260506_193943`, including many files from external source-agent knowledge-base trees.

## Problem

Unbounded workspace scans waste context, increase latency, introduce irrelevant evidence, and can cause incorrect outputs. Elma needs project-aware scope control, especially for source/doc/config/test prompts.

## Root Cause Hypothesis

Confirmed: default search/glob/shell paths include generated outputs, node_modules, trash, project_tmp logs, and `_knowledge_base` unless the model manually excludes them.

Likely: workspace policy is not consistently injected into tool calls and shell commands.

## Proposed Solution

Implement deterministic scope policy for common local tasks:

- Inspect `src/workspace_policy.rs`, `src/tool_calling.rs`, `src/tools/helpers.rs`, `elma-tools/src/tools/search.rs`, `elma-tools/src/tools/glob.rs`, and shell preflight modules.
- Define default excluded paths for generated/cache/vendor/session/testing outputs: `.git`, `target`, `.trash`, `.kilo/node_modules`, `.opencode/node_modules`, `project_tmp`, `sessions`, and `_knowledge_base` unless explicitly requested.
- Apply the policy in `search`, `glob`, `observe`, and shell command review.
- Add a transcript notice when a broad scan is narrowed by policy.
- Persist oversized tool output with a compact summary that includes truncation and artifact path.

## Acceptance Criteria

- [ ] `docs/**/*.md` tasks do not first scan node_modules.
- [ ] `src/**/*.rs` tasks do not include `.trash`, `project_tmp`, or `_knowledge_base` unless requested.
- [ ] Tool outputs over a configured threshold are summarized and cited by artifact path.

## Verification Plan

Replay prompts 03, 04, 06, and 07. Confirm session evidence stays within intended scope and no multi-megabyte raw result is injected into active context.

## Dependencies

None.

