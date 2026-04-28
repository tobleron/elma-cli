"""Tests for TextChunker module."""

import pytest
from src.chunker import TextChunker, Chunk


class TestTextChunker:
    """Test cases for TextChunker class."""

    def setup_method(self):
        """Set up test fixtures."""
        self.chunker = TextChunker(chunk_size=100, chunk_overlap=20)

    def test_short_text_returns_single_chunk(self):
        """Test that short text returns single chunk."""
        text = "Short text"
        chunks = self.chunker.chunk_by_characters(text)
        assert len(chunks) == 1
        assert chunks[0].text == text
        assert chunks[0].chunk_index == 0

    def test_empty_text_returns_empty_list(self):
        """Test that empty text returns empty list."""
        chunks = self.chunker.chunk_by_characters("")
        assert len(chunks) == 0

    def test_chunk_by_characters_basic(self):
        """Test basic chunking by characters with overlap.

        With chunk_size=100, chunk_overlap=20, step=80.
        text="A"*250:
        - chunk[0]: text[0:100] = 100 chars
        - chunk[1]: text[80:180] = 100 chars (overlap 20 with chunk[0])
        - chunk[2]: text[160:250] = 90 chars (partial)
        - chunk[3]: text[240:250] = 10 chars (partial)
        """
        text = "A" * 250
        chunks = self.chunker.chunk_by_characters(text)
        assert len(chunks) == 4
        assert chunks[0].text == "A" * 100
        assert chunks[1].text == "A" * 100
        assert chunks[2].text == "A" * 90
        assert chunks[3].text == "A" * 10

    def test_chunk_metadata(self):
        """Test that chunks have correct metadata."""
        text = "Hello World!"
        source = "test.txt"
        chunks = self.chunker.chunk_by_characters(text, source)
        assert chunks[0].source_file == source
        assert chunks[0].chunk_index == 0

    def test_chunk_with_metadata_returns_dicts(self):
        """Test chunk_with_metadata returns proper dictionaries."""
        text = "Test content here"
        source = "doc.txt"
        chunks = self.chunker.chunk_with_metadata(text, source)
        assert len(chunks) == 1
        assert "text" in chunks[0]
        assert "chunk_index" in chunks[0]
        assert "source_file" in chunks[0]

    def test_custom_chunk_size(self):
        """Test chunker with custom chunk size.

        With chunk_size=50, chunk_overlap=10, step=40.
        text="X"*120:
        - chunk[0]: text[0:50] = 50 chars
        - chunk[1]: text[40:90] = 50 chars (overlap 10 with chunk[0])
        - chunk[2]: text[80:120] = 40 chars (partial)
        """
        chunker = TextChunker(chunk_size=50, chunk_overlap=10)
        text = "X" * 120
        chunks = chunker.chunk_by_characters(text)
        assert len(chunks) == 3
        assert chunks[0].text == "X" * 50
        assert chunks[1].text == "X" * 50
        assert chunks[2].text == "X" * 40
