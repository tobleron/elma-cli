# Task 457: Rust-First File Operation Tool Completeness

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-4 days
**Dependencies:** Task 387, Task 456, Task 441
**References:** source-agent parity: common file-management tools

## Objective

Complete rust-native file operation coverage so Elma does not use shell for common local filesystem actions.

## Tool Families

Add or verify structured tools for:

- stat/metadata
- copy
- move/rename
- mkdir
- safe delete/trash
- touch/create empty file
- file size/count helpers
- path existence checks

## Implementation Plan

1. Use Task 386 matrix to identify missing file-operation equivalents.
2. Add tool declarations with small schemas and executor parity.
3. Enforce stale-read, workspace policy, symlink, and protected-path gates.
4. Route shell fallbacks through Task 387 only when no native operation exists.
5. Add tests for safe success and safe failure.

## Verification

```bash
cargo test file_ops
cargo test tool_calling
cargo test workspace_policy
cargo build
```

## Done Criteria

- Common file operations are available as rust-native tools.
- Mutations respect policy and stale-read checks.
- Shell is not used for basic file operations unless explicitly requested.

