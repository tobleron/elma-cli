# Task 070: Hierarchical Evidence Compaction

## Priority
**P2 - EFFICIENCY & OBSERVABILITY (Tier B)**
**Depends on:** Tier A stability (tasks 065-069)

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
