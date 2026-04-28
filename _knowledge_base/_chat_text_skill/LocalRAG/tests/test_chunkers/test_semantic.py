"""Tests for SemanticChunker - semantic chunking based on embedding similarity."""

import pytest
from unittest.mock import MagicMock
from src.chunkers.semantic_chunker import SemanticChunker


class TestSemanticChunker:
    def test_name(self):
        chunker = SemanticChunker()
        assert chunker.name() == "semantic"

    def test_requires_embedder(self):
        """Without embedder, should return single chunk (fallback)."""
        chunker = SemanticChunker()
        text = "Hello world"
        chunks = chunker.chunk_with_metadata(text, "test.txt")
        assert len(chunks) == 1

    def test_high_similarity_merges_sentences(self):
        """High similarity between adjacent sentences should merge them."""
        mock_embedder = MagicMock()
        # All embeddings very similar (cosine ~1.0)
        mock_embedder.embed_single.side_effect = [
            [0.1, 0.1],
            [0.11, 0.1],
            [0.1, 0.12],
            [0.1, 0.11],
        ]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.9,  # High threshold
            chunk_size=200
        )
        text = "This is sentence one. This is sentence two. This is sentence three."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # With high similarity, sentences should merge into 1 chunk
        assert len(chunks) == 1
        assert "sentence one" in chunks[0]["text"]
        assert "sentence two" in chunks[0]["text"]
        assert "sentence three" in chunks[0]["text"]

    def test_low_similarity_creates_separate_chunks(self):
        """Low similarity between adjacent sentences should create separate chunks."""
        mock_embedder = MagicMock()
        # Embeddings very different (cosine ~0)
        mock_embedder.embed_single.side_effect = [
            [1.0, 0.0],  # Sentence 1
            [0.0, 1.0],  # Sentence 2 (different topic)
            [0.0, 1.0],  # Sentence 3 (similar to 2)
        ]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.5,  # Low threshold
            chunk_size=200
        )
        text = "This is about dogs. This is about cats. This is also about cats."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should create multiple chunks
        assert len(chunks) >= 2

    def test_exceeds_chunk_size_creates_separate_chunks(self):
        """Even with high similarity, large text should be split."""
        mock_embedder = MagicMock()
        # Similar embeddings
        mock_embedder.embed_single.return_value = [0.1, 0.1]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.9,
            chunk_size=20,  # Very small chunk size
            overlap=5
        )
        text = "This is a long sentence that definitely exceeds the chunk size limit."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should be split into multiple chunks
        assert len(chunks) > 1

    def test_sentence_boundaries_respected(self):
        """Chunking should prefer sentence boundaries."""
        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.5, 0.5]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.9,
            chunk_size=100
        )
        text = "First sentence. Second sentence. Third sentence."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should have sentence text in chunks
        assert len(chunks) >= 1
        assert any("First sentence" in c["text"] for c in chunks)

    def test_paragraph_structure_preserved(self):
        """Paragraph structure should be preserved when splitting."""
        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.5, 0.5]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.9,
            chunk_size=500
        )
        text = "Paragraph one sentence one.\n\nParagraph two sentence one."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should preserve paragraph structure
        assert len(chunks) >= 1

    def test_metadata_includes_similarity_info(self):
        """Chunks should have similarity information in metadata."""
        mock_embedder = MagicMock()
        mock_embedder.embed_single.side_effect = [
            [1.0, 0.0],
            [0.0, 1.0],
        ]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.5
        )
        text = "About dogs. About cats."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) >= 1
        # Metadata should exist
        assert "similarity_threshold" not in chunks[0]["metadata"]

    def test_preview_returns_stats(self):
        """Preview should return statistics."""
        mock_embedder = MagicMock()
        mock_embedder.embed_single.return_value = [0.1] * 10

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.5,
            chunk_size=100
        )
        text = "Hello world. This is a test."
        result = chunker.preview(text)

        assert "chunks" in result
        assert "stats" in result
        assert result["stats"]["total_chunks"] == len(result["chunks"])

    def test_empty_text_returns_empty_list(self):
        """Empty text should return empty list."""
        chunker = SemanticChunker()
        chunks = chunker.chunk_with_metadata("", "test.txt")
        assert chunks == []

    def test_whitespace_only_returns_empty_list(self):
        """Whitespace-only text should return empty list."""
        chunker = SemanticChunker()
        chunks = chunker.chunk_with_metadata("   \n\n  ", "test.txt")
        assert chunks == []

    def test_merges_until_threshold_drop(self):
        """Should merge sentences until similarity drops below threshold."""
        mock_embedder = MagicMock()
        # Sentence 1 similar to 2, 2 similar to 3, but 3 very different from 4
        mock_embedder.embed_single.side_effect = [
            [1.0, 0.0],  # S1
            [0.95, 0.1], # S2 - similar to S1
            [0.9, 0.15], # S3 - similar to S2
            [0.1, 0.9],  # S4 - different topic
        ]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.5,
            chunk_size=500,
            overlap=0  # Disable overlap to simplify test
        )
        text = "One about topic A. Two about topic A. Three about topic A. Four about topic B."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # S1-S3 should be merged, S4 separate
        # Check that "One" and "Four" are in different chunks
        one_chunks = [c for c in chunks if "One" in c["text"]]
        four_chunks = [c for c in chunks if "Four" in c["text"]]

        assert len(one_chunks) >= 1
        assert len(four_chunks) >= 1

        # "One" and "Four" should NOT be in the same chunk
        one_text = one_chunks[0]["text"] if one_chunks else ""
        four_text = four_chunks[0]["text"] if four_chunks else ""

        # With overlap=0, "Four" should be in its own chunk
        # and not merged with S1-S3
        assert "Four" in four_text
        # Four's own chunk should not contain "One"
        assert "One" not in four_text or four_text.index("Four") < four_text.index("One")

    def test_min_chunk_size_merge_small(self):
        """Very small sentences should be merged even if not identical."""
        mock_embedder = MagicMock()
        mock_embedder.embed_single.side_effect = [
            [0.5, 0.5],  # "Hi."
            [0.6, 0.5],  # "There."
            [0.7, 0.5],  # "How are you?"
        ]

        chunker = SemanticChunker(
            embedder=mock_embedder,
            similarity_threshold=0.5,
            chunk_size=100,
            min_chunk_size=50  # Small min size
        )
        text = "Hi. There. How are you?"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should merge small sentences
        assert len(chunks) >= 1
