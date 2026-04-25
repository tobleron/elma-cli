# 260 - Hybrid Document Retrieval BM25 Embeddings And Source Diversity

Status: completed
Priority: P1

## Goal

Replace substring search with a retrieval layer suitable for ebook chat on local models.

## Retrieval Design

Use a staged local-first design:

1. Lexical retrieval with BM25 or Tantivy.
2. Optional embedding retrieval through Elma's configured local model/provider path.
3. Hybrid fusion using reciprocal rank fusion or a similar deterministic combiner.
4. Source diversity using max-per-document and max-per-section controls.
5. Optional rerank only when a reliable local reranker is configured.

## Backend Plan

- Evaluate `tantivy` for local full-text indexing and BM25.
- Keep embeddings optional because Elma must remain local-first and small-model-friendly.
- If adding a vector index, evaluate simple cosine over cached embeddings first before adding HNSW.
- Evaluate `fastembed` only as an optional local embedding backend because it introduces model download/cache concerns.

## Requirements

- No hardcoded user-word routing.
- Retrieval can use query terms and embeddings; route decisions must remain model/evidence based.
- Keep provenance with every hit.
- Support source filters: document, chapter, page range, format.
- Add query cache for repeated questions.
- Add thresholds and insufficient-evidence reporting.

## Acceptance Criteria

- Exact phrase questions find relevant chunks.
- Semantic paraphrase questions improve when embeddings are enabled.
- Retrieval does not return only one chapter when the answer requires multiple sections.
- Unsupported embedding backend falls back to lexical retrieval.
- Every retrieved hit carries citation-ready provenance.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_retrieval`
- Fixture questions with expected source chunks.
- Real CLI validation with one PDF and one EPUB.

