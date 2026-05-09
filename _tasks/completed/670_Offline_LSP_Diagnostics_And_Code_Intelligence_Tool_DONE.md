# Task 670: Offline LSP Diagnostics And Code Intelligence Tool

**Status:** pending
**Priority:** MEDIUM
**Type:** Offline Feature / Tooling
**Scope:** `src/tools/`, `elma-tools/src/`, `src/repo_map.rs`, `src/tool_registry.rs`
**Source:** deferred task 464, postponed task 027

## Summary

Add an optional local LSP diagnostics/code-intelligence tool with strict workspace boundaries and deterministic fallback when language servers are absent.

## Evidence And Gap

- Repo map exists, but LSP diagnostics can provide compiler/language-specific evidence without network access.
- Deferred Task 464 specified fake JSON-RPC LSP fixtures and no requirement for rust-analyzer in tests.

## Implementation Plan

1. Add LSP process discovery and JSON-RPC client helpers.
2. Support diagnostics, symbol lookup, definition/reference requests, and document sync for files already in the workspace.
3. Keep the tool optional and report missing language servers as structured non-fatal errors.
4. Add fake LSP test server fixtures.

## Acceptance Criteria

- [ ] Tool never accesses files outside workspace policy.
- [ ] Missing LSP server produces useful local fallback guidance.
- [ ] JSON-RPC framing and malformed response errors are tested.
- [ ] Diagnostics can be attached as evidence.

## Verification Plan

Run fake LSP tests and, when available, a manual Rust diagnostics query.

