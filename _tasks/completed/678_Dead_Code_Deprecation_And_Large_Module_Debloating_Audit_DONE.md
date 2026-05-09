# Task 678: Dead Code Deprecation And Large Module Debloating Audit

**Status:** pending
**Priority:** MEDIUM
**Type:** Refactor / Maintenance
**Scope:** `src/`, `elma-tools/src/`, docs/module exports
**Source:** deferred task 484, postponed task 064, docs de-bloating guidance

## Summary

Identify dead modules, stale compatibility paths, duplicate systems, and oversized modules that should be split for maintainability.

## Evidence And Gap

- Development guidelines list high-risk large files such as `intel_units`, `json_error_handler`, `program_policy`, `defaults_evidence`, and `types_core`.
- UI and tool execution still have large central files.
- Some completed/deferred tasks refer to old paths and duplicate task numbers.

## Implementation Plan

1. Run compile- and grep-based reachability analysis for modules and public exports.
2. Classify each candidate as active, test-only, compatibility, deprecated, or dead.
3. Split large modules only when it reduces real coupling or matches existing module patterns.
4. Update docs/tasks if file ownership changes.

## Acceptance Criteria

- [ ] Dead code decisions are evidence-backed.
- [ ] No active behavior is removed without tests.
- [ ] Large module split proposals identify exact seams and verification.
- [ ] Completed/pending duplicate task references are reconciled.

## Verification Plan

Run `cargo check --all-targets`, analyzer, and targeted tests for moved modules.

