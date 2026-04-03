# Task 070: Hierarchical Evidence Compaction

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 027: Implement Hierarchical Evidence Compaction

## Context
`compact_evidence_once` currently processes raw output in a single call. Large outputs can exceed model context or cause poor summary quality.

## Objective
Implement a "Map-Reduce" style compaction for large shell outputs:
- Split large outputs into manageable chunks.
- Process chunks in parallel to extract key facts.
- Synthesize chunk-level facts into a final, coherent evidence summary.
- Update `src/intel_compression.rs` (from Task 024) with this logic.

## Success Criteria
- System handles 10,000+ line command outputs without context overflow.
- Summary quality remains high regardless of input size.
