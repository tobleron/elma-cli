# Task 803: Path Resolution — Absolute Paths for Read Operations

## Status
- **Priority:** Medium
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Fix path resolution failures when reading files discovered via directory listing. In the session, `read target/release/full.rmeta` failed with `file_not_found` even though `ls target/release` showed the file. The relative path resolved incorrectly.

## Root Cause
The read tool resolved the relative path against a different working directory than the ls tool used. The model assumes paths are always relative to the workspace root, but some tools may have different cwd contexts.

## Requirements
- After listing a directory, construct the full absolute path when reading files from that directory.
- Add a tool-level path canonicalization: before reading, resolve the path against the workspace root and check existence with `stat`.
- Add prompt instruction: "When reading files discovered via `ls`, use the full absolute path (e.g., `/Users/.../target/release/elma-cli`), not a relative path."
- Consider adding a `test -f` pre-check before read operations on paths that might be directories or symlinks.

## Failure Mode Fixed
- File not found errors for files that actually exist
- Relative path resolution inconsistency between tools
