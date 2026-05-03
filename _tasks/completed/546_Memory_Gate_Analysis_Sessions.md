# Task 546: Memory Gate Workspace Evidence Requirement

**Status:** pending
**Priority:** MEDIUM
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P12 — Medium Confidence

## Summary

The memory gate was skipped at the end of the session with `memory_gate_status=skip reason=missing_workspace_evidence`. The session produced an architecture summary, 5 risk identifications, and a patch proposal — all high-value findings. None of this was persisted to memory. The next session starts with no awareness of these findings.

The requirement for "workspace evidence" as the gate condition is too strict for analysis sessions that primarily read and observe rather than mutate files.

## Evidence

- `trace_debug.log` line 140: `memory_gate_status=skip reason=missing_workspace_evidence`
- Session produced: architecture summary, 5 risks (some verified, some assumed), a patch proposal, a next-step checklist
- No memory written to `_knowledge_base/` or session-level persistent storage

## Root Cause

The memory gate's condition requires "workspace evidence" — likely a mutation signal (file write, edit, patch apply). A read-only analysis session produces no such signal, so memory is never written even when the session produces valuable, reusable findings.

## Implementation Plan

1. Audit the memory gate logic: identify what constitutes "workspace evidence"
2. Add a second gate condition: `high_value_findings` — triggered when the session produces:
   - An architecture summary
   - ≥3 verified risks
   - A next-step checklist
   - Or any explicit `respond` call with a structured report
3. When `high_value_findings` is true, write a memory entry regardless of workspace mutations
4. The memory entry should capture: session ID, model, key findings summary, risks list, next-step checklist
5. Surface memory write success/skip as a transcript row: `ℹ️ Memory updated` or `ℹ️ Memory skipped: <reason>`

## Success Criteria

- [ ] Analysis sessions that produce architecture summaries or risk reports write to memory
- [ ] Memory write/skip is surfaced in the transcript
- [ ] Read-only sessions are not permanently excluded from memory
- [ ] Mutating sessions continue to write memory as before

## Verification

```bash
cargo build
# Run a read-only analysis session
# Verify memory gate does not skip
# Check _knowledge_base/ or session memory for persisted findings
```
