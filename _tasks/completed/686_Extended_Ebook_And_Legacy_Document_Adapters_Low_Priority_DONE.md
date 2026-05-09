# Task 686: Extended Ebook And Legacy Document Adapters Low Priority

**Status:** pending
**Priority:** LOW
**Type:** Offline Feature / Document Intelligence
**Scope:** `src/document_adapter.rs`, `src/hybrid_search.rs`, `tests/fixtures`
**Source:** postponed tasks 252-256 and 083

## Summary

Extend document adapters for additional ebook, comic, and legacy formats only after the persistent local index and resource-bound extraction path are stable.

## Evidence And Gap

- Document intelligence already exists, but old tasks list MOBI/AZW/KFX, FB2/DjVu, RTF/DOCX/DOC, CBZ/CBR/IBA, and exotic legacy formats.
- Adapter expansion should not destabilize core document extraction or require online services.

## Implementation Plan

1. Prioritize formats with reliable local parsers and clear licensing.
2. Add capability matrix entries: supported, best-effort, metadata-only, unsupported.
3. Enforce extraction size/time/memory bounds and provenance.
4. Feed extracted chunks into Task 668 indexing.

## Acceptance Criteria

- [ ] Unsupported formats fail clearly and locally.
- [ ] Supported adapters produce provenance-rich chunks.
- [ ] Resource limits prevent huge archives from stalling Elma.
- [ ] Tests use small fixtures and do not require network.

## Verification Plan

Run fixture extraction/indexing tests for each added format class.

