"""Fixed size character-based chunker."""

from typing import List, Dict, Any

from .base import TextChunker
from ._registry import register_chunker
from observability import get_logger, traced


class FixedSizeChunker(TextChunker):
    """Splits text by character count with overlap.

    This is the simplest chunking strategy - splits text into
    fixed-size chunks with overlapping character regions.
    """

    def __init__(self, chunk_size: int = 512, overlap: int = 50):
        """Initialize FixedSizeChunker.

        Args:
            chunk_size: Maximum characters per chunk
            overlap: Character overlap between adjacent chunks
        """
        if overlap >= chunk_size:
            raise ValueError(f"overlap ({overlap}) must be less than chunk_size ({chunk_size})")
        self.chunk_size = chunk_size
        self.overlap = overlap
        self.logger = get_logger(__name__)

    def name(self) -> str:
        return "fixed"

    def chunk_with_metadata(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Split text into overlapping fixed-size chunks."""
        self.logger.debug("Chunking text",
                        strategy="fixed",
                        text_length=len(text),
                        source=source)

        if not text or not text.strip():
            self.logger.warning("Empty text provided for chunking")
            return []

        if len(text) <= self.chunk_size:
            return [{
                "text": text,
                "chunk_index": 0,
                "source": source,
                "metadata": {"start_char": 0, "end_char": len(text)}
            }]

        chunks = []
        step = self.chunk_size - self.overlap
        start = 0
        chunk_index = 0

        while start < len(text):
            end = min(start + self.chunk_size, len(text))
            chunk_text = text[start:end]

            chunks.append({
                "text": chunk_text,
                "chunk_index": chunk_index,
                "source": source,
                "metadata": {"start_char": start, "end_char": end}
            })

            chunk_index += 1
            start += step

        return chunks

    def preview(self, text: str, **kwargs) -> Dict[str, Any]:
        """Preview chunking results with statistics."""
        chunks = self.chunk_with_metadata(text, "preview")

        if not chunks:
            return {
                "chunks": [],
                "stats": {"total_chunks": 0, "avg_size": 0, "min_size": 0, "max_size": 0},
                "params": {"chunk_size": self.chunk_size, "overlap": self.overlap}
            }

        sizes = [len(c["text"]) for c in chunks]
        return {
            "chunks": [
                {"index": i, "text": c["text"], "size": len(c["text"])}
                for i, c in enumerate(chunks)
            ],
            "stats": {
                "total_chunks": len(chunks),
                "avg_size": sum(sizes) // len(sizes),
                "min_size": min(sizes),
                "max_size": max(sizes)
            },
            "params": {"chunk_size": self.chunk_size, "overlap": self.overlap}
        }


register_chunker("fixed", FixedSizeChunker)