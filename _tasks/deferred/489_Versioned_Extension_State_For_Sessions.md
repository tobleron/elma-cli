# Task 489: Versioned Extension State For Sessions

**Status:** pending
**Source patterns:** Goose extension state, Roo persisted task state, OpenHands event persistence
**Depends on:** completed Task 282 (session garbage collector), completed Task 338 (event log)

## Summary

Add versioned per-session extension state so tools, recipes, skills, MCP adapters, and UI modules can persist structured state without changing the core session schema for every feature.

## Why

As Elma adds optional tools and workflows, session state will otherwise become a collection of one-off tables or JSON blobs. Goose's extension-state pattern keeps optional feature state namespaced and migratable.

## Implementation Plan

1. Add an `extension_state` table keyed by session id, extension name, version, and JSON payload.
2. Define a trait for loading, validating, migrating, and saving extension state.
3. Expose helper APIs for optional modules.
4. Include extension-state metadata in diagnostics bundles.
5. Add migration tests.

## Success Criteria

- [ ] Optional modules can persist state without core schema edits.
- [ ] State keys are versioned and namespaced.
- [ ] Missing or unsupported extension state degrades safely.
- [ ] Session GC can clean extension state.
- [ ] Tests cover load, save, migration, and corrupt payload handling.

## Anti-Patterns To Avoid

- Do not create unversioned opaque JSON dumps.
- Do not let extensions mutate unrelated session state.
- Do not require all extensions to load during startup.
