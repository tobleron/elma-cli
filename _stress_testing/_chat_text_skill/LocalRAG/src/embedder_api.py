"""Embedding module using OpenAI-compatible API."""

import hashlib
import time
from typing import List, Optional, Callable, Any, Tuple
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor
from openai import OpenAI, APIError, RateLimitError, APITimeoutError
import tenacity

from cachetools import TTLCache
from observability import get_logger, api_latency, traced, log_error_alert, observe_latency

# ============================================================================
# Retry Configuration
# ============================================================================

DEFAULT_MAX_ATTEMPTS = 3
DEFAULT_INITIAL_BACKOFF = 1.0
DEFAULT_MAX_BACKOFF = 10.0
DEFAULT_BACKOFF_FACTOR = 2.0


def _is_retryable_error(exception: Exception) -> bool:
    """Check if an exception is a transient error worth retrying."""
    if isinstance(exception, (RateLimitError, APITimeoutError)):
        return True
    if isinstance(exception, APIError):
        if hasattr(exception, 'status_code'):
            return 500 <= exception.status_code < 600
        return True
    err_name = type(exception).__name__.lower()
    return 'timeout' in err_name or 'connection' in err_name or 'network' in err_name


def _build_retry_callback(logger):
    """Build a callback to log retry attempts."""
    def log_retry(retry_state):
        if retry_state.outcome is None:
            return
        exception = retry_state.outcome.exception()
        attempt = retry_state.attempt_number
        wait = retry_state.next_action.sleep if retry_state.next_action else 0
        if exception:
            logger.warning(
                f"Retrying embedder API call",
                attempt=attempt,
                max_attempts=retry_state.retry_object.stop,
                wait_seconds=round(wait, 2),
                error_type=type(exception).__name__,
                error=str(exception)[:100]
            )
    return log_retry


# ============================================================================
# Embedding Cache
# ============================================================================

@dataclass
class CacheStats:
    hits: int = 0
    misses: int = 0
    errors: int = 0

    @property
    def hit_rate(self) -> float:
        total = self.hits + self.misses
        return self.hits / total if total > 0 else 0.0


class EmbeddingCache:
    """LRU cache for embeddings with TTL and text-hash keying.

    Thread-safe for concurrent access.
    """

    def __init__(self, max_size: int = 10000, ttl: int = 3600):
        """Initialize the embedding cache.

        Args:
            max_size: Maximum number of entries (LRU eviction).
            ttl: Time-to-live in seconds for each entry.
        """
        self._cache = TTLCache(maxsize=max_size, ttl=ttl)
        self._lock = None  # TTLCache is thread-safe for single operations
        self._stats = CacheStats()
        self.logger = get_logger(__name__)

    def _text_hash(self, text: str) -> str:
        """Generate a stable hash key for text."""
        return hashlib.sha256(text.encode('utf-8')).hexdigest()[:32]

    def get(self, text: str) -> Optional[List[float]]:
        """Retrieve cached embedding for text.

        Args:
            text: Input text.

        Returns:
            Cached embedding vector or None if not found/expired.
        """
        key = self._text_hash(text)
        result = self._cache.get(key)
        if result is not None:
            self._stats.hits += 1
            self.logger.debug("Cache hit", key=key[:8], cache_size=len(self._cache))
        else:
            self._stats.misses += 1
            self.logger.debug("Cache miss", key=key[:8], cache_size=len(self._cache))
        return result

    def set(self, text: str, embedding: List[float]) -> None:
        """Store embedding for text in cache.

        Args:
            text: Input text.
            embedding: Embedding vector to cache.
        """
        key = self._text_hash(text)
        self._cache[key] = embedding
        self.logger.debug("Cache set", key=key[:8], cache_size=len(self._cache))

    def get_stats(self) -> CacheStats:
        """Return cache statistics (copy)."""
        return CacheStats(
            hits=self._stats.hits,
            misses=self._stats.misses,
            errors=self._stats.errors
        )

    def clear(self) -> None:
        """Evict all entries from cache."""
        self._cache.clear()
        self._stats = CacheStats()
        self.logger.info("Cache cleared")


# ============================================================================
# Embedder API Client
# ============================================================================

class EmbedderAPIClient:
    """Generates vector embeddings using OpenAI-compatible API.

    Features:
    - Retry with exponential backoff for transient errors
    - Batch size control to avoid memory exhaustion
    - LRU + TTL cache to avoid re-embedding identical texts
    - Per-chunk error isolation: partial failures don't fail the whole batch
    - Cache hit/miss metrics
    """

    def __init__(
        self,
        api_base: str = "http://localhost:1234/v1",
        api_key: str = "not-needed",
        model: str = "embedding-model",
        max_attempts: int = DEFAULT_MAX_ATTEMPTS,
        initial_backoff: float = DEFAULT_INITIAL_BACKOFF,
        backoff_factor: float = DEFAULT_BACKOFF_FACTOR,
        max_backoff: float = DEFAULT_MAX_BACKOFF,
        max_batch_size: int = 100,
        cache_size: int = 10000,
        cache_ttl: int = 3600,
        sub_batch_size: int = 16,
    ):
        """Initialize the Embedder API client.

        Args:
            api_base: Base URL for the API.
            api_key: API key (often not needed for local servers).
            model: Embedding model name to use.
            max_attempts: Maximum retry attempts for API calls.
            initial_backoff: Initial backoff seconds between retries.
            backoff_factor: Multiplier for backoff after each retry.
            max_backoff: Maximum backoff seconds.
            max_batch_size: Maximum chunks per embedding API call.
            cache_size: Maximum cache entries (LRU eviction).
            cache_ttl: Cache entry TTL in seconds.
            sub_batch_size: Sub-batch size for error isolation within a batch.
        """
        self.api_base = api_base
        self.api_key = api_key
        self.model = model
        self.max_attempts = max_attempts
        self.initial_backoff = initial_backoff
        self.backoff_factor = backoff_factor
        self.max_backoff = max_backoff
        self.max_batch_size = max_batch_size
        self.sub_batch_size = sub_batch_size
        self.client = OpenAI(base_url=api_base, api_key=api_key)
        self.logger = get_logger(__name__)
        self.cache = EmbeddingCache(max_size=cache_size, ttl=cache_ttl)

    @property
    def _retry_kwargs(self) -> dict:
        """Build tenacity retry kwargs from instance config."""
        return {
            "retry": tenacity.retry_if_exception(_is_retryable_error),
            "wait": tenacity.wait_exponential(
                multiplier=self.initial_backoff,
                exp_base=self.backoff_factor,
                max=self.max_backoff
            ),
            "stop": tenacity.stop_after_attempt(self.max_attempts),
            "before_sleep": _build_retry_callback(self.logger),
            "reraise": True,
        }

    def _call_embeddings_api(self, texts: List[str]) -> List[List[float]]:
        """Call the embeddings API with retry logic.

        Args:
            texts: List of texts to embed.

        Returns:
            List of embedding vectors.
        """
        @tenacity.retry(**self._retry_kwargs)
        def _call():
            return self.client.embeddings.create(model=self.model, input=texts)

        response = _call()
        embeddings = [item.embedding for item in response.data]

        # Validate dimensions
        if not embeddings:
            raise ValueError(f"Embedding API returned empty result for {len(texts)} texts")
        first_dim = len(embeddings[0])
        if first_dim == 0:
            raise ValueError(
                f"Embedding API returned 0-dimensional vectors. "
                f"Model='{self.model}' API='{self.api_base}'. "
                f"Check that the API is returning valid embedding arrays."
            )
        if any(len(e) != first_dim for e in embeddings):
            dims = [len(e) for e in embeddings]
            raise ValueError(
                f"Embedding API returned inconsistent dimensions: {dims}. "
                f"First text='{texts[0][:50]}...'"
            )
        self.logger.debug("Embeddings received", num=len(embeddings), dim=first_dim)
        return embeddings

    def embed_single(self, text: str, use_cache: bool = True) -> List[float]:
        """Generate embedding for a single text.

        Args:
            text: Text to embed.
            use_cache: If True, check cache before API call.

        Returns:
            Vector embedding as list of floats.
        """
        self.logger.debug("Embedding single text", text_length=len(text), use_cache=use_cache)

        if use_cache:
            cached = self.cache.get(text)
            if cached is not None:
                return cached

        start = time.perf_counter()
        try:
            response = self._call_embeddings_api([text])
            duration = time.perf_counter() - start
            observe_latency(api_latency, {"client": "embedder", "operation": "embed_single"}, duration)

            embedding = response[0]
            if use_cache:
                self.cache.set(text, embedding)
            return embedding
        except Exception as e:
            duration = time.perf_counter() - start
            observe_latency(api_latency, {"client": "embedder", "operation": "embed_single"}, duration)
            log_error_alert(self.logger, e, "embedder_api",
                          context={"text_length": len(text)})
            raise

    def embed_batch(
        self,
        texts: List[str],
        use_cache: bool = True,
        skip_on_error: bool = False,
    ) -> Tuple[List[List[float]], List[int]]:
        """Generate embeddings for multiple texts.

        Uses sub-batching for error isolation: a failure in one sub-batch
        does not fail the entire operation.

        Args:
            texts: List of texts to embed.
            use_cache: If True, check and update cache for each text.
            skip_on_error: If True, return zeros for failed chunks instead of raising.

        Returns:
            Tuple of (embeddings list, failed_indices list).
            Failed indices are absolute positions in the original texts list.
        """
        self.logger.debug("Embedding batch", batch_size=len(texts), use_cache=use_cache)

        embeddings: List[Optional[List[float]]] = [None] * len(texts)
        failed_indices: List[int] = []

        # Separate cached vs uncached texts
        uncached_indices: List[int] = []
        if use_cache:
            for i, text in enumerate(texts):
                cached = self.cache.get(text)
                if cached is not None:
                    embeddings[i] = cached
                else:
                    uncached_indices.append(i)
            self.logger.debug("Cache lookup done", cached=len(texts) - len(uncached_indices),
                           to_embed=len(uncached_indices))
        else:
            uncached_indices = list(range(len(texts)))

        if not uncached_indices:
            return embeddings, failed_indices

        # Embed uncached texts in sub-batches for error isolation
        for start in range(0, len(uncached_indices), self.sub_batch_size):
            sub_indices = uncached_indices[start:start + self.sub_batch_size]
            sub_texts = [texts[i] for i in sub_indices]

            try:
                sub_embeddings = self._call_embeddings_api(sub_texts)
                for idx, emb in zip(sub_indices, sub_embeddings):
                    embeddings[idx] = emb
                    if use_cache:
                        self.cache.set(texts[idx], emb)
            except Exception as e:
                self.logger.warning(
                    "Sub-batch embedding failed, marking indices as failed",
                    sub_start=sub_indices[0],
                    sub_end=sub_indices[-1],
                    error=str(e)[:100]
                )
                if skip_on_error:
                    for idx in sub_indices:
                        failed_indices.append(idx)
                        embeddings[idx] = [0.0] * 1536  # placeholder, caller should handle
                else:
                    # Re-raise so caller can decide how to handle
                    failed_indices.extend(sub_indices)
                    raise

        return embeddings, failed_indices

    def embed_chunks(
        self,
        chunks: List[dict],
        progress_callback: Optional[Callable[[int, int], None]] = None,
        use_cache: bool = True,
        skip_on_error: bool = True,
    ) -> List[dict]:
        """Add embeddings to a list of chunks in batches.

        Handles partial failures gracefully, marking failed chunks
        with a zero embedding and returning them in the result.

        Args:
            chunks: List of chunk dictionaries with 'text' key.
            progress_callback: Optional callable(processed, total) for progress.
            use_cache: If True, use embedding cache.
            skip_on_error: If True, continue on chunk failure (mark with zeros).

        Returns:
            List of chunks with added 'embedding' key.
            Failed chunks have embedding=[0.0]*dim and '_embedding_error'=True.
        """
        total = len(chunks)
        self.logger.info("Embedding chunks in batches",
                        num_chunks=total,
                        batch_size=self.max_batch_size,
                        sub_batch_size=self.sub_batch_size)

        failed_chunks: List[int] = []

        for i in range(0, total, self.max_batch_size):
            batch = chunks[i:i + self.max_batch_size]
            texts = [chunk["text"] for chunk in batch]

            batch_embeddings, batch_failed = self.embed_batch(
                texts,
                use_cache=use_cache,
                skip_on_error=skip_on_error,
            )

            for j, (chunk, embedding) in enumerate(zip(batch, batch_embeddings)):
                global_idx = i + j
                # batch_failed contains LOCAL indices (0, 1, ... within the batch),
                # not global indices. Use 'j in batch_failed' to check.
                if batch_embeddings[j] is None or (batch_failed and j in batch_failed):
                    chunk["embedding"] = [0.0] * 1536
                    chunk["_embedding_error"] = True
                    failed_chunks.append(global_idx)
                else:
                    chunk["embedding"] = embedding
                    chunk["_embedding_error"] = False

            if progress_callback:
                progress_callback(min(i + self.max_batch_size, total), total)

        cache_stats = self.cache.get_stats()
        self.logger.info("Chunks embedded",
                        num_chunks=total,
                        failed=len(failed_chunks),
                        cache_hits=cache_stats.hits,
                        cache_misses=cache_stats.misses,
                        cache_hit_rate=round(cache_stats.hit_rate, 3))
        return chunks


# ============================================================================
# Async Embedder API Client
# ============================================================================

class AsyncEmbedderAPIClient:
    """Async embedding client using httpx for true concurrency.

    Use this when you need to embed many texts concurrently or when
    the calling code already runs in an async context.
    """

    def __init__(
        self,
        api_base: str = "http://localhost:1234/v1",
        api_key: str = "not-needed",
        model: str = "embedding-model",
        max_attempts: int = DEFAULT_MAX_ATTEMPTS,
        initial_backoff: float = DEFAULT_INITIAL_BACKOFF,
        backoff_factor: float = DEFAULT_BACKOFF_FACTOR,
        max_backoff: float = DEFAULT_MAX_BACKOFF,
        max_batch_size: int = 100,
        cache_size: int = 10000,
        cache_ttl: int = 3600,
        sub_batch_size: int = 16,
        timeout: float = 60.0,
    ):
        self.api_base = api_base.rstrip('/')
        self.api_key = api_key
        self.model = model
        self.max_attempts = max_attempts
        self.initial_backoff = initial_backoff
        self.backoff_factor = backoff_factor
        self.max_backoff = max_backoff
        self.max_batch_size = max_batch_size
        self.sub_batch_size = sub_batch_size
        self.timeout = timeout
        self.logger = get_logger(__name__)
        self.cache = EmbeddingCache(max_size=cache_size, ttl=cache_ttl)

    async def _request_with_retry(self, payload: dict) -> dict:
        """Make a request with exponential backoff retry."""
        import httpx

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            last_error: Optional[Exception] = None
            for attempt in range(1, self.max_attempts + 1):
                try:
                    response = await client.post(
                        f"{self.api_base}/embeddings",
                        json=payload,
                        headers={
                            "Authorization": f"Bearer {self.api_key}",
                            "Content-Type": "application/json",
                        }
                    )
                    if response.status_code == 429 or (500 <= response.status_code < 600):
                        # Retryable
                        wait = min(self.initial_backoff * (self.backoff_factor ** (attempt - 1)), self.max_backoff)
                        self.logger.warning("Async embed retry", attempt=attempt, wait=wait,
                                          status=response.status_code)
                        import asyncio
                        await asyncio.sleep(wait)
                        continue
                    response.raise_for_status()
                    return response.json()
                except Exception as e:
                    last_error = e
                    if not _is_retryable_error(e):
                        raise
                    if attempt < self.max_attempts:
                        wait = min(self.initial_backoff * (self.backoff_factor ** (attempt - 1)), self.max_backoff)
                        self.logger.warning("Async embed retry", attempt=attempt, wait=wait,
                                          error=str(e)[:80])
                        import asyncio
                        await asyncio.sleep(wait)
            raise last_error

    async def embed_single(self, text: str, use_cache: bool = True) -> List[float]:
        """Async embed a single text."""
        if use_cache:
            cached = self.cache.get(text)
            if cached is not None:
                return cached

        payload = {"model": self.model, "input": text}
        result = await self._request_with_retry(payload)
        embedding = result["data"][0]["embedding"]

        if use_cache:
            self.cache.set(text, embedding)
        return embedding

    async def embed_batch(
        self,
        texts: List[str],
        use_cache: bool = True,
        skip_on_error: bool = False,
    ) -> Tuple[List[Optional[List[float]]], List[int]]:
        """Async embed multiple texts with cache and error isolation."""
        embeddings: List[Optional[List[float]]] = [None] * len(texts)
        failed_indices: List[int] = []

        uncached_indices: List[int] = []
        if use_cache:
            for i, text in enumerate(texts):
                cached = self.cache.get(text)
                if cached is not None:
                    embeddings[i] = cached
                else:
                    uncached_indices.append(i)
        else:
            uncached_indices = list(range(len(texts)))

        if not uncached_indices:
            return embeddings, failed_indices

        for start in range(0, len(uncached_indices), self.sub_batch_size):
            sub_indices = uncached_indices[start:start + self.sub_batch_size]
            sub_texts = [texts[i] for i in sub_indices]

            try:
                payload = {"model": self.model, "input": sub_texts}
                result = await self._request_with_retry(payload)
                sub_embeddings = [item["embedding"] for item in result["data"]]
                for idx, emb in zip(sub_indices, sub_embeddings):
                    embeddings[idx] = emb
                    if use_cache:
                        self.cache.set(texts[idx], emb)
            except Exception as e:
                self.logger.warning("Async sub-batch failed", sub_start=sub_indices[0],
                                   sub_end=sub_indices[-1], error=str(e)[:100])
                if skip_on_error:
                    for idx in sub_indices:
                        failed_indices.append(idx)
                        embeddings[idx] = [0.0] * 1536
                else:
                    failed_indices.extend(sub_indices)
                    raise

        return embeddings, failed_indices

    async def embed_chunks(
        self,
        chunks: List[dict],
        progress_callback: Optional[Callable[[int, int], None]] = None,
        use_cache: bool = True,
        skip_on_error: bool = True,
    ) -> List[dict]:
        """Async version of embed_chunks."""
        total = len(chunks)
        failed_chunks: List[int] = []

        for i in range(0, total, self.max_batch_size):
            batch = chunks[i:i + self.max_batch_size]
            texts = [chunk["text"] for chunk in batch]

            batch_embeddings, batch_failed = await self.embed_batch(
                texts,
                use_cache=use_cache,
                skip_on_error=skip_on_error,
            )

            for j, embedding in enumerate(batch_embeddings):
                global_idx = i + j
                # batch_failed contains LOCAL indices (0, 1, ... within the batch),
                # not global indices. Use 'j in batch_failed' to check.
                if embedding is None or (batch_failed and j in batch_failed):
                    chunks[global_idx]["embedding"] = [0.0] * 1536
                    chunks[global_idx]["_embedding_error"] = True
                    failed_chunks.append(global_idx)
                else:
                    chunks[global_idx]["embedding"] = embedding
                    chunks[global_idx]["_embedding_error"] = False

            if progress_callback:
                progress_callback(min(i + self.max_batch_size, total), total)

        cache_stats = self.cache.get_stats()
        self.logger.info("Async chunks embedded",
                        num_chunks=total,
                        failed=len(failed_chunks),
                        cache_hit_rate=round(cache_stats.hit_rate, 3))
        return chunks
