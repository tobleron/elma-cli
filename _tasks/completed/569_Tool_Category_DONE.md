# 569 — Separate Read-Only vs Mutating Tool Categories

- **Priority**: Medium
- **Category**: Tool Calling
- **Depends on**: 552 (split tool_calling.rs)
- **Blocks**: None

## Problem Statement

The tool registry (`elma-tools/src/registry.rs`) has `ToolPolicy` with `mutates_workspace` and `risks` fields, but this metadata is not consistently used:

1. **Tool definitions sent to the model** don't distinguish read-only from mutating tools
2. **Permission gate** only applies to `shell` tool, not `edit`/`write`/`patch`/`move`/`trash`
3. **Stop policy** treats all tool failures identically regardless of risk
4. **Evidence ledger** records all tools as evidence sources but doesn't distinguish observation from mutation

## Why This Matters for Small Local LLMs

Small models benefit from clear categories:
- Knowing which tools are safe to experiment with vs which require careful use
- Understanding that read errors are safe to retry but write errors should not be blindly retried
- Having permission prompts that explain WHY a tool is being flagged

## Current Behavior

Tool definitions sent to the model (via `ToolDefinition`) only include `name`, `description`, and `parameters`. The `ToolPolicy` metadata exists in the registry but isn't exposed to the model.

## Recommended Target Behavior

1. **In tool descriptions** (sent to model): Add safety tags like `[Read-Only]`, `[Modifies Files]`, `[Requires Approval]`
2. **In permission prompts**: Show tool category and risk level
3. **In stop policy**: Distinguish read failures (retry safe) from write failures (stop)
4. **In evidence ledger**: Tag evidence entries as `Observation` or `Mutation`

## Source Files That Need Modification

- `elma-tools/src/registry.rs` — Add category metadata to `ToolDefinitionExt`
- `src/tool_registry.rs` — Expose category in built tool definitions
- `src/tool_calling.rs` — Apply permission gate based on tool policy
- `src/permission_gate.rs` — Accept tool metadata for better prompts
- `src/stop_policy.rs` — Distinguish read vs write failures

## Step-by-Step Implementation Plan

1. Add `ToolCategory` enum: `ReadOnly`, `ReadWrite`, `Destructive`, `Meta`, `Network`
2. Add `category` field to `ToolPolicy` in `elma-tools`
3. Update all tool definitions to include category
4. Add category tags to tool descriptions sent to model
5. Update permission gate to handle all mutating tool categories
6. Update stop policy to use different thresholds for read vs write failures
7. Update evidence ledger entry source with category

## Acceptance Criteria

- All tools have a `ToolCategory` in their policy metadata
- Model-facing tool descriptions include safety tags
- Permission gate applies to all `ReadWrite`/`Destructive` tools
- Stop policy has different stagnation thresholds for read vs write tools
- Evidence entries distinguish observation from mutation

## Risks and Migration Notes

- Adding permission prompts for edit/write/patch will change UX. Consider a transition period with warnings first.
- Tool description changes will affect model behavior. Run scenario tests.
