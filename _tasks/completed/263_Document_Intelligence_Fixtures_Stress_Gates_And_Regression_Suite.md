# 263 - Document Intelligence Fixtures Stress Gates And Regression Suite

Status: Pending
Priority: P1 gate for completing the ebook track
Depends on: 247 through 262

## Goal

Create a fixture and stress suite that proves document chat works through the real CLI path, not just isolated adapter functions.

## Required Fixture Classes

| Class | Examples |
|---|---|
| Full-text common | TXT, HTML, XHTML, PDF, EPUB, MOBI, FB2, DjVu text layer, DOCX, RTF |
| Degraded/common | image-only PDF, image-only DjVu, CBZ metadata-only, CBZ text sidecar |
| Recognized unsupported | DRM-like AZW/AZW3/KFX, LRX, LRF, LIT, unsupported CHM, ambiguous PDB |
| Corrupt/malformed | broken ZIP, malformed EPUB, invalid PDF, malformed XML |
| Large documents | synthetic long EPUB, long PDF, long TXT |

## Fixture Policy

- Prefer generated minimal fixtures when possible.
- Keep large copyrighted files out of the repo unless licensing is explicit.
- If using local-only stress files, document that they are optional and not required for CI.
- Each fixture must have an expected capability state and expected extraction summary.

## Required Tests

- Format detection tests.
- Adapter extraction tests.
- Model serialization tests.
- Chunking tests.
- Cache invalidation tests.
- Retrieval tests.
- Citation-answering tests.
- CLI read-path integration tests.
- PTY or real CLI tests for user-facing document behavior.

## Stress Gates

- Large PDF/EPUB extraction does not hang.
- Malformed files do not panic.
- Unsupported formats return within a bounded time.
- Re-reading cached books is faster and does not re-run extraction.
- Context planning never silently truncates full-document requests.

## Required Commands

- `cargo fmt`
- `cargo build`
- `cargo test`
- Relevant document-specific test filters.
- Real CLI validation for at least:
  - read a PDF
  - read an EPUB
  - ask a citation question
  - request a full-book summary
  - read an unsupported/degraded format

## Acceptance Criteria

- The ebook track cannot be archived until these tests pass or documented blockers are accepted.
- Every format in Task 248 has a fixture or an explicitly documented missing-fixture reason.
- The real CLI path demonstrates extraction metadata, planning, retrieval, and grounded final answers.

