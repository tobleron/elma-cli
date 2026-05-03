# Task 465: Persistent Project Memory And RAG Index

**Status:** pending
**Source patterns:** OpenCrabs hybrid memory, AgenticSeek memory manager, LocalAGI vector stores
**Revives:** `_tasks/postponed/078_Long_Term_Tactical_Memory.md`, `_tasks/postponed/081_Analogy_Based_Reasoning_Engine.md`
**Depends on:** completed Task 287 (evidence ledger), Task 463 (repo map)

## Summary

Add a persistent project memory and retrieval index that stores verified facts, decisions, resolved paths, and reusable task patterns with evidence pointers and staleness controls.

## Why

Elma should learn local project facts without hallucinating from stale memory. Reference agents use memory and retrieval to reduce repeated investigation. The key for Elma is to store only grounded, source-linked facts and invalidate them when files change.

## Implementation Plan

1. Define memory entry types: project fact, path correction, decision, workflow pattern, and resolved failure.
2. Attach every memory entry to evidence IDs, file hashes, or session events.
3. Add retrieval by semantic/hybrid search with strict token budgets.
4. Mark entries stale when referenced files change.
5. Add user-visible controls to inspect and delete memory entries.

## Success Criteria

- [ ] Memory entries always include provenance.
- [ ] Stale entries are excluded or clearly marked.
- [ ] Retrieval improves repeated repo questions without broad scans.
- [ ] User can inspect and remove project memory.
- [ ] Tests cover staleness, provenance, and token-budgeted retrieval.

## Anti-Patterns To Avoid

- Do not store unsupported model guesses as memory.
- Do not let memory override fresh file evidence.
- Do not create a hidden personalization store without user control.
