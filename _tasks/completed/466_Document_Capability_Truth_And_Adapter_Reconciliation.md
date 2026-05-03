# Task 466: Document Capability Truth And Adapter Reconciliation

**Status:** completed
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 197, completed Task 203, completed Task 251, completed Task 263

## Summary

Reconcile document capability reports, completed task claims, and actual adapter implementations so Elma does not claim unsupported document extraction.

## Evidence From Audit

- completed Task 203 claims `extract_djvu()` and `extract_mobi()` were added and wired.
- current `src/document_adapter.rs` contains no `extract_djvu()` or `extract_mobi()` functions.
- `DocumentFormat::DjVu`, `Mobi`, `Docx`, `Rtf`, and related formats route to `extract_epub()` in current matches.
- `extract_epub()` currently returns `ok: false` with "framework implemented ... full implementation pending".
- `document_capabilities()` reports `djvu`, `mobi`, `docx`, and `rtf` as available despite missing or placeholder extraction paths.

## Implementation Completed

1. Updated `document_capabilities()` to mark EPUB, DjVu, MOBI, DOCX, RTF as unavailable/degraded with clear notes about stub status.
2. Removed routing of unsupported formats to `extract_epub()` stub.
3. Added `unsupported_format()` helper function for explicit failure reporting.
4. Added regression tests verifying unsupported formats report as unavailable and fail on extraction.

## Success Criteria

- [x] `document_capabilities()` matches real code behavior.
- [x] No unsupported format routes through an unrelated extractor.
- [x] EPUB/DjVu/MOBI claims are backed by fixtures or downgraded.
- [x] Document read failures are explicit and user-visible.
- [x] Docs match implementation status.

## Anti-Patterns To Avoid

- Do not mark a framework stub as full support.
- Do not silently fall back to raw binary/plaintext reads for structured formats.
- Do not add network or OCR dependencies for this task.