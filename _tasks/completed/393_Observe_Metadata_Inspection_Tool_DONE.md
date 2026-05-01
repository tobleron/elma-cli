# Task 393: Observe Metadata Inspection Tool

**Status:** Pending
**Priority:** HIGH
**Estimated effort:** 1-2 days
**Dependencies:** Task 387, Task 395, Task 396
**References:** source-agent parity: file stat/view/discovery tools; objectives.md context efficiency

## Objective

Add a rust-native `observe` tool/step for metadata-only inspection so Elma can discover workspace facts without consuming full file contents.

## Problem

Elma can read files, search files, and list directories, but it lacks a first-class metadata-only operation. Small models waste context when they read full content just to learn whether a path exists, file size, modified time, type, or directory shape.

## Scope

`observe` should support:

- path existence
- file type
- size
- modified time
- directory child count
- symlink target status without escaping workspace
- MIME/type hint where cheap

## Implementation Plan

1. Add an `observe` tool declaration in `elma-tools`.
2. Add a `Step::Observe` or direct tool executor path, following existing architecture.
3. Implement metadata collection in Rust using `std::fs` and existing path policy helpers.
4. Integrate stale-read and workspace policy once Tasks 395 and 396 are available.
5. Prefer `observe` before `read` when planning needs metadata only.
6. Emit concise transcript/evidence rows without putting full file content into context.

## Non-Scope

- Do not use shell `stat`, `ls`, or `find` for the core implementation.
- Do not read file contents except for tiny type sniffing if already allowed by policy.
- Do not add keyword routing.

## Verification

```bash
cargo test observe
cargo test tool_calling
cargo test workspace_policy
cargo build
```

## Done Criteria

- Metadata inspection works through a rust-native tool.
- Planning can use metadata without full reads.
- Symlink and workspace boundaries are enforced.
- Results are small-model-friendly and transcript-visible.

