"""Tests for embedder_api module."""

import pytest
from unittest.mock import patch, MagicMock, AsyncMock

from src.embedder_api import (
    EmbeddingCache,
    EmbedderAPIClient,
    AsyncEmbedderAPIClient,
)


# ==============================================================================
# EmbeddingCache Tests
# ==============================================================================

class TestEmbeddingCache:
    def test_set_and_get(self):
        cache = EmbeddingCache(max_size=100, ttl=60)
        cache.set("hello", [0.1, 0.2, 0.3])
        assert cache.get("hello") == [0.1, 0.2, 0.3]

    def test_cache_miss(self):
        cache = EmbeddingCache(max_size=100, ttl=60)
        assert cache.get("nonexistent") is None

    def test_stats(self):
        cache = EmbeddingCache(max_size=100, ttl=60)
        cache.set("text1", [0.1])
        cache.get("text1")
        cache.get("text2")
        stats = cache.get_stats()
        assert stats.hits == 1
        assert stats.misses == 1
        assert stats.hit_rate == 0.5

    def test_lru_eviction(self):
        cache = EmbeddingCache(max_size=2, ttl=60)
        cache.set("a", [1.0])
        cache.set("b", [2.0])
        cache.set("c", [3.0])
        assert cache.get("a") is None
        assert cache.get("c") == [3.0]

    def test_clear(self):
        cache = EmbeddingCache(max_size=100, ttl=60)
        cache.set("text1", [0.1])
        cache.clear()
        stats = cache.get_stats()
        assert stats.hits == 0 and stats.misses == 0

    def test_overwrite(self):
        cache = EmbeddingCache(max_size=100, ttl=60)
        cache.set("hello", [1.0])
        cache.set("hello", [2.0])
        assert cache.get("hello") == [2.0]


# ==============================================================================
# EmbedderAPIClient Tests
#
# Strategy: Patch _call_embeddings_api (bypasses tenacity's retry decorator).
# The mock returns raw embedding lists directly.
# ==============================================================================

def _make_mock_call(text_to_embs):
    """Build a side_effect for _call_embeddings_api that maps texts to embeddings.

    Args:
        text_to_embs: dict mapping frozenset of texts -> list of embedding vectors.
    Returns:
        A side_effect function suitable for patching _call_embeddings_api.
    """
    def mock_call(texts):
        key = frozenset(texts)
        if key not in text_to_embs:
            raise RuntimeError(f"No mock for: {texts}")
        return text_to_embs[key]
    return mock_call


class TestEmbedderAPIClient:
    def _client(self, **kwargs):
        client = EmbedderAPIClient(**kwargs)
        client.cache = EmbeddingCache(max_size=100, ttl=60)
        return client

    def test_embed_single_no_cache_hit(self):
        client = self._client(max_attempts=1)
        side_effect = _make_mock_call({
            frozenset(["hello"]): [[0.1] * 1536],
        })
        with patch.object(client, '_call_embeddings_api', side_effect=side_effect):
            result = client.embed_single("hello", use_cache=False)
        assert result == [0.1] * 1536

    def test_embed_single_cache_hit_skips_api(self):
        client = self._client()
        client.cache.set("hello", [0.9] * 1536)
        with patch.object(client, '_call_embeddings_api') as mock_api:
            result = client.embed_single("hello", use_cache=True)
        assert result == [0.9] * 1536
        mock_api.assert_not_called()

    def test_embed_single_caches_after_api_call(self):
        client = self._client(max_attempts=1)
        side_effect = _make_mock_call({
            frozenset(["hello"]): [[0.5] * 1536],
        })
        with patch.object(client, '_call_embeddings_api', side_effect=side_effect):
            result = client.embed_single("hello", use_cache=True)
        assert result == [0.5] * 1536
        assert client.cache.get("hello") == [0.5] * 1536

    def test_embed_batch_all_uncached(self):
        client = self._client(max_attempts=1)
        side_effect = _make_mock_call({
            frozenset(["t1", "t2"]): [[0.1] * 1536, [0.2] * 1536],
        })
        with patch.object(client, '_call_embeddings_api', side_effect=side_effect):
            embeddings, failed = client.embed_batch(["t1", "t2"], use_cache=False)
        assert embeddings[0] == [0.1] * 1536
        assert embeddings[1] == [0.2] * 1536
        assert failed == []

    def test_embed_batch_all_cached(self):
        client = self._client()
        client.cache.set("t1", [0.9] * 1536)
        client.cache.set("t2", [0.8] * 1536)
        with patch.object(client, '_call_embeddings_api') as mock_api:
            embeddings, failed = client.embed_batch(["t1", "t2"], use_cache=True)
        assert embeddings[0] == [0.9] * 1536
        assert embeddings[1] == [0.8] * 1536
        mock_api.assert_not_called()

    def test_embed_batch_partial_cache(self):
        client = self._client(max_attempts=1)
        client.cache.set("t1", [0.9] * 1536)
        side_effect = _make_mock_call({
            frozenset(["t2"]): [[0.2] * 1536],
        })
        with patch.object(client, '_call_embeddings_api', side_effect=side_effect):
            embeddings, failed = client.embed_batch(["t1", "t2"], use_cache=True)
        assert embeddings[0] == [0.9] * 1536
        assert embeddings[1] == [0.2] * 1536
        assert failed == []

    def test_embed_batch_error_raises_when_not_skipping(self):
        client = self._client(max_attempts=1)
        with patch.object(client, '_call_embeddings_api', side_effect=Exception("API failure")):
            with pytest.raises(Exception, match="API failure"):
                client.embed_batch(["t1"], use_cache=False, skip_on_error=False)

    def test_embed_batch_empty_input(self):
        client = self._client()
        embeddings, failed = client.embed_batch([], use_cache=False)
        assert embeddings == []
        assert failed == []

    def test_embed_chunks_progress_callback(self):
        client = self._client(max_attempts=1)
        client.max_batch_size = 2
        chunks = [{"text": f"t{i}"} for i in range(4)]
        calls = []

        def side_effect(texts):
            return [[0.1] * 1536] * len(texts)

        with patch.object(client, '_call_embeddings_api', side_effect=side_effect):
            client.embed_chunks(
                chunks,
                use_cache=False,
                progress_callback=lambda done, total: calls.append((done, total)),
            )

        assert calls == [(2, 4), (4, 4)]

    def test_embed_chunks_partial_failure_marks_error_flag(self):
        client = self._client(max_attempts=1)
        client.max_batch_size = 4
        client.sub_batch_size = 2
        chunks = [{"text": f"t{i}"} for i in range(5)]

        def side_effect(texts):
            key = frozenset(texts)
            if key == frozenset(["t0", "t1"]):
                return [[0.1] * 1536, [0.2] * 1536]
            if key == frozenset(["t2", "t3"]):
                return [[0.3] * 1536, [0.4] * 1536]
            if key == frozenset(["t4"]):
                raise Exception("t4 failed")
            return [[0.0] * 1536] * len(texts)

        with patch.object(client, '_call_embeddings_api', side_effect=side_effect):
            result = client.embed_chunks(chunks, use_cache=False, skip_on_error=True)

        assert result[0]["embedding"] == [0.1] * 1536
        assert result[0]["_embedding_error"] is False
        assert result[4]["embedding"] == [0.0] * 1536
        assert result[4]["_embedding_error"] is True


# ==============================================================================
# AsyncEmbedderAPIClient Tests
# ==============================================================================

class TestAsyncEmbedderAPIClient:
    @pytest.mark.asyncio
    async def test_async_embed_single_cache_hit(self):
        client = AsyncEmbedderAPIClient()
        client.cache.set("hello", [0.7] * 1536)

        with patch.object(client, '_request_with_retry', new_callable=AsyncMock) as mock_req:
            result = await client.embed_single("hello", use_cache=True)
            assert result == [0.7] * 1536
            mock_req.assert_not_called()

    @pytest.mark.asyncio
    async def test_async_embed_single_cache_miss(self):
        client = AsyncEmbedderAPIClient()
        client.cache = EmbeddingCache(max_size=100, ttl=60)

        async def mock_request(payload):
            return {"data": [{"embedding": [0.5] * 1536}]}

        with patch.object(client, '_request_with_retry', new_callable=AsyncMock, side_effect=mock_request):
            result = await client.embed_single("hello", use_cache=True)

        assert result == [0.5] * 1536
        assert client.cache.get("hello") == [0.5] * 1536

    @pytest.mark.asyncio
    async def test_async_embed_batch(self):
        client = AsyncEmbedderAPIClient()
        client.sub_batch_size = 2

        async def mock_request(payload):
            texts = payload["input"]
            return {"data": [{"embedding": [0.1] * 1536} for _ in texts]}

        with patch.object(client, '_request_with_retry', new_callable=AsyncMock, side_effect=mock_request):
            embeddings, failed = await client.embed_batch(["t1", "t2"], use_cache=False)

        assert embeddings[0] == [0.1] * 1536
        assert embeddings[1] == [0.1] * 1536
        assert failed == []

    @pytest.mark.asyncio
    async def test_async_embed_batch_error_isolation(self):
        client = AsyncEmbedderAPIClient()
        client.sub_batch_size = 1

        async def mock_request(payload):
            if payload["input"] == ["fail"]:
                raise Exception("simulated failure")
            return {"data": [{"embedding": [0.2] * 1536}]}

        with patch.object(client, '_request_with_retry', new_callable=AsyncMock, side_effect=mock_request):
            embeddings, failed = await client.embed_batch(
                ["ok", "fail"],
                use_cache=False,
                skip_on_error=True,
            )

        assert embeddings[0] == [0.2] * 1536
        assert embeddings[1] is not None  # placeholder
        assert 1 in failed
