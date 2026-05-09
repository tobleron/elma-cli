# Task 645: Unified Strict Tool Argument Parsing And Model Facing Error Contract

**Status:** pending
**Priority:** CRITICAL
**Type:** Tooling / Model Robustness
**Scope:** `src/tool_calling.rs`, `src/json_parser.rs`, `src/tools/validation.rs`, `elma-tools/src/`
**Source:** old unified tool trait task, `_knowledge_base` Roo tool validation tests, Codex tool registry

## Summary

Route every tool argument through one strict parse, repair, validation, and model-facing error contract.

## Evidence And Gap

- `tool_calling.rs` directly parses `tool_call.function.arguments`, then calls `parse_model_json`, then applies ad hoc repair for `read` and `exists`.
- `src/tools/validation.rs` has schemas, while `elma-tools/src/registry.rs` also defines tool parameters.
- Local small/dense models need deterministic, concise correction messages instead of scattered special cases.

## Implementation Plan

1. Add a single `parse_tool_arguments(tool_name, raw)` API that returns typed `ToolArgs` or `ToolArgError`.
2. Generate/derive validation schemas from the canonical registry so declarations and executors cannot drift.
3. Replace ad hoc path extraction with deterministic repair rules scoped by schema.
4. Return compact structured errors that include required fields, invalid fields, and one copyable strict JSON example.
5. Add unknown-field policy tests per tool.

## Acceptance Criteria

- [ ] No tool executor parses raw model JSON independently.
- [ ] Registry schema and validator schema parity is tested.
- [ ] Missing/invalid args never execute a tool.
- [ ] Tool error corrections remain compact enough for small models.

## Verification Plan

Run `cargo test tool_arg validation tool_registry` and malformed argument fixtures for every model-callable tool.

