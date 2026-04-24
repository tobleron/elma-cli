# Task 193: Project Guidance Loader And Init Scaffold

## Priority
P0

## Objective
Make Elma project-aware in arbitrary folders by loading `AGENTS.md`, `_tasks/TASKS.md`, and the active master task when present, and by adding `/init` to scaffold a portable Elma project layout.

## Why This Exists
Elma needs a consistent project contract in any folder. A fresh session must be able to discover how a project wants Elma to behave without depending on prior chat context.

## Required Behavior
- At startup, Elma must look for and load, in this order:
  1. `AGENTS.md`
  2. `_tasks/TASKS.md`
  3. the active master task referenced by `_tasks/TASKS.md` when it exists
- The loaded guidance must be persisted into the session as a snapshot artifact.
- `/init` must create a usable scaffold in any folder without overwriting existing files.

## Scaffold Output
`/init` must create, when missing:
- `AGENTS.md`
- `_tasks/TASKS.md`
- `_tasks/active/`
- `_tasks/pending/`
- `_tasks/completed/`
- `_tasks/postponed/`
- `_dev-tasks/`
- one starter active master plan

## Starter Template Requirements
The generated scaffold must be portable and minimal.
It must include:
- task numbering rules
- folder meaning
- one starter master plan that describes how to continue planning in that repo
- no repo-specific assumptions copied from `elma-cli` beyond the generic structure

## Discovery Rules
- If `AGENTS.md` exists, load it first.
- If `_tasks/TASKS.md` exists but references a missing active master task, record that clearly in the guidance snapshot instead of failing startup.
- If no files exist, Elma should still run normally and `/init` should remain available.
- Loading guidance must be read-only.

## Session Artifacts
Persist at least:
- `guidance_snapshot.txt` or equivalent human-readable artifact
- trace line indicating which files were loaded and which were missing

## Acceptance Criteria
- Running Elma in a fresh folder plus `/init` produces a usable scaffold.
- Existing folders with `AGENTS.md` and `_tasks` are loaded automatically into the runtime guidance context.
- Existing files are preserved; `/init` only creates missing items.
- Missing referenced active task files do not crash startup.

## Required Tests
- `/init` in an empty temp directory creates the full scaffold
- `/init` in a partially initialized directory only creates missing items
- guidance loading prefers `AGENTS.md` before `_tasks/TASKS.md`
- missing active master task reference is reported cleanly
- startup still succeeds when none of the guidance files exist


## Completion Note
Completed during Task 204 verification on 2026-04-23.
Verified with the relevant automated checks available in this repo, including `cargo build`, targeted tests, and UI parity or startup checks where applicable.
