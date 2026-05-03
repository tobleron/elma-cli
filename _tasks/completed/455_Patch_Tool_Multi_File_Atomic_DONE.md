# Task 455: Patch Tool Multi-File Atomic Changes

**Status:** pending
**Priority:** high
**Primary surfaces:** `elma-tools/src/tools/patch.rs`, `src/tool_calling.rs`, `src/edit_engine.rs`
**Depends on:** Task 326 (shared edit robustness)
**Related tasks:** Task 456 (file context tracker), completed Task 339 (tool metadata policy), Task 441 (workspace policy files)

## Objective

Finish the existing `patch` tool so it is executable, safe, and transaction-like across multiple files. The patch tool must support add, update, and delete operations in one tool call, validate the entire plan before writing, and roll back all changed files if any operation fails.

## Current Code Reality

- `elma-tools/src/tools/patch.rs` already registers a non-deferred `patch` tool.
- `elma-tools/src/tools/patch.rs` already contains `parse_patch`, `ParsedPatch`, `PatchOperation`, and parser tests.
- `elma-tools/src/lib.rs` already re-exports the patch parser types.
- `src/tool_calling.rs::execute_tool_call` does not execute `patch`; a model calling it receives `Unknown tool: patch`.
- The current parser trims update bodies, which can corrupt leading/trailing whitespace required for exact source patches.
- Prompt core already mentions `patch`; do not edit `src/prompt_core.rs` for this task.

## Design Requirements

### Patch Format

Keep the existing custom format. Do not switch to unified diff.

```text
*** Begin Patch ***
*** Add File: src/new.rs ***
content
*** Update File: src/main.rs ***
<<<<<<< ORIGINAL
old exact text
=======
new exact text
>>>>>>> UPDATED
*** Delete File: src/old.rs ***
*** End Patch ***
```

Parser improvements required:

- preserve operation body bytes exactly except for the structural delimiters
- do not `trim()` `old_string`, `new_string`, or add-file content
- reject empty paths
- reject absolute paths unless the executor canonicalizes them inside workspace
- reject duplicate paths after normalization, not only raw string duplicates
- add parser error spans or enough context to identify the failing section

### Executor

Add a `patch` executor path in `src/tool_calling.rs` backed by a dedicated module such as `src/patch_executor.rs`.

The executor must use the shared edit/path/fingerprint behavior from Task 326 where possible. It must not duplicate stale-read and path safety logic.

### Transaction Contract

True atomicity across multiple files is not portable. Implement a best-effort transaction with explicit journaling:

1. Parse patch.
2. Normalize and validate every target path.
3. Validate read context and current fingerprints for all existing target files.
4. Validate every operation without writing.
5. Create a transaction journal under `session.artifacts_dir/patch_transactions/<tool_call_id>/`.
6. Snapshot all existing affected files into the journal.
7. Stage new file contents with temp files in each target directory.
8. Apply operations.
9. Verify final fingerprints and emit result.
10. On any failure after step 6, restore snapshots and remove newly added files.

The user-facing output must state that rollback was attempted and whether rollback succeeded.

### Operation Rules

- Add file: parent directory must exist unless the patch explicitly creates it through a supported future operation; file must not already exist.
- Update file: file must exist, must have been read, `old_string` must match exactly once unless an explicit future `replace_all` extension is added.
- Delete file: file must exist, must have been read, and must not be a directory.
- Symlink targets must be resolved and must not escape the workspace.
- Maximum operations default: 50.
- Maximum patch payload default: 256 KiB.
- Maximum per-file output default: use the edit engine limit from Task 326.

### Tool Exposure

Because completed Task 339 exists, mark `patch` as:

- write-capable
- destructive
- not concurrency-safe
- workspace-filesystem-scoped
- requires prior read for update/delete
- executor-required

This task must prevent unusable exposure by either wiring the executor before keeping `patch` non-deferred or temporarily deferring/hiding it until executable.

## Implementation Steps

1. Harden `elma-tools/src/tools/patch.rs` parser without changing the public patch format.
2. Add parser regression tests for whitespace preservation and malformed delimiters.
3. Add `src/patch_executor.rs` with validation, journal, apply, rollback, and result formatting.
4. Wire `"patch"` in `src/tool_calling.rs::execute_tool_call`.
5. Reuse `src/edit_engine.rs` for path checks, encoding, exact match counting, and safe writes.
6. Add evidence ledger integration through the existing tool-loop generic tool path.
7. Ensure the tool result includes files changed, additions, removals, and per-file status.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools patch
cargo test patch_executor
cargo test edit_engine
cargo test tool_calling
cargo build
```

Required coverage:

- parse add file
- parse update file preserving leading spaces and trailing newline
- parse delete file
- reject missing begin marker
- reject missing end marker
- reject malformed update delimiters
- reject duplicate raw paths
- reject duplicate canonical paths
- successful add + update + delete patch
- validation failure changes no files
- mid-apply failure rolls back all prior changes
- rollback failure is reported explicitly
- update old string not found fails
- update old string multiple matches fails
- update/delete without prior read fails
- stale file fails
- add existing file fails
- delete missing file fails
- symlink escape fails
- oversized patch payload fails before parsing all operations
- `execute_tool_call` no longer returns `Unknown tool: patch`

Manual probe:

```bash
rg -n '"patch" =>|Unknown tool: patch|parse_patch|patch_transactions' src elma-tools/src
```

The probe must show an executor arm, parser tests, and transaction/journal handling.

## Done Criteria

- All verification commands pass.
- `patch` is either fully executable or not exposed as callable.
- No patch operation writes before full validation succeeds.
- Rollback behavior is tested and documented in result output.
- No source prompt changes are included.

## Anti-Patterns

- Do not apply operations while still parsing or validating later operations.
- Do not leave successful transaction backups beside source files.
- Do not trim exact source text inside update hunks.
- Do not rely on model instructions for read-before-write safety.
- Do not call shell tools to apply patches.
