# Task 671: Offline Data Analysis Mode With Bounded Local Execution

**Status:** pending
**Priority:** MEDIUM
**Type:** Offline Feature / Tooling
**Scope:** `src/interpreter_tools.rs`, `src/tool_calling.rs`, `src/execution_profiles.rs`, `src/session_store.rs`
**Source:** deferred task 468

## Summary

Add an offline data-analysis mode that uses bounded local Python/Node execution, artifact references, and explicit resource limits.

## Evidence And Gap

- `run_python` and `run_node` tools exist, but data-analysis workflows need repeatable artifact handling, safe temp directories, and output budgets.
- Deferred Task 468 identified a mode but needs modern strict JSON/tool integration.

## Implementation Plan

1. Define data-analysis execution profile with workspace/temp paths, timeout, memory/output caps, and no network by default.
2. Store generated charts/tables/data artifacts with provenance.
3. Provide deterministic guidance for CSV/JSON/TSV/XLSX-style inputs without assuming internet libraries.
4. Add tests for timeout, large output truncation, artifact creation, and final answer citations.

## Acceptance Criteria

- [ ] Data analysis can run offline with clear resource bounds.
- [ ] Artifacts are referenced from session output and not inlined when large.
- [ ] Failed code execution is surfaced honestly.
- [ ] Network is denied unless explicitly configured.

## Verification Plan

Run local CSV/JSON analysis fixtures and inspect artifacts/session transcript.

