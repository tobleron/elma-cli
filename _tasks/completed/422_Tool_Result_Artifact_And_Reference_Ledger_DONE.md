# Task 422: Tool Result Artifact And Reference Ledger

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Dependencies:** Task 381, Task 391
**References:** source-agent parity: reference tools, grounded summaries, evidence continuity

## Objective

Create a reference ledger that maps tool outputs, generated artifacts, citations, and final-answer claims to stable session references.

## Problem

As Elma gains more tools, large outputs and artifacts cannot all stay in prompt context. The system needs stable references that can be summarized, cited, reloaded, and used by finalization without hallucinating.

## Implementation Plan

1. Define `ReferenceEntry` with id, kind, path/source, summary, timestamp, and evidence hash.
2. Register outputs from read/search/fetch/browser/download/interpreter/background tools.
3. Let finalization cite reference ids internally while rendering clean plain text externally.
4. Expose reference events as collapsible transcript rows.
5. Add cleanup behavior tied to session retention.

## Verification

```bash
cargo test reference
cargo test evidence
cargo test session
cargo build
```

## Done Criteria

- Large tool outputs can be represented by stable references.
- Final answers can be checked against references.
- References survive session resume.
- Missing references fail closed.

