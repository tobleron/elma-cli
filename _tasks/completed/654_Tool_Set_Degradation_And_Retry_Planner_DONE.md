# Task 654: Tool Set Degradation And Retry Planner

**Status:** pending
**Priority:** HIGH
**Type:** Reliability / Tooling
**Scope:** `src/tool_loop.rs`, `src/tool_registry.rs`, `src/tool_discovery.rs`, `src/stop_policy.rs`
**Source:** deferred task 504, user priority on best error handling

## Summary

When a tool repeatedly fails or is unsupported by the current model/provider, degrade the tool set or retry strategy deterministically instead of repeating failures.

## Evidence And Gap

- `stop_policy.rs` detects repeated failures, but fallback behavior is not a structured retry plan tied to tool families.
- Tool discovery and tool execution are separate from model capability and failure classification.
- Dense local models often need fewer tools and clearer schemas after repeated tool-call mistakes.

## Implementation Plan

1. Track failure rates by tool name, tool family, argument schema, and provider/model.
2. Add retry plans: repair args, load narrower tool set, switch to read-only mode, ask clarification, or finalization with honest incomplete status.
3. Make degradation visible in transcript and session diagnostics.
4. Avoid hiding tools globally across sessions unless a model capability probe confirms incompatibility.

## Acceptance Criteria

- [ ] Repeated tool failures trigger a deterministic alternative plan.
- [ ] The same invalid call is not retried indefinitely.
- [ ] Tool degradation is scoped to the current turn/session/model.
- [ ] Tests cover failure dedup, degradation, and recovery after a later success.

## Verification Plan

Run synthetic tool-loop fixtures with invalid args and unavailable tools.

