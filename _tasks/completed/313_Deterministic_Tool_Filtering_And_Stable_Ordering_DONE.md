# Task 313: Deterministic Tool Filtering and Stable Ordering

## Priority
**P1 — FOUNDATION ENHANCEMENT**
**Created:** 2026-04-27
**Triggered by:** Research into top-tier agent tool exposure patterns (Codex, Roo-Code, Goose, Crush)

## Status
**COMPLETED — Phase 1 & 2 implemented. Phase 3 (prerequisite validation) deferred to future task.**

## Problem
Elma's tool registry returns all 7 core tools in `HashMap` iteration order (non-deterministic). This:
1. **Breaks prompt caching** — tool ordering changes between requests, invalidating the cache
2. **Wastes tokens on irrelevant tools** — a CHAT route still sees `shell`, `edit`, `search` schemas even though it only needs `respond`
3. **Increases hallucination surface** — more tools = more opportunities for the model to pick the wrong one

## Evidence
- `src/tool_registry.rs:333` — `default_tools()` uses `HashMap.values()` → non-deterministic order
- `src/tool_calling.rs:16` — `build_tool_definitions()` always returns all tools, no filtering
- `src/app_chat_orchestrator.rs:40` — `route_decision` is discarded with `_` prefix before reaching tool builder

## Comparison with Existing Tasks

| Task | Scope | Overlap | Relationship |
|------|-------|---------|--------------|
| **002** (Mode System) | User-facing mode switching (Architect/Code/Ask/Debug/Orchestrator) | Tool filtering | Broader — includes custom modes, `/mode` command, mode profiles. Task 313 is automatic context-aware filtering, not user-selected modes. |
| **003** (Mode Manager) | Mode switching logic and persistence | Tool filtering | Depends on 002. Task 313 does not require mode switching — it uses existing route classification. |
| **264** (Dynamic Tool Registry) | Searchable capabilities + deferred loading | Registry structure | Already implemented. Task 313 adds filtering/ordering on top of the existing registry. |
| **291** (Core Tools Always Available) | Remove discovery prerequisite for core tools | Tool availability | Already implemented. Task 313 does not hide core tools from routes that need them. |

**Conclusion:** No existing task covers automatic deterministic tool filtering based on route classification. Task 313 is the surgical prerequisite that makes the broader mode system (002/003) easier to implement later.

## Root Causes
1. **No stable ordering** — `HashMap` iteration order is randomized by `RandomState`
2. **Route context discarded** — `route_decision` is computed upstream but thrown away before tool building
3. **No filtering logic** — all tools are always visible regardless of task type

## Design

### Phase 1: Stable Ordering (implemented)
Sort tools by name before returning. This:
- Enables prompt caching (same tools in same order = cache hit)
- Makes debugging reproducible
- Costs ~0 tokens (same tools, just ordered)

### Phase 2: Context-Aware Filtering (implemented)
Map route strings to relevant tool subsets:

| Route | Visible Tools | Rationale |
|-------|--------------|-----------|
| CHAT | respond, read_evidence | Conversational reply only; read_evidence for referencing past evidence |
| SHELL | shell, read, search, respond, update_todo_list, read_evidence, tool_search | Full workspace interaction |
| PLAN | read, search, respond, update_todo_list, read_evidence, tool_search | Read-only investigation |
| DECIDE | read, search, respond, update_todo_list, read_evidence | Evidence gathering for decisions |
| WORKFLOW | all tools | Complex multi-step work |
| unknown | all tools | Safe fallback |

### Phase 3: Prerequisite Validation (future)
Before exposing a tool, validate it can work:
- `shell`: shell session is alive
- `search`: `rg` binary is available
- `edit`: workspace is writable
- `read_evidence`: evidence ledger has entries

This matches the Hermes `check_fn` pattern.

## Implementation

### Files Changed
- `src/tool_registry.rs` — add sorting + `build_tools_for_context()`
- `src/tool_calling.rs` — add context-aware builder
- `src/tool_loop.rs` — accept context hint, pass to builder
- `src/orchestration_core.rs` — accept context hint, pass to tool loop
- `src/app_chat_orchestrator.rs` — pass `route_decision.route` as context

### Key Design Decisions
1. **Conservative filtering**: Unknown routes get all tools (safe fallback)
2. **CHAT gets read_evidence**: The model may need to reference previous session evidence even in chat mode
3. **No signature breakage**: Existing `build_tool_definitions()` kept for compatibility
4. **Deterministic sorting**: Alphabetic by tool name for cache stability

## Verification
- [x] `cargo test` passes (including `test_prompt_unchanged`)
- [x] `cargo check --all-targets` passes
- [x] `cargo test tool_registry::tests` passes (15/15)
- [x] `cargo test --bin elma-cli` passes (658/659; 1 pre-existing `env_utils` failure unrelated to this change)

## Experiment Log
- [x] Implemented stable ordering in `default_tools()`, `build_current_tools()`, `get_tools()`
- [x] Implemented `build_tools_for_context()` with route-based filtering
- [x] Threaded `context_hint` through `app_chat_orchestrator.rs` → `orchestration_core.rs` → `tool_loop.rs` → `tool_calling.rs`
- [x] Added tests: `test_build_tools_for_context_chat`, `test_build_tools_for_context_shell`, `test_build_tools_for_context_unknown`, `test_stable_ordering`
- [x] Fixed brittle test `test_get_tool_names` that broke due to `tool_search` description mentioning "read"

## Results
| Metric | Before | After |
|--------|--------|-------|
| Tool ordering | Non-deterministic (HashMap) | Alphabetic by name |
| CHAT route tool count | 7 | 2 (respond, read_evidence) |
| SHELL route tool count | 7 | 7 (all tools) |
| Unknown route tool count | 7 | 7 (safe fallback) |
| Prompt cache stability | Low (order changes) | High (stable order) |

## Files Changed
- `src/tool_registry.rs` — sorting + `build_tools_for_context()` + tests
- `src/tool_calling.rs` — `build_tool_definitions_for_context()`
- `src/tool_loop.rs` — accept `context_hint` parameter
- `src/orchestration_core.rs` — accept `context_hint`, pass to tool loop
- `src/app_chat_orchestrator.rs` — pass `route_decision.route` as context
- `docs/SYSTEM_PROMPT_EVOLUTION.md` — document v3 prompt change (rg preference)
