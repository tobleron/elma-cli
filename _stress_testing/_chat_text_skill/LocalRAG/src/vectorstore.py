"""Vector store module for Local RAG using Chroma."""

from typing import List, Dict, Any, Optional, Tuple
import gzip
import hashlib
import json
import time
import chromadb
from chromadb.config import Settings

from observability import get_logger, vectorstore_operations, traced, log_error_alert, observe_latency


class VectorStore:
    """Vector storage and retrieval using Chroma with persistence.

    Supports HNSW index configuration for tuning search quality vs speed.
    """

    # Chroma HNSW defaults
    DEFAULT_HNSW_SPACE = "l2"
    DEFAULT_HNSW_EF_CONSTRUCTION = 100
    DEFAULT_HNSW_EF_SEARCH = 100
    DEFAULT_HNSW_MAX_NEIGHBORS = 16

    def __init__(
        self,
        persist_directory: str = "data/chroma_db",
        collection_name: str = "documents",
        hnsw_space: Optional[str] = None,
        hnsw_ef_construction: Optional[int] = None,
        hnsw_ef_search: Optional[int] = None,
        hnsw_max_neighbors: Optional[int] = None,
        _client: Optional[chromadb.ClientAPI] = None,
    ):
        """Initialize the vector store.

        Args:
            persist_directory: Directory for Chroma persistence.
            collection_name: Name of the collection.
            hnsw_space: Distance metric — "l2" (default), "cosine", or "ip" (inner product).
            hnsw_ef_construction: Build-time accuracy vs speed. Higher = better accuracy, slower build.
                Only applies to NEW collections (cannot change after creation).
            hnsw_ef_search: Query-time accuracy vs speed. Higher = better accuracy, slower search.
                Can be updated on existing collections via configure_ef_search().
            hnsw_max_neighbors: Max neighbors in HNSW graph. Only applies to NEW collections.
            _client: Internal use only — shared Chroma client instance (used by VectorStoreMulti).
        """
        self.persist_directory = persist_directory
        self.collection_name = collection_name
        self.hnsw_space = hnsw_space or self.DEFAULT_HNSW_SPACE
        self.hnsw_ef_construction = hnsw_ef_construction or self.DEFAULT_HNSW_EF_CONSTRUCTION
        self.hnsw_ef_search = hnsw_ef_search or self.DEFAULT_HNSW_EF_SEARCH
        self.hnsw_max_neighbors = hnsw_max_neighbors or self.DEFAULT_HNSW_MAX_NEIGHBORS
        self.client = _client if _client is not None else chromadb.PersistentClient(path=persist_directory)
        self._collection = None
        self.logger = get_logger(__name__)

    @property
    def _hnsw_config(self) -> dict:
        """Build HNSW configuration dict for Chroma API."""
        return {
            "hnsw": {
                "space": self.hnsw_space,
                "ef_construction": self.hnsw_ef_construction,
                "ef_search": self.hnsw_ef_search,
                "max_neighbors": self.hnsw_max_neighbors,
            }
        }

    @property
    def collection(self):
        """Get or create the collection lazily.

        For new collections, applies HNSW configuration.
        For existing collections, HNSW config params (space, ef_construction, max_neighbors)
        are locked at creation time and cannot be changed.
        """
        if self._collection is None:
            self._collection = self.client.get_or_create_collection(
                name=self.collection_name,
                configuration=self._hnsw_config
            )
        return self._collection

    def configure_ef_search(self, ef_search: int) -> bool:
        """Update ef_search on an existing collection.

        Args:
            ef_search: New ef_search value (higher = better accuracy, slower search).

        Returns:
            True if update was applied, False if collection didn't exist yet.
        """
        if self._collection is None:
            # Collection not yet created — update will be applied at first access via __init__
            self.hnsw_ef_search = ef_search
            self.logger.info("ef_search queued for new collection", ef_search=ef_search)
            return False

        self._collection.modify(configuration={"hnsw": {"ef_search": ef_search}})
        actual = self._collection.configuration_json["hnsw"]["ef_search"]
        if actual == ef_search:
            self.hnsw_ef_search = ef_search
            self.logger.info("ef_search updated", ef_search=ef_search)
            return True
        else:
            self.logger.warning("ef_search update did not take effect", requested=ef_search, actual=actual)
            return False

    def get_index_config(self) -> Dict[str, Any]:
        """Return current HNSW index configuration.

        Returns:
            Dict with space, ef_construction, ef_search, max_neighbors values.
        """
        if self._collection is None:
            # Not yet created — return what's set in __init__
            return {
                "space": self.hnsw_space,
                "ef_construction": self.hnsw_ef_construction,
                "ef_search": self.hnsw_ef_search,
                "max_neighbors": self.hnsw_max_neighbors,
                "note": "from constructor (collection not yet created)"
            }
        return dict(self._collection.configuration_json["hnsw"])

    def _text_hash(self, text: str) -> str:
        """Generate a stable short hash for text content.

        Args:
            text: Text content to hash.

        Returns:
            First 16 characters of SHA256 hex digest.
        """
        return hashlib.sha256(text.encode('utf-8')).hexdigest()[:16]

    def insert(self, text: str, embedding: List[float], chunk_index: int, source_file: str) -> str:
        """Insert a chunk with its embedding.

        Args:
            text: Text content.
            embedding: Vector embedding.
            chunk_index: Index of chunk in source document.
            source_file: Source file name.

        Returns:
            ID of inserted record.
        """
        self.logger.debug("Inserting chunk", source_file=source_file, chunk_index=chunk_index)

        start = time.perf_counter()
        try:
            doc_id = f"{source_file}_{chunk_index}"
            self.collection.add(
                ids=[doc_id],
                documents=[text],
                embeddings=[embedding],
                metadatas=[{"chunk_index": chunk_index, "source_file": source_file}]
            )

            duration = time.perf_counter() - start
            observe_latency(vectorstore_operations, {"operation": "insert"}, duration)

            return doc_id
        except Exception as e:
            duration = time.perf_counter() - start
            observe_latency(vectorstore_operations, {"operation": "insert"}, duration)
            log_error_alert(self.logger, e, "vectorstore",
                          context={"source_file": source_file, "chunk_index": chunk_index})
            raise

    def insert_batch(self, chunks: List[Dict[str, Any]]) -> List[str]:
        """Insert multiple chunks with their embeddings.

        Args:
            chunks: List of chunk dictionaries with 'text', 'embedding', 'chunk_index', 'source_file'.

        Returns:
            List of inserted IDs.
        """
        self.logger.info("Batch inserting chunks", num_chunks=len(chunks))

        start = time.perf_counter()
        try:
            ids = []
            documents = []
            embeddings = []
            metadatas = []

            for chunk in chunks:
                doc_id = f"{chunk['source_file']}_{chunk['chunk_index']}"
                ids.append(doc_id)
                documents.append(chunk["text"])
                embeddings.append(chunk["embedding"])
                metadatas.append({
                    "chunk_index": chunk["chunk_index"],
                    "source_file": chunk["source_file"],
                    "content_hash": self._text_hash(chunk["text"]),
                    "embedding": chunk["embedding"],
                })

            self.collection.add(
                ids=ids,
                documents=documents,
                embeddings=embeddings,
                metadatas=metadatas
            )

            duration = time.perf_counter() - start
            observe_latency(vectorstore_operations, {"operation": "insert_batch"}, duration)

            self.logger.info("Batch insert completed", num_inserted=len(ids))
            return ids
        except Exception as e:
            duration = time.perf_counter() - start
            observe_latency(vectorstore_operations, {"operation": "insert_batch"}, duration)
            log_error_alert(self.logger, e, "vectorstore",
                          context={"batch_size": len(chunks)})
            raise

    def search(
        self,
        query_embedding: List[float],
        top_k: int = 5,
        filter_sources: Optional[List[str]] = None
    ) -> List[Dict[str, Any]]:
        """Search for most similar chunks.

        Args:
            query_embedding: Query vector.
            top_k: Number of results to return.
            filter_sources: If provided, only search in these documents (list).

        Returns:
            List of result dictionaries with 'text', 'source_file', 'chunk_index', 'score'.
        """
        self.logger.debug("Searching vector store", top_k=top_k,
                        has_filters=filter_sources is not None)

        start = time.perf_counter()
        try:
            if filter_sources:
                if len(filter_sources) == 1:
                    where_filter = {"source_file": filter_sources[0]}
                else:
                    where_filter = {"source_file": {"$in": filter_sources}}
            else:
                where_filter = None

            results = self.collection.query(
                query_embeddings=[query_embedding],
                n_results=top_k,
                where=where_filter,
                include=["documents", "metadatas", "distances"]
            )

            search_results = []
            if results["ids"] and len(results["ids"]) > 0:
                for i in range(len(results["ids"][0])):
                    doc_id = results["ids"][0][i]
                    distance = results["distances"][0][i]
                    document = results["documents"][0][i]
                    metadata = results["metadatas"][0][i]

                    search_results.append({
                        "id": doc_id,
                        "text": document,
                        "chunk_index": metadata.get("chunk_index", 0),
                        "source_file": metadata.get("source_file", "unknown"),
                        "score": 1.0 / (1.0 + distance)
                    })

            duration = time.perf_counter() - start
            observe_latency(vectorstore_operations, {"operation": "search"}, duration)

            self.logger.debug("Search completed", num_results=len(search_results))
            return search_results
        except Exception as e:
            duration = time.perf_counter() - start
            observe_latency(vectorstore_operations, {"operation": "search"}, duration)
            log_error_alert(self.logger, e, "vectorstore",
                          context={"top_k": top_k})
            raise

    def get_by_id(self, doc_id: str) -> Optional[Dict[str, Any]]:
        """Retrieve a chunk by its ID.

        Args:
            doc_id: ID of the chunk to retrieve.

        Returns:
            Dictionary with chunk data or None if not found.
        """
        results = self.collection.get(ids=[doc_id], include=["documents", "metadatas"])

        if not results["ids"] or len(results["ids"]) == 0:
            return None

        return {
            "id": results["ids"][0],
            "text": results["documents"][0],
            "chunk_index": results["metadatas"][0].get("chunk_index", 0),
            "source_file": results["metadatas"][0].get("source_file", "unknown")
        }

    def count(self) -> int:
        """Get total number of stored chunks.

        Returns:
            Number of chunks in the store.
        """
        return self.collection.count()

    def get_indexed_documents(self) -> List[Dict[str, Any]]:
        """Get list of all indexed documents with chunk counts.

        Returns:
            List of dictionaries with 'source_file' and 'chunk_count'.
        """
        all_data = self.collection.get()

        doc_stats = {}
        for i, metadata in enumerate(all_data["metadatas"]):
            source = metadata.get("source_file", "unknown")
            if source not in doc_stats:
                doc_stats[source] = {"source_file": source, "chunk_count": 0}
            doc_stats[source]["chunk_count"] += 1

        return list(doc_stats.values())

    def get_document_chunks(self, source_file: str) -> Dict[int, Dict[str, Any]]:
        """Get all chunks for a specific document.

        Args:
            source_file: Name of the document.

        Returns:
            Dict mapping chunk_index -> {text, content_hash, embedding, id}.
        """
        all_data = self.collection.get(
            include=["documents", "metadatas"]
        )

        result: Dict[int, Dict[str, Any]] = {}
        for i, metadata in enumerate(all_data["metadatas"]):
            if metadata.get("source_file") == source_file:
                chunk_index = metadata.get("chunk_index", 0)
                result[chunk_index] = {
                    "id": all_data["ids"][i],
                    "text": all_data["documents"][i],
                    "content_hash": metadata.get("content_hash", ""),
                    "embedding": metadata.get("embedding"),
                }
        return result

    def upsert_document(
        self,
        chunks: List[Dict[str, Any]],
        embedder,
    ) -> Tuple[int, int, int]:
        """Incrementally upsert document chunks.

        Compares new chunks against existing ones and only performs:
        - INSERT for new chunks
        - UPDATE (delete + insert) for changed chunks
        - DELETE for removed chunks

        Args:
            chunks: List of chunk dicts with 'text', 'embedding', 'chunk_index', 'source_file'.
            embedder: Embedder client for re-embedding changed chunks.

        Returns:
            Tuple of (inserted_count, updated_count, deleted_count).
        """
        if not chunks:
            return 0, 0, 0

        source_file = chunks[0]["source_file"]
        self.logger.info("Upserting document", source=source_file, num_chunks=len(chunks))

        # Get existing chunks for this document
        existing = self.get_document_chunks(source_file)
        existing_indices = set(existing.keys())
        new_indices = set(chunk["chunk_index"] for chunk in chunks)

        # Determine operations
        to_insert: List[Dict[str, Any]] = []
        to_update: List[Dict[str, Any]] = []
        indices_to_delete: List[str] = []

        for chunk in chunks:
            idx = chunk["chunk_index"]
            content_hash = self._text_hash(chunk["text"])
            if idx not in existing:
                # New chunk
                to_insert.append(chunk)
            elif existing[idx]["content_hash"] != content_hash:
                # Changed chunk — will need re-embedding
                to_update.append(chunk)
            # else: unchanged — skip (use stored embedding)

        for idx in existing_indices - new_indices:
            # Removed chunk
            indices_to_delete.append(existing[idx]["id"])

        total_embedded = 0

        # Delete removed chunks
        if indices_to_delete:
            self.collection.delete(ids=indices_to_delete)

        # Re-embed changed chunks if needed
        chunks_to_embed = to_update
        embeddings_map: Dict[int, List[float]] = {}

        if chunks_to_embed and embedder is not None:
            texts_to_embed: List[str] = []
            indices_to_embed: List[int] = []

            for c in chunks_to_embed:
                old_emb = existing[c["chunk_index"]].get("embedding")
                if old_emb is not None:
                    # Reuse stored embedding
                    embeddings_map[c["chunk_index"]] = old_emb
                else:
                    texts_to_embed.append(c["text"])
                    indices_to_embed.append(c["chunk_index"])

            if texts_to_embed:
                emb_results, _ = embedder.embed_batch(texts_to_embed, use_cache=True)
                for idx, emb in zip(indices_to_embed, emb_results):
                    embeddings_map[idx] = emb
                    total_embedded += 1

        # For update chunks where text changed, delete old first
        if to_update:
            ids_to_delete = [existing[c["chunk_index"]]["id"] for c in to_update]
            self.collection.delete(ids=ids_to_delete)
            self.logger.debug("Deleted updated chunks", count=len(ids_to_delete))

        # Build final chunk list (insert + update with resolved embeddings)
        all_chunks_to_add: List[Dict[str, Any]] = []
        for chunk in to_insert:
            all_chunks_to_add.append(chunk)

        for chunk in to_update:
            updated_chunk = dict(chunk)
            updated_chunk["embedding"] = embeddings_map.get(
                chunk["chunk_index"], chunk["embedding"]
            )
            all_chunks_to_add.append(updated_chunk)

        # Batch insert
        if all_chunks_to_add:
            self._insert_chunks_with_hash(all_chunks_to_add)

        inserted = len(to_insert)
        updated = len(to_update)
        deleted = len(indices_to_delete)

        self.logger.info("Upsert completed",
                        source=source_file,
                        inserted=inserted,
                        updated=updated,
                        deleted=deleted,
                        embeddings_called=total_embedded)

        return inserted, updated, deleted

    def _insert_chunks_with_hash(self, chunks: List[Dict[str, Any]]) -> List[str]:
        """Insert chunks with content_hash and embedding in metadata.

        Args:
            chunks: List of chunk dicts with 'text', 'embedding', 'chunk_index', 'source_file'.

        Returns:
            List of inserted IDs.
        """
        ids = []
        documents = []
        embeddings = []
        metadatas = []

        for chunk in chunks:
            doc_id = f"{chunk['source_file']}_{chunk['chunk_index']}"
            ids.append(doc_id)
            documents.append(chunk["text"])
            embeddings.append(chunk["embedding"])
            metadatas.append({
                "chunk_index": chunk["chunk_index"],
                "source_file": chunk["source_file"],
                "content_hash": self._text_hash(chunk["text"]),
                "embedding": chunk["embedding"],
            })

        self.collection.add(
            ids=ids,
            documents=documents,
            embeddings=embeddings,
            metadatas=metadatas
        )
        return ids

    def delete_document(self, source_file: str) -> int:
        """Delete all chunks from a specific document.

        Args:
            source_file: Name of the document to delete.

        Returns:
            Number of chunks deleted.
        """
        all_data = self.collection.get(include=["metadatas"])

        ids_to_delete = []
        for i, metadata in enumerate(all_data["metadatas"]):
            if metadata.get("source_file") == source_file:
                ids_to_delete.append(all_data["ids"][i])

        if ids_to_delete:
            self.collection.delete(ids=ids_to_delete)

        return len(ids_to_delete)

    def clear_all(self) -> None:
        """Delete all documents from the collection."""
        self.client.delete_collection(name=self.collection_name)
        self._collection = self.client.get_or_create_collection(name=self.collection_name)

    def export_collection(self, filepath: str, compressed: bool = True) -> int:
        """Export the entire collection to a JSON file.

        Exports all chunks (id, text, embedding, metadata) and collection
        configuration (HNSW index settings).

        Args:
            filepath: Path to the output file (.json or .json.gz).
            compressed: If True, gzip-compress the output.

        Returns:
            Number of chunks exported.
        """
        self.logger.info("Exporting collection", path=filepath, compressed=compressed)

        all_data = self.collection.get(include=["documents", "metadatas", "embeddings"])

        def _to_list(val):
            """Convert numpy arrays to Python lists for JSON serialization."""
            if hasattr(val, "tolist"):
                return val.tolist()
            return val

        records = []
        for i in range(len(all_data["ids"])):
            records.append({
                "id": all_data["ids"][i],
                "text": all_data["documents"][i],
                "embedding": _to_list(all_data["embeddings"][i]),
                "metadata": all_data["metadatas"][i],
            })

        payload = {
            "version": 1,
            "collection": self.collection_name,
            "hnsw_config": self.get_index_config(),
            "chunk_count": len(records),
            "records": records,
        }

        json_body = json.dumps(payload, ensure_ascii=False)

        if compressed:
            with gzip.open(filepath, "wt", encoding="utf-8") as f:
                f.write(json_body)
        else:
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(json_body)

        self.logger.info("Export completed", chunk_count=len(records), path=filepath)
        return len(records)

    def import_collection(
        self,
        filepath: str,
        mode: str = "replace",
    ) -> Tuple[int, int]:
        """Import chunks from a backup file into the collection.

        Args:
            filepath: Path to the backup file (.json or .json.gz).
            mode: "replace" — delete all existing chunks first, then import all.
                  "merge" — add/update records by id, keep existing ones.

        Returns:
            Tuple of (imported_count, skipped_count).
        """
        self.logger.info("Importing collection", path=filepath, mode=mode)

        # Read and parse
        if filepath.endswith(".gz"):
            with gzip.open(filepath, "rt", encoding="utf-8") as f:
                payload = json.load(f)
        else:
            with open(filepath, "r", encoding="utf-8") as f:
                payload = json.load(f)

        version = payload.get("version")
        if version != 1:
            raise ValueError(f"Unsupported backup version: {version}. Expected 1.")

        records = payload.get("records", [])
        self.logger.info("Backup loaded", chunk_count=len(records))

        if mode == "replace":
            # Delete all existing chunks — get ids via empty get + metadata lookup
            existing = self.collection.get(include=["metadatas"])
            if existing["ids"]:
                self.collection.delete(ids=existing["ids"])
            self.logger.debug("Cleared existing collection")

        elif mode == "merge":
            # Identify existing ids so we can skip them
            existing = self.collection.get(include=["metadatas"])
            existing_ids = set(existing["ids"])
            self.logger.debug("Merge mode", existing_count=len(existing_ids))
        else:
            raise ValueError(f"Invalid mode: {mode}. Use 'replace' or 'merge'.")

        imported = 0
        skipped = 0

        for record in records:
            rec_id = record["id"]
            if mode == "merge" and rec_id in existing_ids:
                skipped += 1
                continue

            self.collection.add(
                ids=[rec_id],
                documents=[record["text"]],
                embeddings=[record["embedding"]],
                metadatas=[record.get("metadata", {})],
            )
            imported += 1

        self.logger.info("Import completed", imported=imported, skipped=skipped)
        return imported, skipped


class VectorStoreMulti:
    """Multi-tenant vector store — each tenant gets an isolated collection.

    All tenants share the same persist_directory but use separate Chroma
    collections, providing logical isolation without separate DB files.

    Example:
        mstore = VectorStoreMulti(persist_directory="data/chroma_db")
        store = mstore.get_tenant("tenant_a")
        store.insert_batch(chunks)
    """

    def __init__(
        self,
        persist_directory: str = "data/chroma_db",
        default_hnsw_space: Optional[str] = None,
        default_hnsw_ef_construction: Optional[int] = None,
        default_hnsw_ef_search: Optional[int] = None,
        default_hnsw_max_neighbors: Optional[int] = None,
    ):
        """Initialize the multi-tenant store.

        Args:
            persist_directory: Shared directory for all tenants.
            default_hnsw_space: Default HNSW space for new tenant collections.
            default_hnsw_ef_construction: Default ef_construction for new tenants.
            default_hnsw_ef_search: Default ef_search for new tenants.
            default_hnsw_max_neighbors: Default max_neighbors for new tenants.
        """
        self.persist_directory = persist_directory
        self.client = chromadb.PersistentClient(path=persist_directory)
        self.logger = get_logger(__name__)
        self._stores: Dict[str, VectorStore] = {}
        self._default_hnsw = {
            "hnsw_space": default_hnsw_space,
            "hnsw_ef_construction": default_hnsw_ef_construction,
            "hnsw_ef_search": default_hnsw_ef_search,
            "hnsw_max_neighbors": default_hnsw_max_neighbors,
        }

    def _collection_name(self, tenant: str) -> str:
        """Build a collection name for a tenant using reversible encoding."""
        # Use escape sequences so the reversal is unambiguous
        safe = tenant.replace("_", "__UNDER__").replace("/", "__SLASH__").replace(":", "__COLON__")
        return f"tenant_{safe}"

    def _tenant_from_collection(self, collection_name: str) -> str:
        """Reverse the encoding to recover the tenant identifier."""
        prefix = "tenant_"
        if not collection_name.startswith(prefix):
            return collection_name
        tenant_raw = collection_name[len(prefix):]
        return tenant_raw.replace("__UNDER__", "_").replace("__SLASH__", "/").replace("__COLON__", ":")

    def get_tenant(self, tenant: str) -> VectorStore:
        """Get or create a VectorStore for a specific tenant.

        Args:
            tenant: Unique tenant identifier (e.g. "user_123" or "org:team").

        Returns:
            VectorStore instance for this tenant (isolated collection).
        """
        if tenant not in self._stores:
            collection_name = self._collection_name(tenant)
            self.logger.info("Creating tenant collection", tenant=tenant, collection=collection_name)
            self._stores[tenant] = VectorStore(
                persist_directory=self.persist_directory,
                collection_name=collection_name,
                _client=self.client,
                **self._default_hnsw,
            )
            # Trigger collection creation so Chroma actually creates the collection
            _ = self._stores[tenant].collection
        return self._stores[tenant]

    def list_tenants(self) -> List[str]:
        """List all tenant identifiers that have collections.

        Returns:
            List of tenant identifiers.
        """
        collections = self.client.list_collections()
        prefix = "tenant_"
        tenants = []
        for col in collections:
            if col.name.startswith(prefix):
                tenants.append(self._tenant_from_collection(col.name))
        return tenants

    def delete_tenant(self, tenant: str) -> bool:
        """Delete all data for a tenant (drops the collection).

        Args:
            tenant: Tenant identifier.

        Returns:
            True if tenant existed and was deleted, False if tenant didn't exist.
        """
        existing = self.list_tenants()
        if tenant not in existing:
            return False

        collection_name = self._collection_name(tenant)
        self.client.delete_collection(name=collection_name)
        if tenant in self._stores:
            del self._stores[tenant]
        self.logger.info("Deleted tenant collection", tenant=tenant)
        return True

        if tenant in self._stores:
            del self._stores[tenant]

        self.logger.info("Deleted tenant collection", tenant=tenant)
        return True

    def tenant_stats(self) -> List[Dict[str, Any]]:
        """Get statistics for all tenants.

        Returns:
            List of dicts with tenant, chunk_count, and collection_name.
        """
        stats = []
        for tenant in self.list_tenants():
            store = self.get_tenant(tenant)
            stats.append({
                "tenant": tenant,
                "chunk_count": store.count(),
                "collection": store.collection_name,
            })
        return stats

