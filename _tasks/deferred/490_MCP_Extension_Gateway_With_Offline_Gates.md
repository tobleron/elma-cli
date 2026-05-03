# Task 490: MCP Extension Gateway With Offline Gates

**Status:** pending
**Source patterns:** Goose MCP extensions, Roo MCP support, Codex MCP integration, OpenHands microagents
**Depends on:** completed Task 339 (tool metadata policy), Task 489 (versioned extension state)

## Summary

Add an optional MCP gateway that can discover and invoke external tools through the same policy, permission, transcript, and session-state systems as native tools.

## Why

Many reference agents use MCP or extension gateways to add tools without bloating the core agent. Elma already has a dynamic tool registry concept, but not a complete permissioned external-tool gateway with offline-first controls.

## Implementation Plan

1. Add MCP server configuration with explicit enable/disable state.
2. Import MCP tool schemas into the unified tool registry with policy metadata.
3. Require per-server and per-tool permission decisions for risky capabilities.
4. Persist extension state with versioned session keys.
5. Surface MCP server start, failure, and tool calls as visible events.

## Success Criteria

- [ ] MCP support is disabled by default.
- [ ] External tools cannot bypass native permission policy.
- [ ] Tool metadata identifies network, filesystem, and destructive behavior.
- [ ] Server failures do not break core Elma tools.
- [ ] Tests cover schema import and denied external tool execution.

## Anti-Patterns To Avoid

- Do not trust external tool descriptions as safety policy.
- Do not expose MCP tools before metadata and permissions are resolved.
- Do not require network access for startup.
