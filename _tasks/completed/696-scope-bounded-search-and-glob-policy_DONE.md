# Task 696: Scope-Bounded Search And Glob Policy

## Type

Architecture / Tool Policy

## Severity

High

## Scope

Search, glob, workspace policy, routing scope

## Session Evidence

Prompt testing round 2 repeatedly searched outside the user-requested scope:

- Prompt 01 asked for `src` only, but search results included `project_tmp/elma_round2_01_terminal.out`.
- Prompt 02 asked for configuration files, but final evidence included `_knowledge_base`, `.claude`, `src/evidence_ledger.rs`, and unrelated persisted output.
- Prompt 05 asked for project test files, but glob results included `_knowledge_base/_source_code_agents/...`.
- Prompt 04 duplicate-function analysis found `tool_part_ad` alongside `src/tool_calling.rs`, mixing generated/temporary fragments with source files.

## Problem

Elma allows model-generated search and glob calls to broaden the user's scope. This pollutes evidence, causes false positives, and makes final answers less trustworthy.

## Proposed Solution

Implement scope-aware search/glob constraints:

- Convert routing scope (`focus_paths`, `include_globs`, `exclude_globs`) into enforced tool policy for `search`, `glob`, and shell `rg/find`.
- Exclude `project_tmp`, `sessions`, `_knowledge_base`, generated backups, terminal transcripts, and known scratch fragments unless explicitly requested.
- For prompts that name a directory (`src`, `docs`, `tests`, config files), restrict search roots to that semantic scope.
- Surface scope decisions as collapsible transcript rows.
- Add a trace entry whenever a model-requested search is narrowed or rejected.

## Acceptance Criteria

- [ ] Prompt 01 searches only `src/` and does not match terminal output under `project_tmp`.
- [ ] Prompt 02 searches config/documented config paths, not arbitrary source or knowledge-base files.
- [ ] Prompt 05 excludes `_knowledge_base` unless explicitly requested.
- [ ] Shell `rg/find` commands receive the same scope guard as native search/glob tools.

## Verification Plan

Replay prompts 01, 02, 04, and 05. Verify `session.md` and `trace_debug.log` show scope rows and no out-of-scope result paths.

