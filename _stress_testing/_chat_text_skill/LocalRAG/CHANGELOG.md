# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- **UI Enhancements** — Conversation history, user feedback, document management:
  - `src/history_manager.py` — HistoryManager for chat history + feedback persistence (JSON)
  - New **📁 Documents Tab** — View indexed docs, chunk counts, delete per-doc or clear all
  - **RAG Tab sidebar** — Conversation history list, load/delete past conversations
  - **Feedback buttons** — 👍/👎 on each assistant answer, stored persistently
  - `tests/test_history_manager.py` — 16 tests for HistoryManager
- **Observability Dashboard** — New 📊 Observability Tab:
  - Log file viewer (last 50 lines, refreshable) + `logs/app.log` with rotation (10MB, 5 backups)
  - Prometheus metrics endpoint `http://localhost:9090/metrics` (background thread server)
  - Live preview: indexed docs count, chunks count, feedback stats
- **CrossEncoderReranker refactor** — API-style configuration:
  - `rerank_api_base`, `rerank_api_key`, `rerank_model` (local path / HuggingFace ID)
  - API mode: POST `{query, texts}` → `{scores}` or `{results}`
  - Local/HuggingFace mode: direct `sentence_transformers.CrossEncoder` loading
  - Disabled (empty): uses embedding fallback
  - `config/api_settings.yaml` new `rerank` section
- **Retrieval Enhancements** — Hybrid search, reranking, MMR, query cache (from plan)
- **Bug fix: embed_batch double-call** — `_call_embeddings_api` ternary was calling API twice for single text
- **Bug fix: embedder validation** — Added dimension validation and error diagnostics

### Changed
- **4 tabs → 5 tabs**: Configuration, Chunking, Documents, RAG, Observability
- **openai pinned to 1.10.0** — langchain-core removed to avoid version conflict
- **Embedding fallback hardcoded dims** — Changed from 1536 to adaptive based on actual API response
  - `FixedSizeChunker` - Uniform fixed-size chunks with overlap
  - `RecursiveChunker` - Recursive separator-based splitting
  - `StructureChunker` - Heading + content as logical units
  - `SemanticChunker` - Embedding-based semantic boundary detection
  - `LLMChunker` - LLM-driven semantic chunking
- **StructureChunker redesign** - Now preserves heading + content as complete logical units
- **SemanticChunker redesign** - Follows article approach: sentence split → embed → merge by similarity
- **Multi-document filtering** - Filter retrieval by specific documents in RAG tab
- **Tab-based UI** - Reorganized UI into Configuration, Chunking, and RAG tabs
- **Config persistence** - API settings saved to `config/api_settings.yaml`
- **Observability infrastructure** - Structured logging, metrics, and tracing:
  - `src/observability.py` - Central module with structlog + prometheus-client
  - Prometheus metrics: rag_query_latency, ingest_latency, api_latency, vectorstore_operations
  - Request context manager and `@traced(phase, op)` decorator
- **Pipeline resilience** - Retry logic, batch size control, content validation, progress tracking
- **VectorStore 4 major improvements** (see below)

### Added
- **VectorStore upsert** - Incremental document updates:
  - `upsert_document(chunks, embedder)` compares content_hash to skip unchanged chunks
  - Embeddings stored in Chroma metadata — unchanged chunks reuse stored embeddings (no API call)
  - Only INSERT (new) / UPDATE (changed) / DELETE (removed) — no full re-index
  - Pipeline `ingest_document` now uses upsert automatically
- **VectorStore HNSW configuration** - Tune index for quality vs speed:
  - `hnsw_space` ("l2"/"cosine"/"ip"), `hnsw_ef_construction`, `hnsw_max_neighbors` at construction
  - `configure_ef_search(N)` updates ef_search on existing collections (no re-index)
  - `get_index_config()` inspects current HNSW settings
- **VectorStore backup export/import** - Persistent backup and restore:
  - `export_collection(filepath, compressed=True)` exports all chunks + HNSW config to gzip JSON
  - `import_collection(filepath, mode="replace"|"merge")` restores from backup
  - "replace" = clear and restore; "merge" = add new without overwriting
- **VectorStoreMulti multi-tenancy** - Collection-level tenant isolation:
  - `VectorStoreMulti` manages per-tenant `VectorStore` instances sharing one persist directory
  - `get_tenant(name)`, `list_tenants()`, `delete_tenant(name)`, `tenant_stats()`
  - Handles tenant names containing `/`, `:`, `_` via reversible encoding

### Fixed
- **PDF loading error handling** - Now raises clear `ValueError` for invalid PDFs
- **YAML config path issue** - Windows path escape sequence problem fixed
- **Circular import issues** - Separated registry into `_registry.py`
- **Chunk metadata field naming** - `source` vs `source_file` inconsistency resolved
- **Chroma API compatibility** - `collection.get(include=["ids"])` removed (not supported); use `get(include=["metadatas"])` instead

### Changed
- **app.py** - Complete UI redesign with tabs
- **StructureChunker** - New algorithm: heading + content as one logical unit
- **SemanticChunker** - New algorithm: sentence split → embed → similarity merge

## [0.1.0] - Initial Release

### Added
- Basic RAG pipeline with document loading, chunking, embedding, and retrieval
- Chroma vector storage
- Streamlit web interface
- OpenAI-compatible API support (LM Studio, Ollama, vLLM)
- Fixed size chunking strategy
- Initial test suite

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-03-20 | Initial commit with basic RAG pipeline |
| 0.2.0 | 2026-03-26 | 5 chunking strategies, UI redesign, multi-doc filtering |
| 0.3.0 | 2026-03-30 | VectorStore upsert, HNSW config, backup export, multi-tenancy |

## Migration Notes

### v0.1.0 → v0.2.0

**Config changes:**
- API settings now persist to `config/api_settings.yaml`
- `chunkers` package structure changed (use `from chunkers import create_chunker`)

**StructureChunker:**
- Now returns heading + content as single chunk (previously separate)
- Metadata includes `heading_text` (plain text) vs `heading` (markdown formatted)

**SemanticChunker:**
- Now uses sentence-level splitting before embedding (previously fixed-size)
- Merges by cosine similarity threshold

**New dependencies:**
- pypdf (for PDF loading)
