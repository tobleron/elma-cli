# Task 463: Symbol-Aware Repo Map And Tag Cache

**Status:** pending
**Source patterns:** Aider repo map, Opencode symbol tools, OpenCrabs hybrid search
**Depends on:** completed Task 343 (token capability registry)
**Related:** Task 464 (optional LSP diagnostics can enrich this map after the baseline exists)

## Summary

Build a token-budgeted repo map that indexes symbols, definitions, references, and file relationships. Cache symbol tags by file hash or mtime so repeated investigations can start from a compact map instead of broad filesystem scans.

## Why

Aider's repo map is one of the strongest reference patterns for efficient coding agents. It gives small models a compact, ranked representation of a large codebase. Elma already has a repo explorer and hybrid search, but it lacks a persistent symbol-level map that can be injected within a precise token budget.

## Implementation Plan

1. Add a `repo_map` module that indexes files with tree-sitter, LSP symbols, or a fallback parser.
2. Store cache entries under the session or project data directory with a cache version.
3. Rank symbols by recency, user-mentioned files, references to edited files, and task relevance.
4. Expose a read-only tool for compact repo-map slices.
5. Integrate the map into repo exploration without modifying `src/prompt_core.rs` unless explicitly approved.

## Success Criteria

- [ ] Repo map generation is bounded by token budget and wall-clock timeout.
- [ ] Cache invalidates when files change.
- [ ] Rust projects produce useful function/type/module entries.
- [ ] Fallback mode works without tree-sitter or LSP.
- [ ] Tests cover cache hits, cache invalidation, and token budget truncation.

## Anti-Patterns To Avoid

- Do not include huge raw file bodies in the map.
- Do not rank by hardcoded request keywords.
- Do not make symbol extraction a blocking requirement for ordinary chat.
