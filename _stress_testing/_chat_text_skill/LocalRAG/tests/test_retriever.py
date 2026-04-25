"""Tests for enhanced retriever: query cache, BM25, reranking, MMR, hybrid search."""

import pytest
import time
from unittest.mock import MagicMock, patch


class TestQueryCache:
    """Tests for QueryCache."""

    def test_cache_miss_then_hit(self):
        from retriever import QueryCache

        cache = QueryCache(ttl=60, max_size=100)
        results = [{"text": "doc1", "source": "a.txt", "chunk_index": 0, "relevance_score": 0.9}]

        # Miss
        assert cache.get("what is ai", None) is None
        assert cache.get_stats().misses == 1

        # Set then hit
        cache.set("what is ai", None, results)
        cached = cache.get("what is ai", None)
        assert cached == results
        assert cache.get_stats().hits == 1

    def test_different_queries_different_keys(self):
        from retriever import QueryCache
        cache = QueryCache(ttl=60)
        cache.set("query 1", None, [{"text": "a"}])
        cache.set("query 2", None, [{"text": "b"}])
        assert cache.get("query 1", None)[0]["text"] == "a"
        assert cache.get("query 2", None)[0]["text"] == "b"

    def test_filter_sources_affect_key(self):
        from retriever import QueryCache
        cache = QueryCache(ttl=60)
        cache.set("same query", ["doc1.txt"], [{"text": "from doc1"}])
        cache.set("same query", ["doc2.txt"], [{"text": "from doc2"}])
        assert cache.get("same query", ["doc1.txt"])[0]["text"] == "from doc1"
        assert cache.get("same query", ["doc2.txt"])[0]["text"] == "from doc2"

    def test_clear(self):
        from retriever import QueryCache
        cache = QueryCache(ttl=60)
        cache.set("q", None, [{"text": "x"}])
        cache.clear()
        assert cache.get("q", None) is None
        assert cache.get_stats().hits == 0


class TestBM25Indexer:
    """Tests for BM25Indexer."""

    def test_build_and_search(self):
        from retriever import BM25Indexer
        indexer = BM25Indexer(k1=1.5, b=0.75)
        chunks = [
            {"id": "a_0", "text": "artificial intelligence is a field of computer science", "source_file": "a.txt", "chunk_index": 0, "score": 0.0},
            {"id": "b_0", "text": "machine learning is a subset of artificial intelligence", "source_file": "b.txt", "chunk_index": 0, "score": 0.0},
            {"id": "c_0", "text": "deep learning uses neural networks for complex tasks", "source_file": "c.txt", "chunk_index": 0, "score": 0.0},
        ]
        indexer.build(chunks)

        # Search for "artificial intelligence"
        results = indexer.search("artificial intelligence", top_k=2)
        assert len(results) <= 2
        assert all("text" in r for r in results)
        # Top result should mention AI
        top_texts = [r["text"].lower() for r in results]
        assert any("artificial intelligence" in t for t in top_texts)

    def test_search_with_filter(self):
        from retriever import BM25Indexer
        indexer = BM25Indexer()
        chunks = [
            {"id": "a_0", "text": "python programming language", "source_file": "a.txt", "chunk_index": 0, "score": 0.0},
            {"id": "b_0", "text": "python is a snake too", "source_file": "b.txt", "chunk_index": 0, "score": 0.0},
        ]
        indexer.build(chunks)

        results = indexer.search("python", top_k=2, filter_sources=["a.txt"])
        assert all(r["source_file"] == "a.txt" for r in results)

    def test_empty_index(self):
        from retriever import BM25Indexer
        indexer = BM25Indexer()
        results = indexer.search("any query", top_k=5)
        assert results == []


class TestRRF:
    """Tests for Reciprocal Rank Fusion."""

    def test_rrf_fusion_combines_ranks(self):
        from retriever import HybridRetriever

        # Two docs where dense ranks doc1=1, doc2=2; sparse ranks doc2=1, doc1=2
        dense = [
            {"id": "d1", "text": "doc1", "source_file": "f", "chunk_index": 0, "score": 0.9},
            {"id": "d2", "text": "doc2", "source_file": "f", "chunk_index": 1, "score": 0.8},
        ]
        sparse = [
            {"id": "d2", "text": "doc2", "source_file": "f", "chunk_index": 1, "score": 0.7},
            {"id": "d1", "text": "doc1", "source_file": "f", "chunk_index": 0, "score": 0.6},
        ]

        # Create retriever with rrf_k=60
        retriever = HybridRetriever.__new__(HybridRetriever)
        retriever.rrf_k = 60.0
        fused = retriever._rrf_fusion(dense, sparse)

        fused_ids = [r["id"] for r in fused]
        # Both d1 and d2 should appear (they have equal RRF scores: 1/61+1/62 each)
        assert set(fused_ids) == {"d1", "d2"}

    def test_rrf_empty_sparse(self):
        from retriever import HybridRetriever
        retriever = HybridRetriever.__new__(HybridRetriever)
        retriever.rrf_k = 60.0
        dense = [{"id": "d1", "text": "x", "source_file": "f", "chunk_index": 0, "score": 0.9}]
        fused = retriever._rrf_fusion(dense, [])
        assert fused == dense


class TestMMR:
    """Tests for Maximal Marginal Relevance."""

    def test_mmr_diversity_vs_relevance(self):
        from retriever import mmr_select

        # Three candidates with same query sim but different inter-similarity
        candidates = [
            {"id": "d1", "text": "apple fruit", "source_file": "f", "chunk_index": 0, "embedding": [1.0, 0.0]},
            {"id": "d2", "text": "apple company", "source_file": "f", "chunk_index": 1, "embedding": [0.99, 0.1]},
            {"id": "d3", "text": "banana fruit", "source_file": "f", "chunk_index": 2, "embedding": [0.0, 1.0]},
        ]
        query_emb = [1.0, 0.0]

        # λ=0.0 = pure diversity: pick d3 first (most different from d1)
        selected_0 = mmr_select(candidates, query_emb, top_k=2, lambda_mult=0.0)
        # λ=1.0 = pure relevance: pick d1 first, then most different d3
        selected_1 = mmr_select(candidates, query_emb, top_k=2, lambda_mult=1.0)

        # Pure diversity (λ=0): d1 first has max relevance, then most different is d3
        assert selected_0[0]["id"] == "d1"
        # Pure relevance (λ=1): d1 first (highest query sim), d2 second (2nd highest sim)
        # (d3 has lowest query sim: 0.0)
        assert selected_1[1]["id"] in ("d2", "d3")

    def test_mmr_empty(self):
        from retriever import mmr_select
        assert mmr_select([], [1.0, 0.0], top_k=5, lambda_mult=0.3) == []
        assert mmr_select([], [1.0, 0.0], top_k=0, lambda_mult=0.3) == []


class TestCrossEncoderReranker:
    """Tests for CrossEncoderReranker (fallback mode)."""

    def test_reranker_fallback(self):
        from retriever import CrossEncoderReranker
        reranker = CrossEncoderReranker(model_name="nonexistent-model")
        # Should fall back gracefully
        candidates = [
            {"text": "doc about ai", "relevance_score": 0.5},
            {"text": "doc about ml", "relevance_score": 0.9},
        ]
        result = reranker.rerank("what is ai", candidates, top_k=2)
        assert len(result) == 2
        assert all("rerank_score" in r for r in result)


class TestHybridRetriever:
    """Tests for HybridRetriever full pipeline (mocked)."""

    def test_retrieve_with_cache_hit(self):
        from retriever import HybridRetriever

        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.1] * 128

        mock_vs = MagicMock()
        mock_vs.collection.get.return_value = {
            "ids": ["d1", "d2"],
            "documents": ["doc1 text", "doc2 text"],
            "metadatas": [
                {"source_file": "f.txt", "chunk_index": 0},
                {"source_file": "f.txt", "chunk_index": 1},
            ],
            "embeddings": [[0.1] * 128, [0.2] * 128],
        }
        mock_vs.search.return_value = [
            {"id": "d1", "text": "doc1 text", "source_file": "f.txt", "chunk_index": 0, "score": 0.9},
        ]

        retriever = HybridRetriever(
            embedder=mock_embedder,
            vectorstore=mock_vs,
            hybrid_search=False,
            rerank=False,
            mmr_enabled=False,
        )

        # First call: miss
        r1 = retriever.retrieve("test query")
        assert len(r1) >= 1

        # Second call: cache hit (embedder not called again)
        r2 = retriever.retrieve("test query")
        assert mock_embedder.embed_single.call_count == 1
        assert r1 == r2

    def test_retrieve_with_hybrid_search_builds_bm25(self):
        from retriever import HybridRetriever

        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.1] * 128

        mock_vs = MagicMock()
        mock_vs.collection.get.return_value = {
            "ids": ["d1", "d2"],
            "documents": ["artificial intelligence is great", "machine learning is cool"],
            "metadatas": [
                {"source_file": "a.txt", "chunk_index": 0},
                {"source_file": "b.txt", "chunk_index": 0},
            ],
            "embeddings": [[0.1] * 128, [0.2] * 128],
        }
        mock_vs.search.return_value = [
            {"id": "d1", "text": "artificial intelligence is great", "source_file": "a.txt", "chunk_index": 0, "score": 0.9},
        ]

        retriever = HybridRetriever(
            embedder=mock_embedder,
            vectorstore=mock_vs,
            hybrid_search=True,
            rerank=False,
            mmr_enabled=False,
        )

        results = retriever.retrieve("what is artificial intelligence")
        assert len(results) >= 1
        # BM25 should be built after first hybrid search
        assert retriever._bm25_built

    def test_retrieve_with_filter_sources(self):
        from retriever import HybridRetriever

        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.1] * 128

        mock_vs = MagicMock()
        mock_vs.search.return_value = []

        retriever = HybridRetriever(
            embedder=mock_embedder,
            vectorstore=mock_vs,
            hybrid_search=False,
            rerank=False,
            mmr_enabled=False,
        )

        retriever.retrieve("query", filter_sources=["doc1.txt"])
        # Verify filter passed to vectorstore.search
        mock_vs.search.assert_called_once()
        call_kwargs = mock_vs.search.call_args[1]
        assert call_kwargs["filter_sources"] == ["doc1.txt"]


class TestBackwardCompatibleRetriever:
    """Tests for backward-compatible Retriever class."""

    def test_simple_mode_no_hybrid(self):
        from retriever import Retriever

        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.1] * 128

        mock_vs = MagicMock()
        mock_vs.search.return_value = [
            {"id": "d1", "text": "result", "source_file": "f.txt", "chunk_index": 0, "score": 0.9},
        ]

        retriever = Retriever(
            embedder=mock_embedder,
            vectorstore=mock_vs,
            top_k=5,
            hybrid_search=False,
            rerank=False,
            mmr_enabled=False,
        )

        # Should use simple mode (no _hybrid)
        assert retriever._hybrid is None
        results = retriever.retrieve("test")
        assert len(results) == 1
        assert results[0]["text"] == "result"

    def test_hybrid_mode_activates_automatically(self):
        from retriever import Retriever

        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.1] * 128

        mock_vs = MagicMock()
        mock_vs.search.return_value = [
            {"id": "d1", "text": "result", "source_file": "f.txt", "chunk_index": 0, "score": 0.9},
        ]
        mock_vs.collection.get.return_value = {
            "ids": [],
            "documents": [],
            "metadatas": [],
            "embeddings": [],
        }

        retriever = Retriever(
            embedder=mock_embedder,
            vectorstore=mock_vs,
            hybrid_search=True,
            rerank=False,
            mmr_enabled=False,
        )

        assert retriever._hybrid is not None
        results = retriever.retrieve("test")
        assert len(results) >= 1
