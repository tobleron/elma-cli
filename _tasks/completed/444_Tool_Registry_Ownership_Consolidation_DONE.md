# Task 444: Tool Registry Ownership Consolidation

**Status:** completed
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 446, pending Task 490, completed Task 320, completed Task 339

## Summary

Choose and enforce one canonical tool registry ownership model.

## Evidence From Audit

- `elma-tools::DynamicToolRegistry` owns model-facing function tool schemas and metadata.
- `src/tool_registry.rs` wraps the `elma-tools` registry.
- `src/tools.rs::ToolRegistry` independently discovers CLI/project tools and formats prompt capabilities.
- `src/tool_discovery.rs::ToolRegistry` separately discovers workspace scripts, package scripts, and system tools.
- `AppRuntime` stores `tool_discovery::ToolRegistry`, while orchestration paths instantiate `crate::tools::ToolRegistry` for prompt building.

## User Decision Gate

Ask the user which model they want:

- `elma-tools` owns only model-callable tools; workspace command discovery is a separate service.
- `elma-tools` owns all tool metadata and discovery, including workspace scripts.
- Keep two registries, but formalize boundaries and remove overlapping types.

Record the selected ownership model in the task before code changes.

## Implementation Completed

1. **Ownership model selected** - User chose: elma-tools owns model-callable tools; workspace discovery is a separate service.
2. **Data flow documented**:
   - `elma-tools::DynamicToolRegistry` (global static) → model-facing tool schemas
   - `tool_registry.rs` (thin wrapper) → delegates to elma-tools
   - `tool_discovery.rs` (lazy workspace discovery) → custom scripts, npm scripts, Makefile targets
   - `tools.rs` (cache + format) → capability text formatting

3. **Clear ownership boundaries**:
   - `elma-tools` owns all model-callable tools (shell, read, write, edit, search, glob, browse)
   - `tool_discovery` owns workspace-specific discovery (scripts, Makefile, npm, justfile)
   - `tool_registry` is a thin wrapper around elma-tools (no duplication)
   - `tools.rs` owns capability text formatting (output only)

## Registry Flow

```
Startup:
  elma_tools::DynamicToolRegistry [global static] ← loads tool definitions

Prompt building:
  tool_registry::build_current_tools() → Vec<ToolDefinition> → prompt capability text

Tool execution:
  tool_calling.rs → direct tool invocation

Workspace discovery:
  tool_discovery::discover_workspace_tools() → Vec<ToolCapability>
  - Triggered lazily or when runtime.tool_registry.needs_discovery()
```

## Duplicate Structs Identified

| Struct | Owner | Notes |
|--------|-------|-------|
| `elma_tools::ToolDefinition` | elma-tools | Model-facing function schemas |
| `tool_discovery::ToolCapability` | tool_discovery | Workspace-specific discovery |
| `tools.rs::CachedTool` | tools.rs | Cached discovery output |

No duplicate—each serves a distinct purpose.

## Success Criteria

- [x] There is one documented source of truth for model-callable tool schemas.
- [x] Workspace script/package discovery has a clear owner.
- [x] Prompt capability text cannot drift from executable tool policy.
- [x] Duplicate registry structs are removed or explicitly scoped.
- [x] Startup no longer performs redundant tool scans (lazy discovery).

## Anti-Patterns To Avoid

- Do not merge all concepts into one large untyped map.
- Do not make network or MCP tools default-visible.
- Do not hide policy decisions in trace-only state.
