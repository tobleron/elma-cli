"""Text chunking module for Local RAG.

This module provides backwards compatibility by wrapping the new
chunkers package with the original TextChunker interface.
"""

from typing import List, Dict, Any
from dataclasses import dataclass

from chunkers import create_chunker


@dataclass
class Chunk:
    """Represents a text chunk with metadata."""

    text: str
    chunk_index: int
    source_file: str


class TextChunker:
    """Splits text documents into overlapping chunks for embedding.

    This class is maintained for backwards compatibility.
    New code should use src.chunkers.create_chunker() directly.
    """

    def __init__(self, chunk_size: int = 512, chunk_overlap: int = 50, strategy: str = "fixed", **kwargs):
        """Initialize the chunker.

        Args:
            chunk_size: Target size of each chunk in characters.
            chunk_overlap: Number of overlapping characters between adjacent chunks.
            strategy: Chunking strategy name ('fixed', 'recursive', 'structure', 'semantic')
            **kwargs: Additional strategy-specific parameters
        """
        self.chunk_size = chunk_size
        self.chunk_overlap = chunk_overlap
        self.strategy = strategy

        # Create underlying chunker from the new chunkers package
        self._chunker = create_chunker(
            strategy,
            chunk_size=chunk_size,
            overlap=chunk_overlap,
            **kwargs
        )

    def chunk_by_characters(
        self, text: str, source_file: str = "unknown"
    ) -> List[Chunk]:
        """Split text into overlapping chunks by character count.

        Args:
            text: The text content to chunk.
            source_file: Name of the source file for metadata.

        Returns:
            List of Chunk objects with text and metadata.
        """
        chunk_dicts = self._chunker.chunk_with_metadata(text, source_file)

        return [
            Chunk(
                text=d["text"],
                chunk_index=d["chunk_index"],
                source_file=d["source"]
            )
            for d in chunk_dicts
        ]

    def chunk_with_metadata(
        self, text: str, source_file: str = "unknown"
    ) -> List[Dict[str, Any]]:
        """Split text and return chunks with full metadata.

        Args:
            text: The text content to chunk.
            source_file: Name of the source file for metadata.

        Returns:
            List of dictionaries containing chunk text and metadata.
        """
        chunks = self._chunker.chunk_with_metadata(text, source_file)
        # Convert "source" to "source_file" for backwards compatibility
        return [
            {**chunk, "source_file": chunk.pop("source")} if "source" in chunk else chunk
            for chunk in chunks
        ]