# Task 386: Source-Agent Tool Parity Gap Matrix

**Status:** Pending
**Priority:** HIGHEST
**Estimated effort:** 1-2 days
**Dependencies:** None
**References:** AGENTS.md offline-first philosophy, user objective for `_knowledge_base/_source_code_agents`

## Objective

Build the canonical parity matrix between tools implemented by source-code agents under `_knowledge_base/_source_code_agents/` and tools available in `elma-cli`.

The output must identify every tool class that another local agent supports, whether Elma already has an equivalent, whether the equivalent is rust-native or shell-backed, and which pending task closes any gap.

## Scope

- Inspect only local source under `_knowledge_base/_source_code_agents/`.
- Cover at least: Claude Code, Codex CLI, Crush, Goose, Roo-Code, OpenHands, Aider, Hermes Agent, AgenticSeek, LocalAGI, and Open Interpreter.
- Include tool families, not only exact names: read/view, write, edit, multi-edit, patch, grep/search, glob, ls, shell, background jobs, fetch, browser, MCP, diagnostics/LSP, memory, todo, code interpreter, git, download, references, recipes, subagents, and slash commands.
- Prefer offline/rust-native equivalents before shell or network-backed equivalents.

## Implementation Plan

1. Create `_tasks/artifacts/source_agent_tool_parity.md`.
2. Inventory source-agent tool names, paths, and implementation notes.
3. Inventory Elma tool declarations from `elma-tools/src/tools/` and executor support from `src/tool_calling.rs` and execution modules.
4. Classify each row as `DONE`, `PENDING`, `MISSING`, `DEFERRED_NETWORK`, or `NOT_APPLICABLE`.
5. Link every `PENDING` or `MISSING` row to a specific pending task number.
6. Mark the preferred implementation mode: `rust_native`, `rust_wrapper`, `shell_fallback`, `network_optional`, or `external_extension`.

## Verification

```bash
rg -n "register\\(|ToolDefinition|execute_tool_call|Unknown tool" elma-tools/src src
rg -n "tools|Tool|tool|browser|mcp|grep|glob|edit|patch|job|memory|download" _knowledge_base/_source_code_agents -g '*.md' -g '*.rs' -g '*.go' -g '*.py' -g '*.ts' -g '*.tsx'
```

## Done Criteria

- The parity matrix exists and is complete enough to drive implementation.
- Every missing equivalent has a pending task or a documented non-goal reason.
- Rust-native/offline priority is explicit in the matrix.
- No source behavior changes are made in this task.

