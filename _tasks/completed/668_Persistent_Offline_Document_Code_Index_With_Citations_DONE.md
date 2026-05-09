# Task 668: Persistent Offline Document Code Index With Citations

**Status:** pending
**Priority:** HIGH
**Type:** Architecture / Offline Feature
**Scope:** `src/document_adapter.rs`, `src/hybrid_search.rs`, `src/repo_map.rs`, `src/file_scout.rs`, `src/session_store.rs`
**Source:** deferred task 465, postponed document tasks, `_knowledge_base` pdfrag/LocalRAG

## Summary

Build a persistent local index for documents and code with provenance-rich chunks, offline search, and citation-ready retrieval.

## Evidence And Gap

- Elma has document extraction and hybrid search modules, but no durable per-workspace index with invalidation and citations.
- `_knowledge_base/_chat_text_skill/pdfrag` includes chunking/storage/search ideas.
- Local-first truthfulness improves when retrieval does not depend on re-reading whole files every turn.

## Implementation Plan

1. Define chunk metadata: path, hash, mtime, byte range, page/section/symbol, extractor version, and quality flags.
2. Persist index in SQLite or an embedded local store with FTS fallback and optional embeddings.
3. Add invalidation by hash/mtime and extractor version.
4. Add retrieval APIs/tools that return compact citation-ready evidence.
5. Keep all indexing offline by default.

## Acceptance Criteria

- [ ] Indexing works without internet or embedding services.
- [ ] Modified/deleted files invalidate stale chunks.
- [ ] Retrieved chunks include enough provenance for grounded answers.
- [ ] Large documents are chunked within memory/time bounds.

## Verification Plan

Index markdown, Rust, PDF/EPUB fixtures, modify files, query, and verify citations point to real ranges.

