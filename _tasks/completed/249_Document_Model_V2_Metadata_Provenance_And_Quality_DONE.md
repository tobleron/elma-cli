# 249 - Document Model V2 Metadata Provenance And Quality

Status: Pending
Priority: P1
Depends on: 248

## Problem

The current model exposes chunks with a flat provenance string. That is too weak for ebook chat because answers need page, chapter, section, archive-entry, and extraction-quality provenance.

## Goal

Create a normalized V2 document model that all adapters must emit before retrieval or context loading.

## Required Types

- `DocumentId`: stable source identity based on canonical path plus content signature.
- `DocumentMetadata`: title, authors, language, publisher, date, ISBN/identifier, source path, file size, format, backend.
- `DocumentUnit`: extracted structural unit before chunking.
- `DocumentChunk`: token-sized retrievable unit after chunking.
- `DocumentQualityReport`: extraction warnings, text coverage, empty pages, encoding repairs, encrypted/DRM flags, image-only flags.
- `DocumentCapability`: format support state and backend explanation.

## Required Provenance Fields

Each unit and chunk must be able to carry:

- source path
- format
- backend
- page number when available
- chapter index and title when available
- section heading path when available
- archive entry path when extracted from containers
- byte or character offsets when feasible
- chunk index and total chunks

## Compatibility Requirements

- Keep a compatibility layer so existing `DocumentExtractionResult` callers do not break during migration.
- Do not force all adapters to fake page numbers.
- Do not make a missing metadata field an extraction failure.
- Keep serialization stable enough for cache files.

## Acceptance Criteria

- PDF units can carry page numbers.
- EPUB units can carry chapter and spine metadata.
- DOCX/HTML/FB2 units can carry heading paths.
- CBZ/IBA/CHM-like packages can carry archive-entry paths.
- Quality flags are visible to read results and later transcript telemetry.

## Verification

- `cargo fmt`
- `cargo build`
- Unit tests for model serialization and backward conversion.
- Golden JSON fixture for one PDF, one EPUB, one DOCX, and one unsupported format.

