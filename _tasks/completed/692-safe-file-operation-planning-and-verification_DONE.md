# Task 692: Safe File Operation Planning And Verification

## Type

Tooling

## Severity

High

## Scope

Tool-specific

## Session Evidence

Prompt 08 requested a timestamped source backup and verification:

- `sessions/s_1778085552_840248000/session.md`: Elma ran `find ... -exec cp {} ./project_tmp/backup_20260506_193943/ \;` before creating the destination directory, producing repeated `cp: ... No such file or directory` messages.
- It then created the directory and tried `csplit`, which failed.
- It eventually ran a flat `cp` into one directory and reported success.
- External verification found `project_tmp/backup_20260506_193943` contains 1,905 files while the source query matched 10,860 `.rs` files, indicating path flattening and basename collisions.

## Problem

File operations that mutate or copy large project trees need deterministic planning and verification. The current model-led shell sequence can copy in the wrong order, flatten directory structure, overwrite same-named files, and still claim success.

## Root Cause Hypothesis

Confirmed: backup execution was not planned as a safe ordered operation and verification was not performed before finalization.

Likely: Elma lacks a high-level copy/backup workflow that preserves relative paths and validates counts/checksums.

## Proposed Solution

Add a safe backup/copy workflow:

- Inspect `src/tool_calling.rs`, `elma-tools/src/tools/copy.rs`, `elma-tools/src/tools/mkdir.rs`, `src/snapshot.rs`, and `src/patch_executor.rs`.
- Provide a deterministic source-tree backup operation that preserves relative paths, creates the destination first, excludes generated/cache/vendor paths by policy, and records a manifest.
- Verify file count and optional hashes after copy.
- Block or warn when a shell `cp` command copies many files into a flat destination.
- Make finalization depend on the verification result.

## Acceptance Criteria

- [ ] Source backup preserves directory hierarchy.
- [ ] Backup verification compares source and destination file counts from the same include/exclude policy.
- [ ] A failed copy, missing destination, or basename collision prevents success finalization.

## Verification Plan

Replay prompt 08 and inspect the backup manifest, file counts, and final answer. Add an integration test with duplicate basenames in different source directories.

## Dependencies

Task 691.

