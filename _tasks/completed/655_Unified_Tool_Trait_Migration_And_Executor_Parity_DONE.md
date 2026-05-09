# Task 655: Unified Tool Trait Migration And Executor Parity

**Status:** pending
**Priority:** CRITICAL
**Type:** Architecture / Tooling
**Scope:** `elma-tools/src/`, `src/tool_registry.rs`, `src/tool_calling.rs`, `src/tools/`
**Source:** old pending `full_migration_to_unified_tool_trait.md`, old pending 005, `_knowledge_base` Codex tool registry

## Summary

Complete the migration to a unified tool trait so declarations, validation, execution, mutation metadata, policy, and UI/event payloads cannot drift.

## Evidence And Gap

- `src/tool_registry.rs` wraps `elma-tools`, but `src/tool_calling.rs` still has a large string-match executor dispatch.
- Existing tests check some registry/executor parity, but tool metadata remains split.
- `_knowledge_base/_source_code_agents/codex-cli/codex-rs/core/src/tools/registry.rs` uses typed handlers with mutation and payload hooks.

## Implementation Plan

1. Define a `ToolHandler` trait in `elma-tools` or a shared crate boundary with schema, metadata, validation, policy, and execute methods.
2. Move per-tool executor arms out of `tool_calling.rs` into tool modules.
3. Require every tool to declare read/write/network/process/session effects.
4. Add compile/test gates that every registered tool has an executor and every executor has a declaration.
5. Preserve existing model-facing tool names and strict JSON schemas.

## Acceptance Criteria

- [ ] No giant string-match dispatch remains for model-callable tools.
- [ ] Tool metadata and executor support are source-of-truth consistent.
- [ ] Mutating tools consistently request snapshots/policy checks.
- [ ] Tool parity tests fail on declaration/executor drift.

## Verification Plan

Run `cargo test tool_registry tool_calling elma_tools` and tool self-test scenarios.

