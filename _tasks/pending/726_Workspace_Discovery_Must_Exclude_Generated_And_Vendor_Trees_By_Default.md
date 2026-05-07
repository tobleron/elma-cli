# Task 726: Workspace Discovery Must Exclude Generated And Vendor Trees By Default

## Type

Workspace Policy / Search And Glob Scope / Context Efficiency

## Severity

High

## Evidence

Round 8 prompts still exposed generated, vendor, and knowledge-base trees in default discovery:

- Prompt 03 `glob **/*.md` returned `.kilo/node_modules/...` README files before project docs.
- Prompt 04 `glob **/*.rs` returned `.trash/.../target/...` files before `src/**/*.rs`.
- Prompt 05 test discovery returned `_knowledge_base/_source_code_agents/...` test files before the local project test structure.
- `workspace_info` still lists heavy non-primary trees such as `_knowledge_base`, old `project_tmp` outputs, backup folders, and session/archive directories.

Affected sessions:

- `sessions/s_1778147182_938803000/session.md`
- `sessions/s_1778147274_655146000/session.md`
- `sessions/s_1778147612_177368000/session.md`

## Problem

Broad discovery pollutes model context with irrelevant files, increases token load, and causes the model to choose poor read targets. This directly contributes to empty read stagnation and weak artifact quality.

Elma should be local-first and enterprise-grade, but default workspace operations must distinguish primary workspace source/docs/tests from generated archives, vendor directories, historical sessions, backups, and `_knowledge_base` reference corpora unless the user explicitly asks for them.

## Requirements

- Apply the same default exclusion policy consistently to `workspace_info`, `glob`, `search`, and any repo-map/discovery tools.
- Exclude by default:
  - `.git`, `target`, `node_modules`, `.kilo`, `.opencode`, `.trash`
  - `sessions`, `project_tmp`, backups, round outputs
  - `_knowledge_base` unless the user explicitly asks to inspect reference agents/source corpora
  - generated snapshot/artifact trees
- Add transcript-native rows when scope is narrowed by default exclusions.
- Allow explicit override through tool args or user intent when the user names excluded trees.
- Add regression tests for prompt-style glob/search queries.

## Likely Files

- `src/workspace_policy.rs`
- `src/workspace.rs`
- `src/tool_calling.rs`
- `elma-tools/src/tools/glob.rs`
- `elma-tools/src/tools/search.rs`
- `src/session_paths.rs`

## Acceptance Criteria

- [ ] `glob **/*.md` prioritizes project docs and excludes `.kilo/node_modules` by default.
- [ ] `glob **/*.rs` excludes `.trash`, `target`, `_knowledge_base`, and generated backup trees by default.
- [ ] The user can still explicitly search `_knowledge_base` when requested.
- [ ] Prompt 03/04/05 traces no longer start from irrelevant generated/vendor paths.

