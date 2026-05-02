# Task 442: Tool Registry Ownership Consolidation

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 432, pending Task 405, completed Task 320, completed Task 339

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

## Implementation Plan

1. Draw the current registry data flow from startup to prompt construction to tool execution.
2. Identify duplicate structs and caches with the same responsibility.
3. Implement the approved canonical ownership boundary.
4. Update call sites to consume one source of truth for tool policy metadata.
5. Add tests that model-facing tools, workspace-discovered commands, and policy metadata stay in sync.

## Success Criteria

- [ ] There is one documented source of truth for model-callable tool schemas.
- [ ] Workspace script/package discovery has a clear owner.
- [ ] Prompt capability text cannot drift from executable tool policy.
- [ ] Duplicate registry structs are removed or explicitly scoped.
- [ ] Startup no longer performs redundant tool scans.

## Anti-Patterns To Avoid

- Do not merge all concepts into one large untyped map.
- Do not make network or MCP tools default-visible.
- Do not hide policy decisions in trace-only state.
