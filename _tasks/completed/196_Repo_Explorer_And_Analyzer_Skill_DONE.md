# Task 196: Repo Explorer And Analyzer Skill

## Priority
P1

## Objective
Add a focused repo-analysis skill that can map repository structure, identify representative entry points, summarize architecture, and present grounded insights.

## Why This Exists
A small local model benefits from a dedicated repo exploration mode instead of a generic “just inspect things” prompt. The repo explorer should gather representative evidence first, then summarize from actual files.

## Required Behavior
- Build a grounded repo overview before making architecture claims.
- Prefer representative files over exhaustive scanning.
- Report findings with concrete file references.
- Stay read-only.

## Output Contract
The skill should be able to produce:
- repository overview
- likely entry points
- key modules and responsibilities
- dependency/config hints
- risky or complex areas
- explicit list of inspected files

## Exploration Strategy
- Start with repository root structure.
- Inspect a bounded set of representative files such as:
  - manifest/build files
  - startup/entry files
  - major module directories
  - task/config/guidance files when relevant
- Prefer `read` and `search` when paths are known.
- Use shell discovery only when needed.

## Required Boundaries
- No broad unsupported claims like “this module handles X” without file evidence.
- No repository-wide deep scan by default if a representative pass is enough.
- No file mutation.

## Acceptance Criteria
- Elma can give a grounded repository overview without broad hallucinated claims.
- Output names inspected files and key findings clearly.
- Output is concise enough to help a constrained model continue work in a later turn.

## Required Tests
- synthetic small repo overview returns file-grounded summary
- output includes at least one inspected file list
- repeated runs do not devolve into broad ungrounded architecture claims


## Completion Note
Completed during Task 204 verification on 2026-04-23.
Verified with the relevant automated checks available in this repo, including `cargo build`, targeted tests, and UI parity or startup checks where applicable.
