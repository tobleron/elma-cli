# Task 656: Tool Metadata Policy And Discoverable Workspace Info

**Status:** pending
**Priority:** HIGH
**Type:** Architecture / Tooling
**Scope:** `src/orchestration_core.rs`, `src/tool_registry.rs`, `elma-tools/src/tools/workspace_info.rs`, `src/tool_discovery.rs`
**Source:** deferred task 500 and superseded metadata tasks, strict JSON/tool-calling architecture

## Summary

Make workspace/environment context discoverable through tools and compact transcript rows instead of overloading the system prompt.

## Evidence And Gap

- `docs/ARCHITECTURE.md` says the system prompt includes workspace context, recent files, project guidance, tool list, and rules.
- Deferred Task 500 proposed workspace as a discoverable info tool.
- Large static prompt context harms small-model effectiveness and context efficiency.

## Implementation Plan

1. Audit system prompt content and separate mandatory policy from discoverable workspace facts.
2. Strengthen `workspace_info` into a compact, versioned, model-callable info tool.
3. Use capability metadata to decide which context is always injected vs discoverable.
4. Emit when workspace context is loaded or refreshed as transcript rows.
5. Add safeguards so small direct answers still have enough minimal context.

## Acceptance Criteria

- [ ] System prompt static workspace payload is reduced and measured.
- [ ] `workspace_info` returns concise, grounded, versioned facts.
- [ ] Tool metadata describes context, policy, and side effects.
- [ ] Regression tests verify repo-specific answers still gather evidence.

## Verification Plan

Compare prompt token counts before/after and run workspace question scenarios requiring `workspace_info`.

