# Task 381: Transcript-Native Operational Visibility

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Dependencies:** None
**References:** AGENTS.md Rule 6, objectives.md "How We Get There" #3, _masterplan.md Task 304

## Problem

AGENTS.md Rule 6 states: "Budgeting, routing/formula choice, compaction, stop reasons, and hidden processes must surface as collapsible transcript rows. Do not bury these in trace-only state, debug logs, or hidden metadata."

Current behavior hides compaction triggers, routing decisions, and stop reasons behind trace-only logging (`trace!` macro). The user cannot see why Elma made certain decisions. The `push_stop_notice` method on `TerminalUI` has 0 callers.

## Objective

Surface all operational events as visible, collapsible transcript rows:
1. **Routing decisions**: When route is selected and why (entropy, margin, source)
2. **Formula selection**: Which formula was chosen and the reason
3. **Stop reasons**: Why the tool loop stopped (stagnation, budget exhausted, timeout)
4. **Compaction events**: When and why context is compacted
5. **Provider events**: Model selection, provider switching, retry attempts
6. **Decomposition events**: When task is split due to model weakness (Task 379)
7. **Tool discovery events**: Which capability was searched, which tools were loaded, and why shell fallback was used

## Implementation Plan

### Phase 1: Define Transcript Meta-Event API

Ensure `TerminalUI` has the necessary methods:

```rust
impl TerminalUI {
    /// Push a collapsible operational event into the transcript
    pub fn push_meta_event(&mut self, category: &str, detail: &str) { ... }

    /// Push a stop reason notice
    pub fn push_stop_notice(&mut self, reason: &str) { ... }
}
```

### Phase 2: Wire Events Into Execution Path

| Event | Location | Category |
|-------|----------|----------|
| Route selected | `app_chat_loop.rs` after routing decision | `ROUTE` |
| Formula selected | `app_chat_orchestrator.rs` after formula selection | `FORMULA` |
| Stop reason | `tool_loop.rs` when stop policy triggers | `STOP` |
| Compaction | Compaction module | `COMPACT` |
| Provider switch | `llm_provider.rs` on switch | `PROVIDER` |
| Retry attempt | `orchestration_retry.rs` per attempt | `RETRY` |
| Decomposition | `orchestration_retry.rs` (Task 379) | `DECOMPOSE` |
| Tool discovery | `tool_registry.rs` / orchestration discovery path | `TOOLS` |

### Phase 3: Collapsible Rendering

Meta-events should render as a single line that expands on selection:
```
[ROUTE] CHAT (entropy=0.12, source=speech_act) ▸
    margin=0.88 speech=CHAT workflow=CHAT mode=DECIDE

[STOP] Stagnation detected after 3 identical responses ▸
    respond ids: 4Gkeve18, tT0n4O9q, g4FFL8Vh
```

### Phase 4: Remove Dead Code

`push_stop_notice` on `TerminalUI` currently has 0 callers. Either wire it in or remove it.

## Files to Modify

| File | Change |
|------|--------|
| `src/ui_terminal.rs` | Ensure `push_meta_event` and `push_stop_notice` exist and work |
| `src/app_chat_loop.rs` | Call `push_meta_event("ROUTE", ...)` after routing decision |
| `src/orchestration_retry.rs` | Call `push_meta_event("RETRY", ...)` and `push_meta_event("DECOMPOSE", ...)` |
| `src/tool_loop.rs` | Call `push_stop_notice(...)` when stop policy triggers |
| `src/llm_provider.rs` | Call `push_meta_event("PROVIDER", ...)` on switch |

## Verification

```bash
cargo build
cargo test transcript
cargo test ui
```

**Manual**: Run any multi-step query. Verify the transcript shows collapsible rows for:
- Route selection (how the system decided to route)
- Stop reason (if any)
- Retry attempts (if any)
