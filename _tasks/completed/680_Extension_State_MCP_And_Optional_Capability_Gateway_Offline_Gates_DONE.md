# Task 680: Extension State MCP And Optional Capability Gateway Offline Gates

**Status:** pending
**Priority:** LOW
**Type:** Architecture / Optional Integration
**Scope:** `src/session_store.rs`, `src/tool_registry.rs`, `src/tool_discovery.rs`, future extension modules
**Source:** deferred tasks 270, 489, 490; postponed task 009

## Summary

Design optional MCP/extension capability support with versioned session state and offline-first gates, without making internet/network behavior part of the core path.

## Evidence And Gap

- MCP and extension tasks are valuable but lower priority than offline core reliability.
- Session state for optional features should be namespaced/versioned to avoid opaque JSON sprawl.

## Implementation Plan

1. Define extension state schema and migrations before adding runtime integrations.
2. Require explicit enablement for MCP/networked extensions.
3. Keep dynamic capabilities visible in transcript and session diagnostics.
4. Prevent optional tools from polluting core prompt/tool context unless selected.

## Acceptance Criteria

- [ ] Optional capability state is versioned and migratable.
- [ ] MCP/network extensions are disabled by default.
- [ ] Core offline behavior is unaffected when extensions are unavailable.
- [ ] Tool discovery shows capability source and risk.

## Verification Plan

Use fake local MCP/extension fixtures only; do not require internet.

