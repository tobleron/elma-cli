"""Tests for FixedSizeChunker."""

import pytest
from src.chunkers.fixed_size_chunker import FixedSizeChunker


class TestFixedSizeChunker:
    def test_name(self):
        chunker = FixedSizeChunker(chunk_size=512, overlap=50)
        assert chunker.name() == "fixed"

    def test_chunk_with_metadata_basic(self):
        chunker = FixedSizeChunker(chunk_size=10, overlap=2)
        text = "Hello World Test"  # 16 chars
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # step = 10 - 2 = 8
        # Chunk 0: positions 0-10 = "Hello Worl" (10 chars)
        # Chunk 1: positions 8-16 = "d World Te" (8 chars) - but algorithm truncates at end
        # Actually: start=0 → end=10 → "Hello Worl", start=8 → end=16 → "d World Te"
        # With correct step logic: only 2 chunks
        assert len(chunks) == 2
        assert chunks[0]["text"] == "Hello Worl"
        assert chunks[1]["text"] == "rld Test"  # 8 chars due to overlap truncation
        assert chunks[0]["chunk_index"] == 0
        assert chunks[0]["source"] == "test.txt"
        assert "metadata" in chunks[0]

    def test_short_text_returns_single_chunk(self):
        chunker = FixedSizeChunker(chunk_size=100, overlap=10)
        text = "Short text"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 1
        assert chunks[0]["text"] == "Short text"

    def test_empty_text_returns_empty_list(self):
        chunker = FixedSizeChunker(chunk_size=512, overlap=50)
        chunks = chunker.chunk_with_metadata("", "test.txt")

        assert chunks == []

    def test_preview(self):
        chunker = FixedSizeChunker(chunk_size=10, overlap=2)
        text = "Hello World Test More"  # 21 chars
        result = chunker.preview(text)

        assert "chunks" in result
        assert "stats" in result
        assert "params" in result
        assert result["stats"]["total_chunks"] == 3  # 0, 8, 16 positions
        assert result["stats"]["min_size"] == 5  # last chunk is only 5 chars
        assert result["stats"]["max_size"] == 10  # first chunks are 10 chars