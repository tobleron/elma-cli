# Task 705: Safe Backup Copy Tool And Verification Workflow

## Type

Tooling / Shell Safety / Transactions

## Severity

High

## Session Evidence

Prompt 08 was interrupted after a futile loop:

- `copy` failed and repair injected `path='ago)'`.
- Elma used a shell command with `csplit`, creating backup directories that did not preserve source files correctly.
- Safer `while read ... cp` commands were blocked by shell preflight as bulk destructive.
- Shell circuit opened after three blocked attempts, then the loop continued with bogus `copy` repairs.

Evidence:

- `project_tmp/round3_sessions/prompt_08_s_1778094994_729906000_interrupted/trace_debug.log`
- generated directories `project_tmp/backup_20260506_221725` and `project_tmp/backup_20260506_221844`

## Problem

Backups are a common autonomous-agent task and should not depend on model-invented shell pipelines. Elma needs a deterministic safe backup path that can copy selected workspace files, preserve hierarchy, exclude generated directories, write a manifest, and verify counts.

## Proposed Solution

Integrate a first-class backup/copy workflow.

Likely source areas:

- `src/safe_operations.rs`
- `src/tool_calling.rs`
- `src/tools/validation.rs`
- `src/shell_preflight.rs`
- `src/tool_repair.rs`
- `elma-tools/src/tools/copy.rs`

Requirements:

- Add or expose a `backup` operation that accepts source root, destination, include globs, exclude globs, and preserve hierarchy.
- Default excludes must include `.git`, `target`, `sessions`, `project_tmp`, `_knowledge_base`, and other generated directories unless explicitly requested.
- Write a manifest with source path, destination path, byte count, and errors.
- Verify file counts and report missing/collision errors.
- Teach preflight to recommend the backup tool when shell copy loops are blocked.
- Stop continuation after shell circuit opens if no valid non-shell backup strategy is available.

## Acceptance Criteria

- [ ] Prompt 08 completes without unsafe shell loops.
- [ ] Backup directory preserves source hierarchy for selected files.
- [ ] Manifest exists and count verification passes.
- [ ] Bad copy repairs do not continue after circuit open.

## Verification Plan

Run `_testing_prompts/08_prompt.txt`.

Check:

- backup directory under `project_tmp`
- manifest file
- source count equals copied count
- no `PREFLIGHT BLOCKED` loop and no bogus `copy` repair paths

