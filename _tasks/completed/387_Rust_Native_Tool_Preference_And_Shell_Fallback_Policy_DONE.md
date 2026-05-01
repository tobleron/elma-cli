# Task 387: Rust-Native Tool Preference And Shell Fallback Policy

**Status:** Pending
**Priority:** HIGHEST
**Estimated effort:** 2-3 days
**Dependencies:** Task 386
**References:** AGENTS.md offline-first behavior, user rust-tool priority requirement

## Objective

Make Elma prefer rust-native tools over shell commands whenever a native equivalent exists, while retaining shell as a bounded fallback for tasks that truly require it.

## Problem

Elma exposes shell and native tools, but the system does not yet have a strict capability policy that says: use the native `read`, `ls`, `glob`, `search`, `write`, `edit`, `patch`, `fetch`, or future rust tools before generating a shell command for the same operation.

Small models are more reliable when they choose from structured tools rather than synthesizing shell syntax.

## Implementation Plan

1. Extend `elma-tools` metadata with:
   - `implementation_kind`: `rust_native`, `rust_wrapper`, `shell`, `network`, `external`
   - `offline_capable`: boolean
   - `workspace_scoped`: boolean
   - `preferred_over_shell`: boolean
   - `shell_equivalents`: list of command families replaced by the tool
2. Update tool selection and planning code so native tools rank higher than shell for equivalent capabilities.
3. Add a fallback rule: shell may be used only when no safe native tool can satisfy the requested capability or when the user explicitly asks for a command.
4. Surface fallback reason as a transcript meta row.
5. Add tests that prove common operations route to native tools:
   - list directory -> `ls`
   - read file -> `read`
   - find files -> `glob`
   - search text -> `search`
   - edit file -> `edit` or `patch`

## Non-Scope

- Do not remove shell.
- Do not add keyword triggers for routing.
- Do not edit `src/prompt_core.rs`.

## Verification

```bash
cargo test -p elma-tools tool
cargo test tool_registry
cargo test routing
cargo build
```

## Done Criteria

- Tool metadata can express rust-native preference.
- Native tools are selected before shell for equivalent offline work.
- Shell fallback is explicit, bounded, and transcript-visible.
- Tests cover at least five native-over-shell cases.

