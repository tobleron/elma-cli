"""RAG Pipeline module for Local RAG."""

from pathlib import Path
from typing import List, Dict, Any, Optional, Union, Callable
import yaml
import time
from contextlib import contextmanager

from loader import DocumentLoader, ContentValidator, DocumentValidationError
from chunker import TextChunker
from vectorstore import VectorStore
from retriever import Retriever
from observability import (
    get_logger, request_context,
    rag_query_latency, ingest_latency, ingest_chunks,
    ingest_total, rag_query_total, active_rag_requests,
    observe_latency, increment_counter
)


class RAGPipeline:
    """Orchestrates the complete RAG flow from document to answer."""

    def __init__(
        self,
        config_path: str = "config/config.yaml",
        use_api: bool = False,
        chunker: Optional[TextChunker] = None
    ):
        """Initialize the RAG pipeline with configuration.

        Args:
            config_path: Path to YAML configuration file.
            use_api: If True, expect API clients to be passed in query().
            chunker: Custom chunker instance. If None, uses default from config.
        """
        self.logger = get_logger(__name__)
        self.config = self._load_config(config_path)
        self.use_api = use_api

        self.loader = DocumentLoader()

        # Use provided chunker or create from config
        if chunker is not None:
            self.chunker = chunker
        else:
            strategy = self.config["chunking"].get("default_strategy", "fixed")
            strategy_params = self.config["chunking"]["strategies"].get(strategy, {})
            self.chunker = TextChunker(
                chunk_size=self.config["chunking"]["chunk_size"],
                chunk_overlap=self.config["chunking"]["chunk_overlap"],
                strategy=strategy,
                **strategy_params
            )

        self.vectorstore = VectorStore(
            persist_directory=self.config["vectorstore"]["persist_directory"]
        )

    def _load_config(self, config_path: str) -> Dict[str, Any]:
        """Load configuration from YAML file.

        Args:
            config_path: Path to config file.

        Returns:
            Configuration dictionary.
        """
        default_config = {
            "chunking": {
                "chunk_size": 512,
                "chunk_overlap": 50,
                "default_strategy": "fixed",
                "strategies": {}
            },
            "retrieval": {
                "top_k": 5,
                "hybrid_search": False,
                "dense_top_k": 20,
                "sparse_top_k": 20,
                "rrf_k": 60,
                "rerank": False,
                "rerank_top_k": 20,
                "rerank_api_base": "",
                "rerank_api_key": "not-needed",
                "rerank_model": "",
                "mmr_enabled": False,
                "mmr_lambda": 0.3,
                "query_cache_ttl": 300,
                "query_cache_size": 1000,
            },
            "vectorstore": {
                "persist_directory": "data/chroma_db"
            }
        }

        if not Path(config_path).exists():
            self.logger.warning(f"Config file not found: {config_path}, using defaults")
            return default_config

        with open(config_path, "r") as f:
            user_config = yaml.safe_load(f)

        for section, values in default_config.items():
            if section not in user_config:
                user_config[section] = values
            else:
                for key, value in values.items():
                    if key not in user_config[section]:
                        user_config[section][key] = value

        return user_config

    @contextmanager
    def _track_phase(self, phase: str):
        """Context manager for phase timing."""
        phase_start = time.perf_counter()
        try:
            yield
        finally:
            duration = time.perf_counter() - phase_start
            observe_latency(ingest_latency, {"phase": phase}, duration)
            self.logger.debug(f"Phase '{phase}' completed",
                            duration_ms=round(duration * 1000, 2))

    def ingest_document(
        self,
        file_path: Union[str, Path],
        embedder=None,
        progress_callback: Optional[Callable[[str, int, int], None]] = None
    ) -> int:
        """Ingest a document into the vector store.

        Flow: load → chunk → embed → store

        Args:
            file_path: Path to document file.
            embedder: Optional embedder instance (required for API mode).
            progress_callback: Optional callable(phase, current, total) for progress.
                phase: one of "load", "chunk", "embed", "store"
                current: items processed so far (0 when phase starts)
                total: total items in this phase (0 when unknown)

        Returns:
            Number of chunks successfully indexed.
        """
        def _report(phase: str, current: int, total: int):
            if progress_callback:
                progress_callback(phase, current, total)

        with request_context(operation="ingest", source_file=str(file_path)) as ctx:
            request_id = ctx["request_id"]
            self.logger.info("Starting document ingestion",
                           source_file=str(file_path))

            start_total = time.perf_counter()

            try:
                with self._track_phase("load"):
                    self.logger.info("Loading document")
                    _report("load", 0, 1)
                    doc_data = self.loader.load_with_metadata(file_path)
                    text = doc_data["content"]
                    source = doc_data["source"]
                    _report("load", 1, 1)

                with self._track_phase("chunk"):
                    self.logger.info(f"Chunking text ({len(text)} chars) using {self.chunker.strategy} strategy")
                    _report("chunk", 0, 1)
                    chunks = self.chunker.chunk_with_metadata(text, source)
                    for i, chunk in enumerate(chunks):
                        ContentValidator.validate_chunk(chunk["text"], i, source)
                    _report("chunk", len(chunks), len(chunks))

                with self._track_phase("embed"):
                    self.logger.info(f"Generating embeddings for {len(chunks)} chunks")
                    _report("embed", 0, len(chunks))
                    chunks_with_embeddings = embedder.embed_chunks(
                        chunks,
                        progress_callback=lambda done, total: _report("embed", done, total)
                    )
                    _report("embed", len(chunks), len(chunks))

                with self._track_phase("store"):
                    self.logger.info("Upserting document in vector database")
                    _report("store", 0, len(chunks_with_embeddings))
                    inserted, updated, deleted = self.vectorstore.upsert_document(
                        chunks_with_embeddings, embedder
                    )
                    ids_count = inserted + updated
                    _report("store", ids_count, ids_count)
                    self.logger.info("Upsert completed",
                                   inserted=inserted, updated=updated, deleted=deleted)

                total_duration = time.perf_counter() - start_total
                observe_latency(ingest_latency, {"phase": "total"}, total_duration)
                ingest_chunks.labels(phase="total").observe(total_duration)
                increment_counter(ingest_total, {"status": "success"})

                self.logger.info("Document ingestion completed",
                               chunks_indexed=ids_count,
                               duration_ms=round(total_duration * 1000, 2))

                return ids_count

            except Exception as e:
                total_duration = time.perf_counter() - start_total
                observe_latency(ingest_latency, {"phase": "total"}, total_duration)
                increment_counter(ingest_total, {"status": "error"})
                self.logger.error("Document ingestion failed",
                                error=str(e),
                                duration_ms=round(total_duration * 1000, 2))
                raise

    def query(
        self,
        question: str,
        filter_sources: Optional[List[str]] = None,
        llm_client=None,
        embedder_client=None
    ) -> Dict[str, Any]:
        """Query the RAG system with a question.

        Flow: embed question → retrieve → generate answer

        Args:
            question: User question.
            filter_sources: If provided, only search in these documents (list).
            llm_client: LLM API client (required for API mode).
            embedder_client: Embedder API client (required for API mode).

        Returns:
            Dictionary with answer and source context.
        """
        with request_context(operation="query") as ctx:
            request_id = ctx["request_id"]
            self.logger.info("Processing RAG query", question=question[:100])

            active_rag_requests.inc()
            start_total = time.perf_counter()

            try:
                start_retrieve = time.perf_counter()
                self.logger.info("Retrieving relevant context")

                retriever = Retriever(
                    embedder=embedder_client,
                    vectorstore=self.vectorstore,
                    top_k=self.config["retrieval"]["top_k"],
                    hybrid_search=self.config["retrieval"].get("hybrid_search", False),
                    dense_top_k=self.config["retrieval"].get("dense_top_k", 20),
                    sparse_top_k=self.config["retrieval"].get("sparse_top_k", 20),
                    rrf_k=float(self.config["retrieval"].get("rrf_k", 60)),
                    rerank=self.config["retrieval"].get("rerank", False),
                    rerank_top_k=self.config["retrieval"].get("rerank_top_k", 20),
                    rerank_api_base=self.config["retrieval"].get("rerank_api_base", ""),
                    rerank_api_key=self.config["retrieval"].get("rerank_api_key", "not-needed"),
                    rerank_model=self.config["retrieval"].get("rerank_model", ""),
                    mmr_enabled=self.config["retrieval"].get("mmr_enabled", False),
                    mmr_lambda=float(self.config["retrieval"].get("mmr_lambda", 0.3)),
                    query_cache_ttl=self.config["retrieval"].get("query_cache_ttl", 300),
                    query_cache_size=self.config["retrieval"].get("query_cache_size", 1000),
                )
                context = retriever.retrieve(question, filter_sources=filter_sources)

                retrieve_duration = time.perf_counter() - start_retrieve
                observe_latency(rag_query_latency, {"phase": "retrieve"}, retrieve_duration)
                self.logger.info("Retrieval completed",
                               num_results=len(context),
                               duration_ms=round(retrieve_duration * 1000, 2))

                start_generate = time.perf_counter()
                self.logger.info("Generating answer")

                gen_result = llm_client.generate_with_reasoning(question, context)

                generate_duration = time.perf_counter() - start_generate
                observe_latency(rag_query_latency, {"phase": "generate"}, generate_duration)

                total_duration = time.perf_counter() - start_total
                observe_latency(rag_query_latency, {"phase": "total"}, total_duration)
                increment_counter(rag_query_total, {"status": "success"})

                self.logger.info("Query completed",
                               num_sources=len(context),
                               total_duration_ms=round(total_duration * 1000, 2),
                               retrieve_duration_ms=round(retrieve_duration * 1000, 2),
                               generate_duration_ms=round(generate_duration * 1000, 2))

                active_rag_requests.dec()
                return {
                    "answer": gen_result.answer,
                    "reasoning": gen_result.reasoning,
                    "cited_sources": gen_result.cited_sources,
                    "confidence": gen_result.confidence,
                    "sources": context,
                    "num_sources": len(context)
                }

            except Exception as e:
                total_duration = time.perf_counter() - start_total
                observe_latency(rag_query_latency, {"phase": "total"}, total_duration)
                increment_counter(rag_query_total, {"status": "error"})
                self.logger.error("Query failed",
                                error=str(e),
                                duration_ms=round(total_duration * 1000, 2))
                active_rag_requests.dec()
                raise