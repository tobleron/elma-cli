# 250 - PDF Page-Aware Rust Extraction Upgrade

Status: COMPLETE (2026-04-25)
Priority: P1
Depends on: 249

## Goal

Upgrade PDF extraction from whole-document text dumping to page-aware extraction with metadata, quality flags, and predictable failure modes.

## Backend Plan

- First use `pdf_extract::extract_text_by_pages` because `pdf-extract` is already in the dependency set and exposes page-separated extraction.
- Evaluate `lopdf` for metadata, page count, encryption detection, and optional fallback extraction.
- Do not add `pdfium-render` by default because it binds to Pdfium and changes distribution posture. It may be proposed later as an explicit feature-gated enhancement.
- Do not add OCR by default. Image-only scanned PDFs must report that no text layer is available.

## Implementation Requirements

- Preserve one `DocumentUnit` per page, with `page_number` set.
- Normalize page text with conservative cleanup:
  - remove repeated nulls and control junk
  - normalize whitespace
  - repair hyphenated line breaks only when the next line starts like a continuation
  - keep paragraph boundaries where possible
- Extract or infer metadata where feasible:
  - title
  - author
  - subject
  - creation/modification date
  - page count
  - encrypted/password-required state
- Detect low-quality extraction:
  - empty pages
  - very low character count per page
  - high replacement-character count
  - likely image-only document

## Acceptance Criteria

- A multi-page PDF produces page-numbered units and chunks.
- Empty text-layer PDFs fail with an explicit no-text-layer message.
- Encrypted PDFs fail cleanly unless a supported empty-password path succeeds.
- The read result can cite `page N`.
- PDF extraction never panics on malformed fixtures.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test pdf_document_adapter`
- Fixture tests for normal PDF, empty/scanned-like PDF, malformed PDF, and encrypted/password-required PDF if available.
- Real CLI validation against at least one larger PDF in the workspace.

