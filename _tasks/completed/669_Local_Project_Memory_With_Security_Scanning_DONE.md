# Task 669: Local Project Memory With Security Scanning

**Status:** pending
**Priority:** MEDIUM
**Type:** Offline Feature / Security
**Scope:** `src/hybrid_search.rs`, `src/effective_history.rs`, `src/project_guidance.rs`, `src/session_store.rs`
**Source:** postponed tasks 023/078, `_knowledge_base` memory and trace summarization patterns

## Summary

Add persistent local project memory that stores useful facts with provenance, expiry, and security scanning instead of unbounded conversation memory.

## Evidence And Gap

- Elma has session summaries and project guidance, but long-term tactical memory remains postponed.
- Memory should improve context efficiency without leaking secrets or persisting hallucinations.
- AGENTS.md requires truth-grounded answers and local-first behavior.

## Implementation Plan

1. Store memory entries with source evidence, confidence, timestamps, workspace id, and expiry/revalidation policy.
2. Scan candidate memories for secrets, credentials, and sensitive paths before writing.
3. Require evidence-backed extraction from sessions or files; do not store unsupported model claims.
4. Retrieve memory only when relevant and cite source artifact/path.

## Acceptance Criteria

- [ ] Memory entries are evidence-backed and workspace-scoped.
- [ ] Secret-like content is rejected or redacted.
- [ ] Stale memory is revalidated before use.
- [ ] Memory retrieval is transcript-visible when it affects answers.

## Verification Plan

Run memory extraction tests with true facts, false claims, secrets, and stale file references.

