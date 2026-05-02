# Task 450: Startup Performance And Repeated Scan Reduction

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 397, pending Task 431, completed Task 285

## Summary

Reduce startup and per-turn overhead from repeated scans, registry construction, workspace summaries, and eager probes while preserving reliability.

## Evidence From Audit

- `orchestration_core.rs` and `orchestration_retry.rs` instantiate `crate::tools::ToolRegistry::new(&workspace_path)` in multiple planning/retry paths.
- `ToolRegistry::new` calls `discover_available_tools`, which can scan PATH/common directories and project files.
- `bootstrap_app` gathers full workspace context and brief at startup.
- First-time startup can perform connectivity checks, model behavior probing, profile loading/sync, context discovery, and optional tuning.
- Large modules such as `tool_loop.rs`, `document_adapter.rs`, and UI renderers concentrate hot-path logic.

## User Decision Gate

Ask the user which performance tradeoff is acceptable:

- Cache aggressively and invalidate by manifest/path mtime.
- Keep startup slow but deterministic.
- Defer nonessential discovery until the model asks for it.

Reliability remains higher priority than raw speed.

## Implementation Plan

1. Instrument startup phases with transcript-visible timing rows.
2. Cache tool discovery and workspace brief in a scoped, invalidatable structure.
3. Reuse runtime registries instead of rebuilding them in retry paths.
4. Defer optional scans until capability discovery needs them.
5. Add a benchmark or snapshot timing harness for cold/warm startup.

## Success Criteria

- [ ] Repeated tool registry scans are eliminated or justified.
- [ ] Cold and warm startup timings are visible.
- [ ] Cached workspace/tool data invalidates correctly.
- [ ] No accuracy regression in tool awareness or workspace grounding.

## Anti-Patterns To Avoid

- Do not remove reasoning or evidence stages merely to improve speed.
- Do not use stale cached workspace facts without visible invalidation.
- Do not hide performance decisions in trace-only logs.
