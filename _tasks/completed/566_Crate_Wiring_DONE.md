# 566 — Wire Unused Crate Integrations (djvu, mobi, epub)

- **Priority**: Medium
- **Category**: Architecture
- **Depends on**: None
- **Blocks**: None

## Problem Statement

The `Cargo.toml` declares dependencies on `djvu-rs`, `mobi`, and `epub` crates, but the extraction/document adapter pipeline may not fully wire these formats. According to `docs/_proposals/004-crate-reconciliation.md` and `docs/_proposals/005-extraction-integration.md`, some of these integrations are incomplete.

The `document_adapter.rs` file (1397+ lines) handles document format detection and text extraction, but needs audit for missing format support.

## Why This Matters for Small Local LLMs

Document intelligence is one of Elma's key skills. If extraction silently fails for certain formats, the model will try to answer questions based on incomplete or missing document content — leading to hallucinated answers about document contents.

## Current Behavior

The document adapter presumably handles PDF (via `pdf-extract`), EPUB (via `epub`), and HTML (via `html2text`). The status of `djvu-rs`, `mobi`, and secondary format support needs verification.

## Recommended Target Behavior

1. Audit all format handlers in `document_adapter.rs`
2. Wire any unwired format extractors
3. Add capability reporting so the model knows which formats are supported
4. Add "format not supported" error for missing extractors (instead of silent failure)

## Source Files That Need Modification

- `src/document_adapter.rs` — Audit and wire format handlers
- `Cargo.toml` — Review feature flags for document crates

## New Files/Modules

Potentially: `src/document_formats/djvu.rs`, `src/document_formats/mobi.rs` if extractors are needed

## Step-by-Step Implementation Plan

1. Run `cargo tree -p elma-cli` and verify all listed document crates are compiled in
2. For each crate, trace the usage in `document_adapter.rs`
3. Identify gaps where a format's extractor is compiled but not called
4. Add extraction support for any missing formats
5. Update `document_capabilities()` to accurately report supported formats
6. Add tests with sample documents for each format
7. Remove any crates that cannot be practically wired

## Recommended Crates

All already in Cargo.toml — no new crates needed.

## Validation/Sanitization Strategy

- All document extraction must be bounded (max file size, max extraction time)
- Document content must be sanitized (no binary data in text output)
- Failed extractions must produce clear error messages for the model

## Testing Plan

1. Test each format with a small sample document
2. Test format detection (correct mime type → correct extractor)
3. Test that extraction failures produce clear errors
4. Test that corrupted documents don't crash the extractor

## Acceptance Criteria

- All document crate dependencies are either wired or removed
- `document_capabilities()` accurately reports supported formats
- Each wired format has at least one test
- Unwired crates are removed from `Cargo.toml`

## Risks and Migration Notes

- Some crates may be abandoned or have security issues. Check crate health before wiring.
- If a format cannot be reliably extracted, remove the dependency rather than shipping broken support.
