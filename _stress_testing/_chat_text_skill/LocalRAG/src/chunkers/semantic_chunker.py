"""Semantic chunker - splits text by natural segments then merges by embedding similarity."""

import re
from typing import List, Dict, Any, Optional
import math

from .base import TextChunker
from ._registry import register_chunker
from observability import get_logger


def cosine_similarity(a: List[float], b: List[float]) -> float:
    """Calculate cosine similarity between two vectors."""
    dot_product = sum(x * y for x, y in zip(a, b))
    norm_a = math.sqrt(sum(x * x for x in a))
    norm_b = math.sqrt(sum(x * x for x in b))

    if norm_a == 0 or norm_b == 0:
        return 0.0

    return dot_product / (norm_a * norm_b)


class SemanticChunker(TextChunker):
    """Semantic chunking based on embedding similarity.

    Approach (from article):
    1. Segment document by natural units (sentences, paragraphs)
    2. Create embeddings for each segment
    3. If adjacent segments have high similarity -> merge
    4. If merged chunk exceeds chunk_size -> start new chunk
    """

    def __init__(
        self,
        chunk_size: int = 512,
        overlap: int = 50,
        similarity_threshold: float = 0.7,
        min_chunk_size: int = 50,
        embedder: Optional[Any] = None
    ):
        """Initialize SemanticChunker.

        Args:
            chunk_size: Maximum characters per chunk
            overlap: Character overlap between adjacent chunks
            similarity_threshold: Cosine similarity threshold for merging (0.0-1.0)
            min_chunk_size: Minimum chunk size before forcing split
            embedder: Embedder instance with embed_single(text) method
        """
        if overlap >= chunk_size:
            raise ValueError(f"overlap ({overlap}) must be less than chunk_size ({chunk_size})")
        self.chunk_size = chunk_size
        self.overlap = overlap
        self.similarity_threshold = similarity_threshold
        self.min_chunk_size = min_chunk_size
        self.embedder = embedder
        self.logger = get_logger(__name__)

    def name(self) -> str:
        return "semantic"

    def _split_into_sentences(self, text: str) -> List[str]:
        """Split text into sentences, respecting natural boundaries."""
        # First split by paragraphs
        paragraphs = text.split('\n\n')

        sentences = []
        for para in paragraphs:
            para = para.strip()
            if not para:
                continue

            # Split paragraph by sentence endings
            # Look for . ! ? followed by space or end
            sentence_pattern = r'[^.!?]+[.!?]+(?:\s|$)'

            parts = re.findall(sentence_pattern, para)
            if parts:
                # Found sentences
                for part in parts:
                    part = part.strip()
                    if part:
                        sentences.append(part)
            else:
                # No sentence boundaries found
                if len(para) <= self.chunk_size:
                    sentences.append(para)
                else:
                    # Split by words for long text without sentence boundaries
                    words = para.split()
                    current = ""
                    for word in words:
                        trial = current + " " + word if current else word
                        if len(trial) <= self.chunk_size:
                            current = trial
                        else:
                            if current:
                                sentences.append(current)
                            current = word
                    if current:
                        sentences.append(current)

        return sentences

    def _get_embedding(self, text: str) -> Optional[List[float]]:
        """Get embedding for text, return None if embedder unavailable."""
        if not self.embedder:
            return None
        try:
            return self.embedder.embed_single(text)
        except Exception:
            return None

    def _compute_similarity(self, emb1: Optional[List[float]], emb2: Optional[List[float]]) -> float:
        """Compute similarity between two embeddings."""
        if emb1 is None or emb2 is None:
            return 1.0  # Assume similar if no embedder
        return cosine_similarity(emb1, emb2)

    def _merge_by_similarity(self, segments: List[str]) -> List[str]:
        """Merge adjacent segments if similarity is high.

        Returns list of merged chunks (text strings).
        """
        if len(segments) <= 1:
            return segments

        # Get embeddings for all segments
        embeddings = [self._get_embedding(seg) for seg in segments]

        # Merge adjacent segments with high similarity
        merged = [segments[0]]

        for i in range(1, len(segments)):
            current = segments[i]
            prev = merged[-1]

            # Compute similarity between previous and current
            similarity = self._compute_similarity(
                embeddings[i - 1] if i - 1 < len(embeddings) else None,
                embeddings[i] if i < len(embeddings) else None
            )

            # If similarity is high, merge
            if similarity >= self.similarity_threshold:
                trial = prev + " " + current
                # But don't merge if it would exceed chunk_size
                if len(trial) <= self.chunk_size:
                    merged[-1] = trial
                else:
                    # Start a new chunk
                    merged.append(current)
            else:
                # Low similarity - start new chunk
                merged.append(current)

        return merged

    def _split_by_sentence_boundary(self, text: str) -> List[str]:
        """Split text by sentences, respecting sentence boundaries."""
        # Find sentence endings
        sentence_endings = r'(?<=[.!?])\s+'

        parts = re.split(sentence_endings, text)
        return [p.strip() for p in parts if p.strip()]

    def _create_chunks(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Create chunks by semantic similarity.

        Algorithm:
        1. Split into sentences
        2. Get embeddings for each sentence
        3. Merge sentences with high similarity into chunks
        4. Handle chunks that exceed chunk_size
        """
        if not text or not text.strip():
            return []

        # Step 1: Split into sentences/paragraphs
        segments = self._split_into_sentences(text)

        if not segments:
            return []

        # Step 2: If no embedder, fall back to simple sentence splitting
        if not self.embedder:
            return self._create_simple_chunks(segments, source)

        # Step 3: Merge by similarity
        merged_segments = self._merge_by_similarity(segments)

        # Step 4: Convert to final chunks
        chunks = []
        for i, segment in enumerate(merged_segments):
            # If segment still too large, split it
            if len(segment) > self.chunk_size:
                sub_chunks = self._split_large_segment(segment)
                for j, sub in enumerate(sub_chunks):
                    chunks.append({
                        "text": sub,
                        "chunk_index": len(chunks),
                        "source": source,
                        "metadata": {
                            "segment_index": i,
                            "sub_chunk_index": j,
                            "is_split": True
                        }
                    })
            else:
                chunks.append({
                    "text": segment,
                    "chunk_index": len(chunks),
                    "source": source,
                    "metadata": {
                        "segment_index": i,
                        "is_split": False
                    }
                })

        # Step 5: Apply overlap between chunks
        if self.overlap > 0 and len(chunks) > 1:
            chunks = self._apply_overlap(chunks)

        return chunks

    def _create_simple_chunks(self, segments: List[str], source: str) -> List[Dict[str, Any]]:
        """Create chunks without using embeddings (fallback mode)."""
        chunks = []
        current_text = ""
        current_len = 0

        for segment in segments:
            segment_len = len(segment)

            if not current_text:
                current_text = segment
                current_len = segment_len
            elif current_len + segment_len + 1 <= self.chunk_size:
                # Add to current chunk
                current_text = current_text + " " + segment
                current_len = len(current_text)
            else:
                # Save current chunk and start new one
                chunks.append({
                    "text": current_text,
                    "chunk_index": len(chunks),
                    "source": source,
                    "metadata": {"fallback": True}
                })
                current_text = segment
                current_len = segment_len

        # Don't forget the last chunk
        if current_text:
            chunks.append({
                "text": current_text,
                "chunk_index": len(chunks),
                "source": source,
                "metadata": {"fallback": True}
            })

        # Apply overlap
        if self.overlap > 0 and len(chunks) > 1:
            chunks = self._apply_overlap(chunks)

        return chunks

    def _split_large_segment(self, text: str) -> List[str]:
        """Split a large segment into smaller chunks by sentences."""
        sentences = self._split_by_sentence_boundary(text)
        result = []
        current = ""

        for sentence in sentences:
            trial = current + " " + sentence if current else sentence
            if len(trial) <= self.chunk_size:
                current = trial
            else:
                if current:
                    result.append(current)
                # Start new chunk, but if single sentence exceeds limit, force split
                if not current:
                    # Long sentence - split by words
                    words = sentence.split()
                    current = ""
                    for word in words:
                        trial = current + " " + word if current else word
                        if len(trial) <= self.chunk_size:
                            current = trial
                        else:
                            if current:
                                result.append(current)
                            current = word
                current = sentence

        if current:
            result.append(current)

        return result if result else [text[:self.chunk_size]]

    def _apply_overlap(self, chunks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Add overlap between adjacent chunks."""
        if self.overlap <= 0 or len(chunks) <= 1:
            return chunks

        result = [chunks[0]]

        for i in range(1, len(chunks)):
            curr = chunks[i]
            prev = result[-1]

            # Get overlap text from start of current chunk
            overlap_text = curr["text"][:self.overlap]

            # Extend previous chunk with overlap
            extended_text = prev["text"] + overlap_text
            result[-1] = {
                **prev,
                "text": extended_text,
                "metadata": {**prev.get("metadata", {}), "overlap_applied": self.overlap}
            }
            result.append(curr)

        return result

    def chunk_with_metadata(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Split text using semantic boundaries based on embedding similarity."""
        self.logger.debug("Chunking text",
                        strategy="semantic",
                        text_length=len(text),
                        source=source)

        if not text or not text.strip():
            self.logger.warning("Empty text provided for chunking")
            return []

        return self._create_chunks(text, source)

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
                    "similarity_threshold": self.similarity_threshold,
                    "min_chunk_size": self.min_chunk_size
                }
            }

        sizes = [len(c["text"]) for c in chunks]
        return {
            "chunks": [
                {"index": i, "text": c["text"][:100] + "..." if len(c["text"]) > 100 else c["text"], "size": len(c["text"])}
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
                "similarity_threshold": self.similarity_threshold,
                "min_chunk_size": self.min_chunk_size
            }
        }


register_chunker("semantic", SemanticChunker)
