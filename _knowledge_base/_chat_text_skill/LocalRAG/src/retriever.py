"""Retriever module for Local RAG — hybrid search, reranking, MMR, and caching."""

import hashlib
import json
import time
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
from dataclasses import dataclass, field
from cachetools import TTLCache
from vectorstore import VectorStore
from observability import get_logger, traced


# ============================================================================
# Query Cache
# ============================================================================

@dataclass
class QueryCacheStats:
    hits: int = 0
    misses: int = 0

    @property
    def hit_rate(self) -> float:
        total = self.hits + self.misses
        return self.hits / total if total > 0 else 0.0


class QueryCache:
    """LRU+TTL cache for retrieval results.

    Key = sha256({"q": query, "f": sorted(filter_sources)}), cache miss on any config change.
    """

    def __init__(self, ttl: int = 300, max_size: int = 1000):
        self._cache = TTLCache(maxsize=max_size, ttl=ttl)
        self._stats = QueryCacheStats()
        self.logger = get_logger(__name__)

    def _make_key(self, query: str, filter_sources: Optional[List[str]]) -> str:
        key_data = {
            "q": query,
            "f": sorted(filter_sources) if filter_sources else None,
        }
        return hashlib.sha256(
            json.dumps(key_data, sort_keys=True, ensure_ascii=True).encode()
        ).hexdigest()[:32]

    def get(self, query: str, filter_sources: Optional[List[str]]) -> Optional[List[Dict[str, Any]]]:
        key = self._make_key(query, filter_sources)
        result = self._cache.get(key)
        if result is not None:
            self._stats.hits += 1
            self.logger.debug("Query cache hit", key=key[:8])
        else:
            self._stats.misses += 1
            self.logger.debug("Query cache miss", key=key[:8])
        return result

    def set(self, query: str, filter_sources: Optional[List[str]], results: List[Dict[str, Any]]) -> None:
        key = self._make_key(query, filter_sources)
        self._cache[key] = results
        self.logger.debug("Query cache set", key=key[:8], size=len(results))

    def get_stats(self) -> QueryCacheStats:
        return QueryCacheStats(hits=self._stats.hits, misses=self._stats.misses)

    def clear(self) -> None:
        self._cache.clear()
        self._stats = QueryCacheStats()
        self.logger.info("Query cache cleared")


# ============================================================================
# BM25 Sparse Indexer
# ============================================================================

class BM25Indexer:
    """BM25 sparse retrieval index over all stored documents.

    Built lazily on first search; rebuilds when collection changes.
    """

    def __init__(self, k1: float = 1.5, b: float = 0.75):
        self.k1 = k1
        self.b = b
        self._index: Optional[Any] = None  # rank_bm25.BM25Plus
        self._corpus: List[Dict[str, Any]] = []  # id -> doc dict
        self._doc_texts: List[str] = []
        self.logger = get_logger(__name__)

    def build(self, chunks: List[Dict[str, Any]]) -> None:
        """Build BM25 index from a list of chunk dicts.

        Args:
            chunks: List of dicts with at least 'id', 'text', 'source_file',
                    'chunk_index', 'score' keys.
        """
        try:
            from rank_bm25 import BM25Plus
        except ImportError:
            self.logger.warning("rank_bm25 not installed, sparse search unavailable")
            self._index = None
            return

        self._corpus = chunks
        self._doc_texts = [chunk.get("text", "") for chunk in chunks]
        # Tokenize: simple whitespace split (BM25Plus handles tokenization internally)
        tokenized = [text.lower().split() for text in self._doc_texts]
        self._index = BM25Plus(tokenized, k1=self.k1, b=self.b)
        self.logger.info("BM25 index built", num_docs=len(chunks))

    def search(
        self,
        query: str,
        top_k: int = 20,
        filter_sources: Optional[List[str]] = None,
    ) -> List[Dict[str, Any]]:
        """Search BM25 index.

        Args:
            query: Query string.
            top_k: Number of results to return.
            filter_sources: Optional document filter.

        Returns:
            List of result dicts (id, text, source_file, chunk_index, score).
        """
        if self._index is None:
            return []

        tokenized_query = query.lower().split()
        raw_scores = self._index.get_scores(tokenized_query)

        # Pair (index, score) and sort descending
        scored = sorted(enumerate(raw_scores), key=lambda x: x[1], reverse=True)

        results = []
        for idx, score in scored:
            if score <= 0:
                break
            chunk = self._corpus[idx]
            if filter_sources and chunk.get("source_file") not in filter_sources:
                continue
            results.append({
                "id": chunk.get("id", f"bm25_{idx}"),
                "text": chunk.get("text", ""),
                "source_file": chunk.get("source_file", "unknown"),
                "chunk_index": chunk.get("chunk_index", 0),
                "score": float(score),
            })
            if len(results) >= top_k:
                break

        return results

    def is_ready(self) -> bool:
        return self._index is not None

    def invalidate(self) -> None:
        self._index = None
        self._corpus = []
        self._doc_texts = []
        self.logger.debug("BM25 index invalidated")


# ============================================================================
# Reranker (Cross-Encoder + embedding fallback)
# ============================================================================

class CrossEncoderReranker:
    """Reranks retrieval candidates using cross-encoder (local/HuggingFace/API) or embedding fallback.

    Modes of operation (checked in order):
        1. API mode   — api_base is provided: POST rerank requests to the API
        2. Local mode — model_path is a local directory: load from disk
        3. HuggingFace — load from HuggingFace Hub (requires network + sentence-transformers)
        4. Disabled    — model_path is empty: use embedding fallback
    """

    def __init__(
        self,
        api_base: str = "",
        api_key: str = "not-needed",
        model_path: str = "",
    ):
        """Initialize the reranker.

        Args:
            api_base: Base URL for reranking API (e.g. "http://localhost:8000/v1").
                      If provided, all reranking is done via this API.
            api_key: API key for the reranking service.
            model_path: Path to a local cross-encoder model, or HuggingFace model ID.
                        Leave empty to disable reranking.
        """
        self.api_base = api_base.rstrip("/")
        self.api_key = api_key
        self.model_path = model_path
        self._model: Optional[Any] = None
        self._mode: str = "disabled"  # "api", "local", "huggingface", "disabled"
        self._available = False
        self.logger = get_logger(__name__)
        self._init_model()

    def _init_model(self) -> None:
        # Priority 1: API mode
        if self.api_base:
            self._mode = "api"
            self._available = True
            self.logger.info("CrossEncoder reranker configured for API mode", api_base=self.api_base)
            return

        # Priority 2: Disabled
        if not self.model_path or self.model_path.strip() == "":
            self._mode = "disabled"
            self.logger.warning("No rerank model configured, using embedding fallback")
            return

        # Priority 3: Local path
        local_path = Path(self.model_path)
        if local_path.is_dir():
            self._mode = "local"
            try:
                from sentence_transformers import CrossEncoder
                self._model = CrossEncoder(str(local_path), max_length=512)
                self._available = True
                self.logger.info("CrossEncoder loaded from local path", path=str(local_path))
            except Exception as e:
                self.logger.warning(f"Failed to load CrossEncoder from local path: {e}, using embedding fallback")
            return

        # Priority 4: HuggingFace Hub
        self._mode = "huggingface"
        try:
            from sentence_transformers import CrossEncoder
            self._model = CrossEncoder(self.model_path, max_length=512)
            self._available = True
            self.logger.info("CrossEncoder loaded from HuggingFace", model=self.model_path)
        except ImportError:
            self.logger.warning("sentence-transformers not installed, using embedding fallback reranker")
        except Exception as e:
            self.logger.warning(f"Failed to load CrossEncoder: {e}, using embedding fallback")

    def rerank(
        self,
        query: str,
        candidates: List[Dict[str, Any]],
        top_k: int = 5,
    ) -> List[Dict[str, Any]]:
        """Rerank candidates by (query, passage) relevance.

        Args:
            query: User query.
            candidates: List of candidate dicts with 'text' key.
            top_k: Number of top results to return.

        Returns:
            Re-ranked list of candidate dicts with added 'rerank_score'.
        """
        if not candidates:
            return []

        if self._mode == "api":
            return self._rerank_api(query, candidates, top_k)
        elif self._available and self._model is not None:
            return self._rerank_local(query, candidates, top_k)
        else:
            return self._rerank_embedding_fallback(query, candidates, top_k)

    def _rerank_api(
        self,
        query: str,
        candidates: List[Dict[str, Any]],
        top_k: int,
    ) -> List[Dict[str, Any]]:
        """Rerank via API endpoint. Expected POST body: {"query": ..., "texts": [...]}
        Response: {"scores": [...]} or {"results": [{"index": i, "score": s}]}"""
        import requests
        try:
            headers = {"Content-Type": "application/json"}
            if self.api_key and self.api_key != "not-needed":
                headers["Authorization"] = f"Bearer {self.api_key}"

            # Try common API formats
            body = {
                "query": query,
                "texts": [c.get("text", "") for c in candidates],
            }
            resp = requests.post(
                f"{self.api_base}/rerank",
                json=body,
                headers=headers,
                timeout=30,
            )
            resp.raise_for_status()
            data = resp.json()

            # Parse response — support multiple formats
            if "results" in data:
                # Format: {"results": [{"index": 0, "score": 0.9}, ...]}
                score_map = {r["index"]: r["score"] for r in data["results"]}
                scores = [score_map.get(i, 0.0) for i in range(len(candidates))]
            elif "scores" in data:
                # Format: {"scores": [0.9, 0.8, ...]}
                scores = data["scores"]
            else:
                raise ValueError(f"Unknown rerank API response format: {data}")

            scored = [{**c, "rerank_score": float(scores[i])} for i, c in enumerate(candidates)]
            scored.sort(key=lambda x: x["rerank_score"], reverse=True)
            return scored[:top_k]
        except Exception as e:
            self.logger.warning(f"API rerank failed: {e}, falling back")
            return self._rerank_embedding_fallback(query, candidates, top_k)

    def _rerank_local(
        self,
        query: str,
        candidates: List[Dict[str, Any]],
        top_k: int,
    ) -> List[Dict[str, Any]]:
        """Use locally loaded cross-encoder for pairwise scoring."""
        try:
            pairs = [(query, c.get("text", "")) for c in candidates]
            scores = self._model.predict(pairs)

            scored = [
                {**c, "rerank_score": float(scores[i])}
                for i, c in enumerate(candidates)
            ]
            scored.sort(key=lambda x: x["rerank_score"], reverse=True)
            return scored[:top_k]
        except Exception as e:
            self.logger.warning(f"CrossEncoder rerank failed: {e}, falling back")
            return self._rerank_embedding_fallback(query, candidates, top_k)

    def _rerank_embedding_fallback(
        self,
        query: str,
        candidates: List[Dict[str, Any]],
        top_k: int,
    ) -> List[Dict[str, Any]]:
        """Fallback: use relevance_score from original retrieval as rerank score."""
        scored = [
            {**c, "rerank_score": c.get("relevance_score", 0.0) + c.get("score", 0.0)}
            for c in candidates
        ]
        scored.sort(key=lambda x: x["rerank_score"], reverse=True)
        return scored[:top_k]


# ============================================================================
# MMR (Maximal Marginal Relevance)
# ============================================================================

def mmr_select(
    candidates: List[Dict[str, Any]],
    query_embedding: List[float],
    top_k: int,
    lambda_mult: float = 0.3,
) -> List[Dict[str, Any]]:
    r"""Select top-k results using Maximal Marginal Relevance.

    MMR = argmax_{d ∈ C\R} [ λ·sim(q,d) - (1-λ)·max_{r ∈ R} sim(d,r) ]

    Args:
        candidates: List of candidate dicts with 'embedding' key.
        query_embedding: Query vector.
        top_k: Number of results to select.
        lambda_mult: Trade-off between relevance (λ) and diversity (1-λ).

    Returns:
        Selected list of candidate dicts.
    """
    if not candidates or top_k <= 0:
        return []

    def cosine_sim(a: List[float], b: List[float]) -> float:
        dot = sum(x * y for x, y in zip(a, b))
        norm_a = sum(x * x for x in a) ** 0.5
        norm_b = sum(x * x for x in b) ** 0.5
        if norm_a == 0 or norm_b == 0:
            return 0.0
        return dot / (norm_a * norm_b)

    # Precompute query-candidate similarities
    for c in candidates:
        c["_q_sim"] = cosine_sim(query_embedding, c.get("embedding", [0.0]))

    selected: List[Dict[str, Any]] = []
    remaining = list(candidates)

    for _ in range(min(top_k, len(remaining))):
        best_score = -float("inf")
        best_idx = 0

        for i, candidate in enumerate(remaining):
            relevance = candidate["_q_sim"]

            # Max similarity to already selected
            max_sim_to_selected = 0.0
            if selected:
                max_sim_to_selected = max(
                    cosine_sim(candidate.get("embedding", [0.0]), s.get("embedding", [0.0]))
                    for s in selected
                )

            mmr_score = lambda_mult * relevance - (1 - lambda_mult) * max_sim_to_selected

            if mmr_score > best_score:
                best_score = mmr_score
                best_idx = i

        selected.append(remaining.pop(best_idx))

    # Clean up temp keys
    for c in candidates:
        c.pop("_q_sim", None)

    return selected


# ============================================================================
# Hybrid Retriever
# ============================================================================

class HybridRetriever:
    """Full-featured retriever: hybrid search + reranking + MMR + query cache."""

    def __init__(
        self,
        embedder,
        vectorstore: VectorStore,
        top_k: int = 5,
        # Hybrid search
        hybrid_search: bool = True,
        dense_top_k: int = 20,
        sparse_top_k: int = 20,
        rrf_k: float = 60.0,
        # Reranking
        rerank: bool = True,
        rerank_top_k: int = 20,
        rerank_api_base: str = "",
        rerank_api_key: str = "not-needed",
        rerank_model: str = "cross-encoder/ms-marco-MiniLM-L-6-v2",
        # MMR
        mmr_enabled: bool = True,
        mmr_lambda: float = 0.3,
        # Cache
        query_cache_ttl: int = 300,
        query_cache_size: int = 1000,
    ):
        self.embedder = embedder
        self.vectorstore = vectorstore
        self.top_k = top_k
        self.hybrid_search = hybrid_search
        self.dense_top_k = dense_top_k
        self.sparse_top_k = sparse_top_k
        self.rrf_k = rrf_k
        self.rerank = rerank
        self.rerank_top_k = rerank_top_k
        self.mmr_enabled = mmr_enabled
        self.mmr_lambda = mmr_lambda
        self.logger = get_logger(__name__)

        self.query_cache = QueryCache(ttl=query_cache_ttl, max_size=query_cache_size)
        self.bm25_indexer = BM25Indexer()
        self.reranker = CrossEncoderReranker(
            api_base=rerank_api_base,
            api_key=rerank_api_key,
            model_path=rerank_model,
        )
        self._bm25_built = False

    def retrieve(
        self,
        query: str,
        top_k: Optional[int] = None,
        filter_sources: Optional[List[str]] = None,
    ) -> List[Dict[str, Any]]:
        """Retrieve relevant chunks with full retrieval pipeline.

        Pipeline: cache → embed → dense search → (optional BM25) → RRF fusion
                  → rerank → MMR → return

        Args:
            query: Query text.
            top_k: Override final result count.
            filter_sources: Optional document filter.

        Returns:
            List of result dicts with text, source, chunk_index, relevance_score.
        """
        k = top_k if top_k is not None else self.top_k

        self.logger.info("Hybrid retrieve",
                        query_length=len(query),
                        top_k=k,
                        has_filters=filter_sources is not None,
                        hybrid=self.hybrid_search,
                        rerank=self.rerank,
                        mmr=self.mmr_enabled)

        # 1. Check query cache
        cached = self.query_cache.get(query, filter_sources)
        if cached is not None:
            self.logger.info("Query cache hit, returning cached results", num=len(cached))
            return cached[:k]

        # 2. Embed query
        start = time.perf_counter()
        query_embedding = self.embedder.embed_single(query)
        embed_ms = (time.perf_counter() - start) * 1000
        self.logger.debug("Query embedded", duration_ms=round(embed_ms, 2))

        # 3. Dense (vector) search
        dense_results = self.vectorstore.search(
            query_embedding,
            top_k=self.dense_top_k,
            filter_sources=filter_sources,
        )
        self.logger.debug("Dense search done", num=len(dense_results))

        # 4. Sparse (BM25) search
        sparse_results: List[Dict[str, Any]] = []
        if self.hybrid_search:
            # Lazy-build BM25 index
            if not self._bm25_built or not self.bm25_indexer.is_ready():
                self._build_bm25_index(filter_sources)
            if self.bm25_indexer.is_ready():
                sparse_results = self.bm25_indexer.search(
                    query,
                    top_k=self.sparse_top_k,
                    filter_sources=filter_sources,
                )
            self.logger.debug("Sparse search done", num=len(sparse_results))

        # 5. RRF Fusion
        fused = self._rrf_fusion(dense_results, sparse_results)
        self.logger.debug("RRF fusion done", num=len(fused))

        # 6. Reranking
        candidates = fused
        if self.rerank and len(candidates) > 1:
            candidates = self.reranker.rerank(
                query,
                candidates,
                top_k=min(self.rerank_top_k, len(candidates)),
            )
            self.logger.debug("Reranking done", num=len(candidates))

        # 7. MMR diversity selection
        if self.mmr_enabled and len(candidates) > k:
            # Attach query embedding to candidates for MMR
            for c in candidates:
                c["embedding"] = query_embedding
            selected = mmr_select(
                candidates,
                query_embedding,
                top_k=k,
                lambda_mult=self.mmr_lambda,
            )
            candidates = selected
            self.logger.debug("MMR done", num=len(candidates))
        else:
            candidates = candidates[:k]

        # 8. Format output
        formatted = self._format_results(candidates)

        # 9. Cache results
        self.query_cache.set(query, filter_sources, formatted)

        cache_stats = self.query_cache.get_stats()
        self.logger.info("Retrieval completed",
                        num_results=len(formatted),
                        cache_hit_rate=round(cache_stats.hit_rate, 3))
        return formatted

    def _build_bm25_index(self, filter_sources: Optional[List[str]] = None) -> None:
        """Build or rebuild the BM25 index from the vector store."""
        self.logger.info("Building BM25 index", filter_sources=filter_sources is not None)

        try:
            # Get all chunks from vector store
            # Use get_indexed_documents approach: get all IDs and fetch
            all_data = self.vectorstore.collection.get(
                include=["documents", "metadatas", "embeddings"]
            )

            chunks: List[Dict[str, Any]] = []
            for i in range(len(all_data["ids"])):
                metadata = all_data["metadatas"][i]
                source = metadata.get("source_file", "unknown")
                if filter_sources and source not in filter_sources:
                    continue
                chunks.append({
                    "id": all_data["ids"][i],
                    "text": all_data["documents"][i],
                    "source_file": source,
                    "chunk_index": metadata.get("chunk_index", 0),
                    "score": 0.0,
                    "embedding": all_data["embeddings"][i] if all_data["embeddings"] else [0.0] * 1536,
                })

            self.bm25_indexer.build(chunks)
            self._bm25_built = True
            self.logger.info("BM25 index built", num_chunks=len(chunks))
        except Exception as e:
            self.logger.error("Failed to build BM25 index", error=str(e))
            self._bm25_built = False

    def _rrf_fusion(
        self,
        dense_results: List[Dict[str, Any]],
        sparse_results: List[Dict[str, Any]],
    ) -> List[Dict[str, Any]]:
        """Reciprocal Rank Fusion of dense and sparse results.

        RRF score = Σ 1/(k + rank_i) for each ranker i.
        """
        if not sparse_results:
            return dense_results
        if not dense_results:
            return sparse_results

        # Map doc_id -> combined RRF score
        rrf_scores: Dict[str, Dict[str, Any]] = {}

        for rank, doc in enumerate(dense_results, start=1):
            doc_id = doc.get("id", f"dense_{rank}")
            if doc_id not in rrf_scores:
                rrf_scores[doc_id] = {**doc, "id": doc_id, "rrf_score": 0.0}
            rrf_scores[doc_id]["rrf_score"] += 1.0 / (self.rrf_k + rank)

        for rank, doc in enumerate(sparse_results, start=1):
            doc_id = doc.get("id", f"sparse_{rank}")
            if doc_id not in rrf_scores:
                rrf_scores[doc_id] = {**doc, "id": doc_id, "rrf_score": 0.0}
            rrf_scores[doc_id]["rrf_score"] += 1.0 / (self.rrf_k + rank)

        fused = sorted(rrf_scores.values(), key=lambda x: x["rrf_score"], reverse=True)
        return fused

    def _format_results(self, results: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Format results for output."""
        formatted = []
        for result in results:
            formatted.append({
                "text": result.get("text", ""),
                "source": result.get("source_file", result.get("source", "unknown")),
                "chunk_index": result.get("chunk_index", 0),
                "relevance_score": round(result.get("rerank_score", result.get("rrf_score", result.get("score", 0.0))), 4),
            })
        return formatted

    def invalidate_cache(self) -> None:
        """Invalidate query cache and BM25 index."""
        self.query_cache.clear()
        self.bm25_indexer.invalidate()
        self._bm25_built = False
        self.logger.info("Cache and BM25 index invalidated")


# ============================================================================
# Original Retriever — backward-compatible wrapper
# ============================================================================

class Retriever:
    """Retrieves relevant chunks from vector store based on query.

    Delegates to HybridRetriever when any advanced feature is enabled,
    otherwise uses simple vector search.
    """

    def __init__(
        self,
        embedder,
        vectorstore: VectorStore,
        top_k: int = 5,
        # Legacy / simple mode config
        hybrid_search: bool = False,
        rerank: bool = False,
        mmr_enabled: bool = False,
        query_cache_ttl: int = 300,
        query_cache_size: int = 1000,
        # Hybrid params (used when hybrid_search=True)
        dense_top_k: int = 20,
        sparse_top_k: int = 20,
        rrf_k: float = 60.0,
        rerank_top_k: int = 20,
        rerank_api_base: str = "",
        rerank_api_key: str = "not-needed",
        rerank_model: str = "",
        mmr_lambda: float = 0.3,
    ):
        self.embedder = embedder
        self.vectorstore = vectorstore
        self.top_k = top_k
        self.logger = get_logger(__name__)

        # Use HybridRetriever if any advanced feature is enabled
        self._hybrid: Optional[HybridRetriever] = None
        if hybrid_search or rerank or mmr_enabled:
            self._hybrid = HybridRetriever(
                embedder=embedder,
                vectorstore=vectorstore,
                top_k=top_k,
                hybrid_search=hybrid_search,
                dense_top_k=dense_top_k,
                sparse_top_k=sparse_top_k,
                rrf_k=rrf_k,
                rerank=rerank,
                rerank_top_k=rerank_top_k,
                rerank_api_base=rerank_api_base,
                rerank_api_key=rerank_api_key,
                rerank_model=rerank_model,
                mmr_enabled=mmr_enabled,
                mmr_lambda=mmr_lambda,
                query_cache_ttl=query_cache_ttl,
                query_cache_size=query_cache_size,
            )
            self.logger.info("Retriever initialized in hybrid mode",
                           hybrid=hybrid_search, rerank=rerank, mmr=mmr_enabled)

    def retrieve(
        self,
        query: str,
        top_k: Optional[int] = None,
        filter_sources: Optional[List[str]] = None,
    ) -> List[Dict[str, Any]]:
        if self._hybrid is not None:
            return self._hybrid.retrieve(query, top_k=top_k, filter_sources=filter_sources)

        # Simple mode: vector search with optional query cache
        if not hasattr(self, "_simple_cache"):
            self._simple_cache = QueryCache(ttl=300, max_size=1000)

        cached = self._simple_cache.get(query, filter_sources)
        if cached is not None:
            return cached[:top_k or self.top_k]

        k = top_k if top_k is not None else self.top_k
        query_embedding = self.embedder.embed_single(query)
        results = self.vectorstore.search(query_embedding, top_k=k, filter_sources=filter_sources)
        formatted = self._format_results(results)
        self._simple_cache.set(query, filter_sources, formatted)
        return formatted

    def _format_results(self, results: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        formatted = []
        for result in results:
            formatted.append({
                "text": result["text"],
                "source": result["source_file"],
                "chunk_index": result["chunk_index"],
                "relevance_score": round(result["score"], 4),
            })
        return formatted
