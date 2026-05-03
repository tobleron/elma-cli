# Task 339: Action And Tool Metadata Policy Unification

**Status:** active — added `ActionRisk`, `ActionPolicy`, `ALL_ACTION_POLICIES` static array, `concurrency_safe_for_tool()` in `src/action_policy.rs`; extended `ToolDefinitionExt` in `elma-tools/src/registry.rs` with `ToolRisk`, `ToolExecutorState`, `requires_permission`, `requires_prior_read`, `concurrency_safe` fields; updated all 13 tool modules with policy metadata; replaced hardcoded `is_concurrency_safe` in `streaming_tool_executor.rs` with metadata-backed lookup via `concurrency_safe_for_tool()`.

**Source patterns:** AgentAction policy mapping, tool registry metadata, concurrency scheduling in streaming executor

**Depends on:** none

## Summary

Add explicit policy metadata for every model-callable DSL action and remaining adapter. Ensure action/tool exposure matches executor support and scheduling/permission behavior.

## Implementation Plan

1. Define `ActionRisk` enum (ReadOnly, WorkspaceWrite, ExternalProcess, Network, ConversationState) and `ActionPolicy` struct in new `src/action_policy.rs`.
2. Create `ALL_ACTION_POLICIES` static array covering all 8 `AgentAction` variants (R, L, S, Y, E, X, ASK, DONE).
3. Add lookup functions `action_policy(command)` and `action_policy_for_variant(variant)`.
4. Add `ToolRisk`, `ToolExecutorState` enums and policy fields to `ToolDefinitionExt` in `elma-tools/src/registry.rs`.
5. Add builder methods (`.with_risks()`, `.with_executor_state()`, `.requires_permission()`, `.requires_prior_read()`, `.concurrency_safe()`).
6. Update all 13 tool modules in `elma-tools/src/tools/*.rs` with accurate policy metadata.
7. Add `concurrency_safe_for_tool(tool_name)` in action_policy.rs that covers all known tool names.
8. Replace hardcoded `matches!(tool_name, "read" | "search" | "respond")` in `streaming_tool_executor.rs` with call to `concurrency_safe_for_tool()`.

## Verification

- `cargo check` — clean
- `cargo test -- --test-threads=1` — **790 passed** (up from 785, includes 5 new action_policy tests)
- All 5 action_policy unit tests pass (all_actions_have_policy, read_actions_concurrency_safe, write_actions_serial, variant_lookup, concurrency_safe_tools)
