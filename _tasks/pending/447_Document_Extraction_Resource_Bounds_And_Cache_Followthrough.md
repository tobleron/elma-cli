# Task 447: Document Extraction Resource Bounds And Cache Followthrough

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 258, completed Task 259, completed Task 260, completed Task 276

## Summary

Make document detection, extraction, chunking, and cache behavior resource-bounded and truthful.

## Evidence From Audit

- `DocumentFormat::detect` calls `std::fs::read(path)`, reading the whole file for magic-byte detection.
- `read_file_with_budget` extracts documents fully and only then applies `DocumentReadBudget`.
- `DocumentIndexCache::save` and `load_cache` are placeholders that do not persist.
- PDF extraction can load all pages before selecting budgeted output.
- `calculate_document_signature` uses `DefaultHasher`, which is not a stable persisted content hash.

## User Decision Gate

Ask the user to choose resource budgets:

- Maximum bytes read for sniffing.
- Maximum document size for one-pass extraction.
- Cache persistence location and retention policy.
- Whether huge documents should default to retrieval-first planning.

## Implementation Plan

1. Replace whole-file sniffing with bounded header reads.
2. Add size-aware staged extraction before full extraction.
3. Implement persistent cache save/load or remove the cache abstraction until it is real.
4. Replace `DefaultHasher` with a stable content signature for persisted cache keys.
5. Add tests with synthetic large files and oversized document behavior.

## Success Criteria

- [ ] Format detection reads bounded bytes.
- [ ] Large documents do not require full extraction before budget decisions.
- [ ] Cache behavior is either real or explicitly removed.
- [ ] Huge-document decisions are visible to the user.
- [ ] Tests prove bounded behavior.

## Anti-Patterns To Avoid

- Do not silently truncate documents and present summaries as complete.
- Do not add network or OCR behavior in this task.
- Do not persist unstable hash values as long-term identities.
