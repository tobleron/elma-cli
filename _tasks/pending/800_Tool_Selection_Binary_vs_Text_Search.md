# Task 800: Tool Selection for Binary vs Text Search

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Teach the model to use file-oriented tools (`glob`, `ls`, `find`, `stat`) for locating binary/executable files, not text search tools (`rg`, `grep`). In the session, `rg pattern=elma-cli` matched Cargo.toml and source code but couldn't find the binary itself.

## Root Cause
The model conflates file-existence search with text-content search. `rg` searches file contents and returns 0 matches for binary files — it is the wrong tool for finding executables.

## Requirements
- Add prompt instruction distinguishing file-path tools from content-search tools:
  - **Find files by name/path**: use `glob`, `ls`, `stat`, `find`
  - **Search file contents**: use `rg`, `grep`, `search`
  - **Find executables specifically**: use `ls -la target/release/` and look for files with execute permission, or `find target/release -type f -perm +111`
- When searching for a compiled binary, prefer `glob` or `ls` over `rg`.
- After a text search returns 0 results for a binary name, inject a hint: "Text search found no matches. Use glob or ls to find the binary file."

## Failure Mode Fixed
- Using content search for file existence (wrong tool)
- Wasted iterations on rg searches for binary paths
