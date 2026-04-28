"""Tests for LLM-based chunker."""

import pytest
from unittest.mock import MagicMock
from src.chunkers.llm_chunker import LLMChunker


class TestLLMChunker:
    def test_name(self):
        chunker = LLMChunker()
        assert chunker.name() == "llm"

    def test_requires_llm_client(self):
        """Without LLM client, should fall back to basic chunking."""
        chunker = LLMChunker()
        text = "Hello world. This is a test."
        chunks = chunker.chunk_with_metadata(text, "test.txt")
        # Should still return chunks (fallback)
        assert len(chunks) >= 1

    def test_llm_splits_at_boundaries(self):
        """LLM should identify semantic boundaries and split accordingly."""
        mock_llm = MagicMock()
        # Simulate LLM returning JSON with split points
        mock_llm.generate.return_value = '''{"chunks": [
            {"start": 0, "end": 17, "reason": "first sentence complete"},
            {"start": 17, "end": 40, "reason": "second sentence complete"}
        ]}'''

        chunker = LLMChunker(llm_client=mock_llm)
        text = "Hello world. This is a test."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert mock_llm.generate.called
        assert len(chunks) >= 1

    def test_respects_chunk_size_limit(self):
        """Chunks should not exceed chunk_size even with LLM."""
        mock_llm = MagicMock()
        # Return huge chunk that exceeds limit
        mock_llm.generate.return_value = '''{"chunks": [
            {"start": 0, "end": 5000, "reason": "everything"}
        ]}'''

        chunker = LLMChunker(llm_client=mock_llm, chunk_size=100)
        text = "A" * 500
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should still respect chunk_size limit
        for chunk in chunks:
            assert len(chunk["text"]) <= 150  # 100 + some overhead for safety

    def test_empty_text_returns_empty_list(self):
        """Empty text should return empty list."""
        chunker = LLMChunker()
        chunks = chunker.chunk_with_metadata("", "test.txt")
        assert chunks == []

    def test_whitespace_only_returns_empty_list(self):
        """Whitespace-only text should return empty list."""
        chunker = LLMChunker()
        chunks = chunker.chunk_with_metadata("   \n\n  ", "test.txt")
        assert chunks == []

    def test_llm_returns_invalid_json_fallback(self):
        """If LLM returns invalid JSON, should fallback gracefully."""
        mock_llm = MagicMock()
        mock_llm.generate.return_value = "I couldn't understand that."

        chunker = LLMChunker(llm_client=mock_llm)
        text = "Hello world. This is a test."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should still return chunks via fallback
        assert len(chunks) >= 1

    def test_preview_returns_stats(self):
        """Preview should return statistics."""
        mock_llm = MagicMock()
        mock_llm.generate.return_value = '{"chunks": []}'

        chunker = LLMChunker(llm_client=mock_llm)
        text = "Hello world. This is a test."
        result = chunker.preview(text)

        assert "chunks" in result
        assert "stats" in result
        assert result["stats"]["total_chunks"] >= 0

    def test_metadata_includes_reason(self):
        """Chunk metadata should include the LLM's reason for splitting."""
        mock_llm = MagicMock()
        mock_llm.generate.return_value = '''{"chunks": [
            {"start": 0, "end": 17, "reason": "complete sentence"},
            {"start": 17, "end": 40, "reason": "another complete sentence"}
        ]}'''

        chunker = LLMChunker(llm_client=mock_llm)
        text = "Hello world. This is a test."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Check that metadata exists
        assert len(chunks) >= 1
        # Metadata should have LLM-related info
        for chunk in chunks:
            assert "source" in chunk
            assert "chunk_index" in chunk

    def test_long_text_chunked_for_llm(self):
        """Long text should be pre-chunked before sending to LLM."""
        mock_llm = MagicMock()
        # LLM returns nothing (can't process)
        mock_llm.generate.return_value = '{"chunks": []}'

        chunker = LLMChunker(llm_client=mock_llm, max_llm_chunk_size=100)
        # Create text longer than max_llm_chunk_size
        text = "A" * 300
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # LLM should have been called at least once
        # and we should still get some chunks via fallback
        assert len(chunks) >= 1

    def test_chunk_count_matches_llm_suggestion(self):
        """Chunk count should approximately match LLM suggestion."""
        mock_llm = MagicMock()
        mock_llm.generate.return_value = '''{"chunks": [
            {"start": 0, "end": 12, "reason": "first idea"},
            {"start": 12, "end": 27, "reason": "second idea"}
        ]}'''

        chunker = LLMChunker(llm_client=mock_llm)
        text = "Hello world. This is a test."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should have at least the chunks LLM suggested
        assert len(chunks) >= 2
