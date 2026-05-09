# Task 711: Backup Workflow Convergence And Safe Backup Tool Exposure

## Type

Tooling / Mutating Workflow / Autonomy

## Severity

High

## Evidence

Round 5 prompt 08 did not complete within the 420 second bound:

- Initial session: `sessions/s_1778120131_951838000`
- Retry session after minor shell-circuit fix: `sessions/s_1778120772_86517000`

The retry made partial progress:

- Detected model and context correctly.
- Created `project_tmp/backup_20260507_052915`.
- Copied `src/`, `config/`, and `docs/` with the `copy` tool.

But it still failed to converge:

- Repeated blocked shell commands like `cp -r src/* ...`
- Shell preflight parsed `cp -r` incorrectly as source `-r`.
- The safe backup implementation exists in `src/safe_operations.rs`, but it is not exposed as a first-class tool the model can select.
- The session timed out before verification/final answer.

## Problem

Backup is a common enterprise CLI workflow and should not rely on the model inventing shell copy commands. Elma already has Rust-native safe backup logic; the runtime needs to expose and prefer it.

## Requirements

- Expose a `backup` or `safe_backup` tool backed by `safe_operations::run_backup_operation`.
- Tool input should accept source directory, destination directory, include patterns, exclude patterns, and verification flag.
- The tool must write a manifest and return source count, copied count, skipped count, errors, manifest path, and verification status.
- Teach shell preflight to parse common flags before source operands, so `cp -r src dest` does not report source `-r`.
- When backup intent is detected through the mutating contract, prefer the safe backup tool over shell `cp`.
- Stop after a successful backup plus verification instead of continuing budget loops.

## Acceptance Criteria

- [ ] `_testing_prompts/08_prompt.txt` completes within the normal prompt timeout.
- [ ] The backup directory contains expected source files and a manifest.
- [ ] Trace shows the safe backup tool result and verification counts.
- [ ] No repeated blocked `cp -r` shell attempts occur after the safe backup tool is available.

