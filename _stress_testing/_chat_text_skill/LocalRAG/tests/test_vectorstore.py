"""Tests for vectorstore module."""

import json
import gzip
import os
import pytest
import tempfile
import shutil
from pathlib import Path

from src.vectorstore import VectorStore, VectorStoreMulti


class TestVectorStoreBasics:
    """Tests for basic vectorstore operations."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.store = VectorStore(persist_directory=self.temp_dir, collection_name="test")

    def teardown_method(self):
        """Clean up temp files."""
        try:
            shutil.rmtree(self.temp_dir)
        except:
            pass

    # -------------------------------------------------------------------------
    # insert_batch with content_hash and embedding in metadata
    # -------------------------------------------------------------------------

    def test_insert_batch_includes_content_hash_in_metadata(self):
        """insert_batch stores content_hash in metadata."""
        chunks = [
            {"text": "hello world", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "doc.txt"},
            {"text": "second chunk", "embedding": [0.2] * 1536, "chunk_index": 1, "source_file": "doc.txt"},
        ]
        self.store.insert_batch(chunks)

        stored = self.store.collection.get(include=["metadatas"])
        hashes = [m["content_hash"] for m in stored["metadatas"]]
        assert len(hashes) == 2
        assert all(h == hashes[0] for h in hashes) is False  # different texts → different hashes
        assert all(len(h) == 16 for h in hashes)

    def test_insert_batch_includes_embedding_in_metadata(self):
        """insert_batch stores embedding in metadata."""
        chunks = [
            {"text": "hello", "embedding": [0.42] * 1536, "chunk_index": 0, "source_file": "doc.txt"},
        ]
        self.store.insert_batch(chunks)

        stored = self.store.collection.get(include=["metadatas"])
        stored_emb = stored["metadatas"][0]["embedding"]
        assert stored_emb == [0.42] * 1536

    def test_text_hash_stable(self):
        """_text_hash returns same value for same text."""
        h1 = self.store._text_hash("hello world")
        h2 = self.store._text_hash("hello world")
        assert h1 == h2
        assert len(h1) == 16

    def test_text_hash_different_for_different_text(self):
        """_text_hash returns different values for different texts."""
        h1 = self.store._text_hash("hello")
        h2 = self.store._text_hash("world")
        assert h1 != h2

    # -------------------------------------------------------------------------
    # get_document_chunks
    # -------------------------------------------------------------------------

    def test_get_document_chunks_returns_all_chunks(self):
        """get_document_chunks returns all chunks for a document."""
        chunks = [
            {"text": "chunk 0", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "doc.txt"},
            {"text": "chunk 1", "embedding": [0.2] * 1536, "chunk_index": 1, "source_file": "doc.txt"},
            {"text": "chunk 2", "embedding": [0.3] * 1536, "chunk_index": 2, "source_file": "doc.txt"},
        ]
        self.store.insert_batch(chunks)

        result = self.store.get_document_chunks("doc.txt")
        assert len(result) == 3
        assert 0 in result and 1 in result and 2 in result
        assert result[0]["text"] == "chunk 0"
        assert result[1]["text"] == "chunk 1"
        assert result[2]["text"] == "chunk 2"

    def test_get_document_chunks_empty_for_unknown_doc(self):
        """get_document_chunks returns empty dict for unknown document."""
        result = self.store.get_document_chunks("nonexistent.txt")
        assert result == {}

    def test_get_document_chunks_excludes_other_docs(self):
        """get_document_chunks only returns chunks for specified document."""
        self.store.insert_batch([
            {"text": "doc1 chunk", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "doc1.txt"},
            {"text": "doc2 chunk", "embedding": [0.2] * 1536, "chunk_index": 0, "source_file": "doc2.txt"},
        ])
        result = self.store.get_document_chunks("doc1.txt")
        assert len(result) == 1
        assert result[0]["text"] == "doc1 chunk"

    # -------------------------------------------------------------------------
    # upsert_document
    # -------------------------------------------------------------------------

    def test_upsert_insert_new_chunks(self):
        """upsert_document inserts brand new chunks when doc doesn't exist."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        chunks = [
            {"text": "new chunk", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "new.txt"},
        ]
        inserted, updated, deleted = self.store.upsert_document(chunks, mock_emb)

        assert inserted == 1
        assert updated == 0
        assert deleted == 0
        assert self.store.count() == 1

    def test_upsert_skip_unchanged_chunks(self):
        """upsert_document skips chunks when text content unchanged."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder(should_increment=False)

        # First: insert (no embed_batch called — chunk goes to to_insert with its own embedding)
        chunks_v1 = [
            {"text": "stable text", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "stable.txt"},
        ]
        self.store.upsert_document(chunks_v1, mock_emb)
        assert self.store.count() == 1

        # Second: same text — should skip (use stored embedding, no embed_batch call)
        chunks_v2 = [
            {"text": "stable text", "embedding": [0.2] * 1536, "chunk_index": 0, "source_file": "stable.txt"},
        ]
        inserted, updated, deleted = self.store.upsert_document(chunks_v2, mock_emb)
        assert inserted == 0
        assert updated == 0
        assert deleted == 0
        assert mock_emb.embed_batch_called == 0  # no re-embedding needed
        assert self.store.count() == 1  # still 1 chunk

    def test_upsert_update_changed_chunks(self):
        """upsert_document updates chunks when text content changed."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        # First: insert
        chunks_v1 = [
            {"text": "original", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "changeme.txt"},
        ]
        self.store.upsert_document(chunks_v1, mock_emb)

        # Second: text changed
        chunks_v2 = [
            {"text": "modified text", "embedding": [0.2] * 1536, "chunk_index": 0, "source_file": "changeme.txt"},
        ]
        inserted, updated, deleted = self.store.upsert_document(chunks_v2, mock_emb)

        assert inserted == 0
        assert updated == 1
        assert deleted == 0

    def test_upsert_delete_removed_chunks(self):
        """upsert_document deletes chunks that are no longer present."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        # First: insert 3 chunks
        chunks_v1 = [
            {"text": f"chunk {i}", "embedding": [0.1] * 1536, "chunk_index": i, "source_file": "shrink.txt"}
            for i in range(3)
        ]
        self.store.upsert_document(chunks_v1, mock_emb)
        assert self.store.count() == 3

        # Second: only 1 chunk at index 0 with DIFFERENT text — should update index 0, delete 1,2
        chunks_v2 = [
            {"text": "only one", "embedding": [0.2] * 1536, "chunk_index": 0, "source_file": "shrink.txt"},
        ]
        inserted, updated, deleted = self.store.upsert_document(chunks_v2, mock_emb)

        assert inserted == 0
        assert updated == 1   # chunk 0 text changed: "chunk 0" → "only one"
        assert deleted == 2   # chunks 1, 2 removed
        assert self.store.count() == 1

    def test_upsert_mixed_operations(self):
        """upsert_document handles insert + update + delete simultaneously."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        # First: insert 3 chunks
        chunks_v1 = [
            {"text": f"chunk {i}", "embedding": [0.1] * 1536, "chunk_index": i, "source_file": "mixed.txt"}
            for i in range(3)
        ]
        self.store.upsert_document(chunks_v1, mock_emb)
        assert self.store.count() == 3

        # Second:
        # - chunk 0: unchanged → skip
        # - chunk 1: changed → update
        # - chunk 2: gone → delete
        # - chunk 3: new → insert
        chunks_v2 = [
            {"text": "chunk 0", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "mixed.txt"},  # unchanged
            {"text": "modified chunk 1", "embedding": [0.2] * 1536, "chunk_index": 1, "source_file": "mixed.txt"},  # updated
            {"text": "brand new chunk 3", "embedding": [0.3] * 1536, "chunk_index": 3, "source_file": "mixed.txt"},  # new
        ]
        inserted, updated, deleted = self.store.upsert_document(chunks_v2, mock_emb)

        assert inserted == 1   # chunk 3
        assert updated == 1   # chunk 1
        assert deleted == 1   # chunk 2
        assert self.store.count() == 3

    def test_upsert_empty_chunks_list(self):
        """upsert_document with empty list returns (0, 0, 0)."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        result = self.store.upsert_document([], mock_emb)
        assert result == (0, 0, 0)

    def test_upsert_document_empty_after_existing(self):
        """upsert_document deletes all chunks when new list is empty (empty doc)."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        # First: insert
        self.store.insert_batch([
            {"text": "some text", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "toempty.txt"},
        ])
        assert self.store.count() == 1

        # Upsert with empty chunks — should delete all for that source
        inserted, updated, deleted = self.store.upsert_document([], mock_emb)
        assert inserted == 0
        assert updated == 0
        assert deleted == 0  # empty list means no comparison possible, no-op

    def test_upsert_with_filter(self):
        """upsert_document works correctly with filter_sources in search."""
        MockEmbedder = self._make_embedder_mock()
        mock_emb = MockEmbedder()

        # Insert for two documents
        self.store.insert_batch([
            {"text": "doc1 content", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "doc1.txt"},
            {"text": "doc2 content", "embedding": [0.2] * 1536, "chunk_index": 0, "source_file": "doc2.txt"},
        ])

        # Upsert doc1 only
        self.store.upsert_document([
            {"text": "doc1 updated", "embedding": [0.3] * 1536, "chunk_index": 0, "source_file": "doc1.txt"},
        ], mock_emb)

        # Search with filter should find updated doc1
        results = self.store.search([0.3] * 1536, top_k=5, filter_sources=["doc1.txt"])
        assert len(results) == 1
        assert results[0]["text"] == "doc1 updated"

    # -------------------------------------------------------------------------
    # Existing functionality regressions
    # -------------------------------------------------------------------------

    def test_delete_document(self):
        """delete_document removes all chunks for a source (existing behavior)."""
        self.store.insert_batch([
            {"text": "chunk 0", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "todelete.txt"},
            {"text": "chunk 1", "embedding": [0.2] * 1536, "chunk_index": 1, "source_file": "todelete.txt"},
            {"text": "other", "embedding": [0.3] * 1536, "chunk_index": 0, "source_file": "other.txt"},
        ])
        deleted = self.store.delete_document("todelete.txt")
        assert deleted == 2
        assert self.store.count() == 1

    def test_clear_all(self):
        """clear_all removes all chunks (existing behavior)."""
        self.store.insert_batch([
            {"text": "chunk", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "a.txt"},
            {"text": "chunk", "embedding": [0.2] * 1536, "chunk_index": 0, "source_file": "b.txt"},
        ])
        assert self.store.count() == 2
        self.store.clear_all()
        assert self.store.count() == 0

    def test_get_indexed_documents(self):
        """get_indexed_documents returns correct stats (existing behavior)."""
        self.store.insert_batch([
            {"text": "a", "embedding": [0.1] * 1536, "chunk_index": 0, "source_file": "a.txt"},
            {"text": "b", "embedding": [0.2] * 1536, "chunk_index": 1, "source_file": "a.txt"},
            {"text": "c", "embedding": [0.3] * 1536, "chunk_index": 0, "source_file": "b.txt"},
        ])
        docs = self.store.get_indexed_documents()
        doc_map = {d["source_file"]: d["chunk_count"] for d in docs}
        assert doc_map["a.txt"] == 2
        assert doc_map["b.txt"] == 1

    def test_count(self):
        """count returns correct number of chunks."""
        self.store.insert_batch([
            {"text": "a", "embedding": [0.1] * 1536, "chunk_index": i, "source_file": "f.txt"}
            for i in range(5)
        ])
        assert self.store.count() == 5

    # -------------------------------------------------------------------------
    # Helper
    # -------------------------------------------------------------------------

    def _make_embedder_mock(self):
        """Return a mock embedder class that tracks embed_batch calls.

        Set should_increment=False to make embed_batch NOT increment the call
        counter (used to verify skip-on-unchanged behavior).
        """
        class MockEmbedder:
            def __init__(self, should_increment=True):
                self.embed_batch_called = 0
                self._should_increment = should_increment

            def embed_batch(self, texts, use_cache=True):
                if self._should_increment:
                    self.embed_batch_called += 1
                return ([0.99] * 1536 for _ in texts), []

        return MockEmbedder


class TestVectorStoreHNSWConfig:
    """Tests for HNSW index configuration."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.store = VectorStore(persist_directory=self.temp_dir, collection_name="test")

    def teardown_method(self):
        """Clean up temp files."""
        try:
            shutil.rmtree(self.temp_dir)
        except:
            pass

    def test_new_collection_uses_hnsw_config(self):
        """New collection uses ef_construction and space from constructor."""
        store = VectorStore(
            persist_directory=self.temp_dir,
            collection_name="hnsw_test",
            hnsw_space="cosine",
            hnsw_ef_construction=200,
            hnsw_ef_search=150,
            hnsw_max_neighbors=32,
        )
        # Trigger collection creation
        _ = store.collection
        config = store.get_index_config()
        assert config["space"] == "cosine"
        assert config["ef_construction"] == 200
        assert config["ef_search"] == 150
        assert config["max_neighbors"] == 32

    def test_new_collection_uses_defaults(self):
        """New collection uses Chroma defaults when no HNSW params specified."""
        store = VectorStore(
            persist_directory=self.temp_dir,
            collection_name="defaults_test",
        )
        _ = store.collection
        config = store.get_index_config()
        assert config["space"] == VectorStore.DEFAULT_HNSW_SPACE
        assert config["ef_construction"] == VectorStore.DEFAULT_HNSW_EF_CONSTRUCTION
        assert config["ef_search"] == VectorStore.DEFAULT_HNSW_EF_SEARCH
        assert config["max_neighbors"] == VectorStore.DEFAULT_HNSW_MAX_NEIGHBORS

    def test_configure_ef_search_updates_existing_collection(self):
        """configure_ef_search updates ef_search on an already-created collection."""
        # Create collection first
        self.store.insert_batch([{
            "text": "hello",
            "embedding": [0.1] * 5,
            "chunk_index": 0,
            "source_file": "doc.txt",
        }])
        # Now update ef_search
        result = self.store.configure_ef_search(250)
        assert result is True
        config = self.store.get_index_config()
        assert config["ef_search"] == 250

    def test_configure_ef_search_queued_for_not_yet_created_collection(self):
        """configure_ef_search before collection exists queues the value."""
        store = VectorStore(
            persist_directory=self.temp_dir,
            collection_name="not_yet_created",
            hnsw_ef_search=100,
        )
        # Call configure_ef_search before collection exists
        result = store.configure_ef_search(300)
        assert result is False  # not yet created
        assert store.hnsw_ef_search == 300
        # Now trigger creation
        _ = store.collection
        config = store.get_index_config()
        assert config["ef_search"] == 300

    def test_get_index_config_before_collection_created(self):
        """get_index_config returns constructor values when collection not yet created."""
        store = VectorStore(
            persist_directory=self.temp_dir,
            collection_name="brand_new",
            hnsw_ef_search=123,
        )
        config = store.get_index_config()
        assert "note" in config
        assert config["ef_search"] == 123

    def test_update_ef_search_no_reindex(self):
        """Updating ef_search does not require re-embedding or re-indexing."""
        self.store.insert_batch([{
            "text": "test",
            "embedding": [0.1] * 5,
            "chunk_index": 0,
            "source_file": "doc.txt",
        }])
        assert self.store.count() == 1
        self.store.configure_ef_search(200)
        assert self.store.count() == 1  # still there
        results = self.store.search([0.1] * 5, top_k=1)
        assert len(results) == 1
        assert results[0]["text"] == "test"


class TestVectorStoreExportImport:
    """Tests for export and import functionality."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.store = VectorStore(persist_directory=self.temp_dir, collection_name="test")

    def teardown_method(self):
        """Clean up temp files."""
        try:
            shutil.rmtree(self.temp_dir)
        except:
            pass

    def _sample_chunks(self):
        return [
            {"text": "first document", "embedding": [0.1] * 5, "chunk_index": 0, "source_file": "a.txt"},
            {"text": "second document", "embedding": [0.2] * 5, "chunk_index": 1, "source_file": "a.txt"},
            {"text": "third document", "embedding": [0.3] * 5, "chunk_index": 0, "source_file": "b.txt"},
        ]

    def test_export_compressed_json(self):
        """export_collection writes a valid gzip-compressed JSON file."""
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "backup.json.gz")
        count = self.store.export_collection(path, compressed=True)

        assert count == 3
        assert os.path.exists(path)
        with gzip.open(path, "rt") as f:
            data = json.load(f)
        assert data["version"] == 1
        assert data["chunk_count"] == 3
        assert len(data["records"]) == 3

    def test_export_uncompressed_json(self):
        """export_collection writes valid uncompressed JSON when compressed=False."""
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "backup.json")
        count = self.store.export_collection(path, compressed=False)

        assert count == 3
        assert os.path.exists(path)
        with open(path) as f:
            data = json.load(f)
        assert data["version"] == 1
        assert data["chunk_count"] == 3

    def test_export_includes_hnsw_config(self):
        """export_collection includes HNSW configuration in the backup."""
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "backup.json.gz")
        self.store.export_collection(path)

        with gzip.open(path, "rt") as f:
            data = json.load(f)
        assert "hnsw_config" in data
        assert "space" in data["hnsw_config"]

    def test_export_empty_collection(self):
        """export_collection returns 0 for an empty collection."""
        path = os.path.join(self.temp_dir, "empty.json.gz")
        count = self.store.export_collection(path)
        assert count == 0

    def test_import_replace_mode(self):
        """import_collection with mode=replace clears and restores all data."""
        # First: export
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "backup.json.gz")
        self.store.export_collection(path)

        # Clear the store
        self.store.clear_all()
        assert self.store.count() == 0

        # Import
        imported, skipped = self.store.import_collection(path, mode="replace")
        assert imported == 3
        assert skipped == 0
        assert self.store.count() == 3

    def test_import_merge_mode(self):
        """import_collection with mode=merge adds without overwriting."""
        # Add one chunk first
        self.store.insert_batch([self._sample_chunks()[0]])
        assert self.store.count() == 1

        # Export a 3-chunk backup (different set of chunks)
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "backup.json.gz")
        self.store.export_collection(path)
        # Now remove all and re-add just the original chunk
        self.store.clear_all()
        self.store.insert_batch([self._sample_chunks()[0]])
        assert self.store.count() == 1

        # Merge import — should add 2 new chunks
        imported, skipped = self.store.import_collection(path, mode="merge")
        assert imported == 2   # 2 new chunks (chunks 1 and 2 from backup)
        assert skipped == 1    # 1 already existed (chunk 0)
        assert self.store.count() == 3

    def test_import_with_filter(self):
        """imported collection data is searchable normally."""
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "backup.json.gz")
        self.store.export_collection(path)

        self.store.clear_all()
        self.store.import_collection(path, mode="replace")

        results = self.store.search([0.1] * 5, top_k=5)
        assert len(results) == 3
        texts = {r["text"] for r in results}
        assert texts == {"first document", "second document", "third document"}

    def test_import_invalid_version_raises(self):
        """import_collection raises ValueError for unsupported version."""
        path = os.path.join(self.temp_dir, "bad_version.json.gz")
        with gzip.open(path, "wt") as f:
            json.dump({"version": 99, "records": []}, f)

        with pytest.raises(ValueError, match="Unsupported backup version"):
            self.store.import_collection(path)

    def test_export_and_import_preserves_all_fields(self):
        """Round-trip export/import preserves id, text, embedding, metadata."""
        self.store.insert_batch(self._sample_chunks())
        path = os.path.join(self.temp_dir, "roundtrip.json.gz")
        self.store.export_collection(path)

        self.store.clear_all()
        self.store.import_collection(path, mode="replace")

        chunks = self.store.get_document_chunks("a.txt")
        assert len(chunks) == 2
        assert chunks[0]["text"] == "first document"
        assert chunks[1]["text"] == "second document"
        # Check content_hash and embedding are preserved
        assert chunks[0]["content_hash"] != ""
        assert len(chunks[0]["embedding"]) == 5


class TestVectorStoreMultiTenant:
    """Tests for multi-tenant isolation."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.mstore = VectorStoreMulti(persist_directory=self.temp_dir)

    def teardown_method(self):
        """Clean up temp files."""
        try:
            shutil.rmtree(self.temp_dir)
        except:
            pass

    def test_get_tenant_returns_isolated_store(self):
        """Different tenants get separate collections with no data overlap."""
        store_a = self.mstore.get_tenant("tenant_a")
        store_b = self.mstore.get_tenant("tenant_b")

        store_a.insert_batch([{
            "text": "from tenant A",
            "embedding": [0.1] * 5,
            "chunk_index": 0,
            "source_file": "a.txt",
        }])
        store_b.insert_batch([{
            "text": "from tenant B",
            "embedding": [0.2] * 5,
            "chunk_index": 0,
            "source_file": "b.txt",
        }])

        # Each store only sees its own data
        assert store_a.count() == 1
        assert store_b.count() == 1
        assert store_a.search([0.1] * 5, top_k=1)[0]["text"] == "from tenant A"
        assert store_b.search([0.2] * 5, top_k=1)[0]["text"] == "from tenant B"

    def test_same_tenant_returns_same_store(self):
        """get_tenant returns the same VectorStore instance for same tenant."""
        s1 = self.mstore.get_tenant("tenant_x")
        s2 = self.mstore.get_tenant("tenant_x")
        assert s1 is s2

    def test_different_tenants_get_different_stores(self):
        """Different tenants always get different VectorStore instances."""
        s1 = self.mstore.get_tenant("tenant_a")
        s2 = self.mstore.get_tenant("tenant_b")
        assert s1 is not s2
        assert s1.collection_name != s2.collection_name

    def test_list_tenants_returns_all_tenants(self):
        """list_tenants returns all tenants that have been initialized."""
        self.mstore.get_tenant("alpha")
        self.mstore.get_tenant("beta")
        self.mstore.get_tenant("gamma")

        tenants = self.mstore.list_tenants()
        assert set(tenants) == {"alpha", "beta", "gamma"}

    def test_delete_tenant_removes_collection(self):
        """delete_tenant drops the tenant's collection."""
        self.mstore.get_tenant("to_delete")
        assert len(self.mstore.list_tenants()) == 1

        result = self.mstore.delete_tenant("to_delete")
        assert result is True
        assert len(self.mstore.list_tenants()) == 0

    def test_delete_tenant_unknown_returns_false(self):
        """delete_tenant returns False if tenant doesn't exist."""
        result = self.mstore.delete_tenant("nonexistent")
        assert result is False

    def test_tenant_stats_reports_chunk_counts(self):
        """tenant_stats returns correct chunk counts per tenant."""
        ta = self.mstore.get_tenant("alpha")
        tb = self.mstore.get_tenant("beta")
        ta.insert_batch([{
            "text": f"chunk {i}",
            "embedding": [0.1] * 5,
            "chunk_index": i,
            "source_file": "f.txt",
        } for i in range(3)])
        tb.insert_batch([{
            "text": "only one",
            "embedding": [0.2] * 5,
            "chunk_index": 0,
            "source_file": "f.txt",
        }])

        stats = {s["tenant"]: s["chunk_count"] for s in self.mstore.tenant_stats()}
        assert stats["alpha"] == 3
        assert stats["beta"] == 1

    def test_tenant_can_use_upsert_and_search(self):
        """A tenant's VectorStore supports upsert, search, and delete normally."""
        store = self.mstore.get_tenant("test_tenant")

        # Upsert
        store.insert_batch([{
            "text": "original",
            "embedding": [0.1] * 5,
            "chunk_index": 0,
            "source_file": "doc.txt",
        }])
        assert store.count() == 1

        # Search
        results = store.search([0.1] * 5, top_k=1)
        assert results[0]["text"] == "original"

        # Delete
        deleted = store.delete_document("doc.txt")
        assert deleted == 1
        assert store.count() == 0


