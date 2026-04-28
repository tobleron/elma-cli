"""Recursive character-based chunker with separator priority."""

from typing import List, Dict, Any

from .base import TextChunker
from ._registry import register_chunker
from observability import get_logger


class RecursiveChunker(TextChunker):
    """Recursively tries different separators until chunk size constraint is met.

    Separators are tried in priority order. If a split produces chunks
    larger than chunk_size, the next separator in the list is tried.
    """

    def __init__(
        self,
        chunk_size: int = 512,
        overlap: int = 50,
        separators: List[str] = None,
        min_chunk_size: int = 50
    ):
        """Initialize RecursiveChunker.

        Args:
            chunk_size: Maximum characters per chunk
            overlap: Character overlap between adjacent chunks
            separators: List of separators in priority order (tried sequentially)
            min_chunk_size: Minimum chunk size before merging
        """
        self.chunk_size = chunk_size
        self.overlap = overlap
        self.separators = separators or ["\n\n", "\n", ". ", " "]
        self.min_chunk_size = min_chunk_size
        self.logger = get_logger(__name__)

    def name(self) -> str:
        return "recursive"

    def _split_by_separator(self, text: str, separator: str) -> List[str]:
        """Split text by a single separator, preserving the separator in chunks."""
        if not separator or separator not in text:
            return [text] if text else []

        parts = text.split(separator)
        result = []
        for i, part in enumerate(parts):
            if i < len(parts) - 1:
                result.append(part + separator)
            else:
                if part:
                    result.append(part)
        return result

    def _recursive_split(
        self,
        chunks: List[str],
        separator_index: int,
        source: str
    ) -> List[Dict[str, Any]]:
        """Recursively split chunks until all are <= chunk_size."""
        if separator_index >= len(self.separators):
            # Final fallback: just return fixed-size chunks
            # Use safe overlap (must be less than chunk_size)
            fallback_overlap = min(self.overlap, self.chunk_size - 1) if self.chunk_size > 1 else 0
            fixed = FixedSizeChunker(self.chunk_size, fallback_overlap)
            result = []
            for i, chunk in enumerate(chunks):
                sub_chunks = fixed.chunk_with_metadata(chunk, source)
                for sc in sub_chunks:
                    sc["metadata"]["separator_used"] = "char"
                    sc["metadata"]["split_level"] = len(self.separators)
                    result.append(sc)
            return result

        separator = self.separators[separator_index]
        needs_split = [c for c in chunks if len(c) > self.chunk_size]

        if not needs_split:
            # All chunks are small enough
            return [{
                "text": c,
                "chunk_index": i,
                "source": source,
                "metadata": {"separator_used": separator, "split_level": separator_index}
            } for i, c in enumerate(chunks)]

        # Split chunks that are too large
        new_chunks = []
        for chunk in chunks:
            if len(chunk) > self.chunk_size:
                parts = self._split_by_separator(chunk, separator)
                new_chunks.extend(parts)
            else:
                new_chunks.append(chunk)

        # Recurse with next separator
        return self._recursive_split(new_chunks, separator_index + 1, source)

    def chunk_with_metadata(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Split text recursively using separator priority."""
        self.logger.debug("Chunking text",
                        strategy="recursive",
                        text_length=len(text),
                        source=source)

        if not text or not text.strip():
            self.logger.warning("Empty text provided for chunking")
            return []

        chunks = self._recursive_split([text], 0, source)

        # Apply overlap by extending each chunk with start of next
        if self.overlap > 0 and len(chunks) > 1:
            chunks = self._apply_overlap(chunks)

        # Re-index after overlap adjustment
        for i, chunk in enumerate(chunks):
            chunk["chunk_index"] = i

        return chunks

    def _apply_overlap(self, chunks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Add overlap between adjacent chunks."""
        if len(chunks) <= 1:
            return chunks

        result = [chunks[0]]
        for i in range(1, len(chunks)):
            prev = result[-1]
            curr = chunks[i]

            # Extend previous chunk with start of current
            overlap_text = curr["text"][:self.overlap]
            extended_text = prev["text"] + overlap_text

            result[-1] = {
                **prev,
                "text": extended_text,
                "metadata": {**prev["metadata"], "overlap_applied": self.overlap}
            }
            result.append(curr)

        return result

    def preview(self, text: str, **kwargs) -> Dict[str, Any]:
        """Preview chunking results with statistics."""
        chunks = self.chunk_with_metadata(text, "preview")

        if not chunks:
            return {
                "chunks": [],
                "stats": {"total_chunks": 0, "avg_size": 0, "min_size": 0, "max_size": 0},
                "params": {
                    "chunk_size": self.chunk_size,
                    "overlap": self.overlap,
                    "separators": self.separators
                }
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
            "params": {
                "chunk_size": self.chunk_size,
                "overlap": self.overlap,
                "separators": self.separators
            }
        }


# Import FixedSizeChunker for fallback
from .fixed_size_chunker import FixedSizeChunker

register_chunker("recursive", RecursiveChunker)
