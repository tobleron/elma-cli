# 257 - Token And Structure-Aware Document Chunking

Status: Pending
Priority: P1
Depends on: 249 through 256 as adapters land

## Problem

Current chunking is a simple size-bytes/lines splitter. It can split inside UTF-8, ignore chapters/pages, lose headings, and produce chunks that are poor retrieval units.

## Goal

Replace flat byte chunking with a structure-aware chunking service that works for small local models and preserves semantic continuity.

## Chunking Strategy

Use this fallback order:

1. Existing document units: page, chapter, heading section, archive entry.
2. Markdown/HTML structure: headings, paragraphs, lists, tables, code blocks.
3. Paragraph boundaries.
4. Sentence boundaries.
5. Word boundaries.
6. Grapheme/character boundaries only as a final fallback.

## Backend Plan

- Evaluate `text-splitter` because it supports semantic splitting and token/character sizing.
- If `text-splitter` is added, keep Elma-specific provenance and overlap logic outside the crate.
- If not added, implement a minimal recursive splitter using Unicode-safe boundaries and Elma's token estimator.

## Requirements

- Configurable target tokens, max tokens, and overlap tokens.
- Never split inside invalid UTF-8.
- Carry heading/page/chapter provenance into every chunk.
- Add chunk hashes for cache stability.
- Avoid crossing document-unit boundaries unless explicitly allowed.
- Record chunking strategy in the quality report.

## Acceptance Criteria

- Long paragraphs are split without panics or invalid UTF-8.
- Page/chapter provenance survives chunking.
- Chunks fit configured token budgets.
- Overlap exists where useful but does not duplicate entire chunks.
- Retrieval tests show better locality than the old splitter.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_chunking`
- Property-style tests for random Unicode input.
- Golden chunk tests for PDF pages, EPUB chapters, HTML headings, and DOCX paragraphs.

