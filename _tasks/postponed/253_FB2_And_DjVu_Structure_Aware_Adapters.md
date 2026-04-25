# 253 - FB2 And DjVu Structure-Aware Adapters

Status: Pending
Priority: P1
Depends on: 249

## Goal

Bring FictionBook and DjVu into the normalized extraction pipeline with useful structure and quality reporting.

## FB2 Requirements

FB2 is XML-based and should be a strong Rust-native target.

- Use the `fb2` crate or direct `quick-xml` traversal.
- Extract title, authors, language, genre, annotation, sequence, and publication info when present.
- Preserve body, section, title, poem, stanza, cite, table, and footnote structure where practical.
- Ignore embedded binary images except for metadata inventory.
- Emit one or more `DocumentUnit` records per section or major body element.

## DjVu Requirements

DjVu support already exists through `djvu-rs`, but it is flat.

- Preserve page numbers.
- Extract NAVM/bookmark/TOC data if exposed by the crate.
- Preserve text-layer zone hierarchy when feasible.
- Report image-only/no-text-layer documents explicitly.
- Do not add OCR by default.

## Acceptance Criteria

- FB2 fixtures produce section-aware chunks with title and author metadata.
- DjVu fixtures with text layers produce page-aware chunks.
- DjVu image-only fixtures fail clearly with `requires_text_layer`.
- Neither adapter emits empty chunks.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test fb2_document_adapter`
- `cargo test djvu_document_adapter`
- Real CLI validation for at least one FB2 or DjVu fixture when available.

