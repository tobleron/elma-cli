# Task 440: Document Capability Truth And Adapter Reconciliation

**Status:** pending
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

## User Decision Gate

Ask the user which formats should be implemented now and which should be truthfully downgraded:

- EPUB full spine/TOC extraction.
- DjVu text-layer extraction.
- MOBI/AZW extraction.
- DOCX ZIP/XML extraction.
- RTF fallback extraction.

For any format not implemented now, update capability reporting to say unavailable or degraded.

## Implementation Plan

1. Build a capability matrix from enum variants, match arms, backend names, docs, and tests.
2. Add failing regression tests for formats that currently claim support without real extraction.
3. Implement or downgrade each format according to user choice.
4. Update completed-task notes only if the project policy allows correcting historical artifacts; otherwise add a reconciliation note in this task.
5. Add fixture-backed tests for implemented formats and explicit failure tests for unsupported formats.

## Success Criteria

- [ ] `document_capabilities()` matches real code behavior.
- [ ] No unsupported format routes through an unrelated extractor.
- [ ] EPUB/DjVu/MOBI claims are backed by fixtures or downgraded.
- [ ] Document read failures are explicit and user-visible.
- [ ] Docs match implementation status.

## Anti-Patterns To Avoid

- Do not mark a framework stub as full support.
- Do not silently fall back to raw binary/plaintext reads for structured formats.
- Do not add network or OCR dependencies for this task.
