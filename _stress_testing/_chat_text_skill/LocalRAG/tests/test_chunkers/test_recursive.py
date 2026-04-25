"""Tests for RecursiveChunker."""

import pytest
from src.chunkers.recursive_chunker import RecursiveChunker


class TestRecursiveChunker:
    def test_name(self):
        chunker = RecursiveChunker()
        assert chunker.name() == "recursive"

    def test_respects_separator_priority(self):
        chunker = RecursiveChunker(chunk_size=30, separators=["\n\n", "\n", " "])
        text = "Hello world.\n\nThis is a test.\n\nMore text here."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should split on \n\n first (paragraph level)
        assert len(chunks) > 0

    def test_handles_text_shorter_than_chunk_size(self):
        chunker = RecursiveChunker(chunk_size=100)
        text = "Short text"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 1
        assert chunks[0]["text"] == "Short text"

    def test_preview(self):
        chunker = RecursiveChunker(chunk_size=20)
        text = "Hello world. This is a longer text that should be split."
        result = chunker.preview(text)

        assert "chunks" in result
        assert "stats" in result
        assert result["stats"]["total_chunks"] >= 1
