# Task 799: Directory Preparation Before File Operations

## Status
- **Priority:** Medium
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Before copying or moving files to a destination, ensure the parent directory exists. In the session, `copy target/debug/elma-cli ~/Desktop/workspace/` failed because `~/Desktop/workspace` didn't exist.

## Root Cause
The model performs copy/move operations without verifying the target directory exists first.

## Requirements
- Add a pre-operation check: before any copy/write to a new path, run `mkdir -p <parent_dir>`.
- Alternatively, wrap the copy tool to auto-create parent directories.
- Add prompt instruction: "When copying files to a new location, create the parent directory first with `mkdir -p`."

## Failure Mode Fixed
- File copy failures due to missing parent directories
- Wasted iterations retrying failed copies
