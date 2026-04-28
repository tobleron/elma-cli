"""LLM-based chunker - uses LLM to identify semantic boundaries."""

import json
import re
from typing import List, Dict, Any, Optional

from .base import TextChunker
from ._registry import register_chunker
from observability import get_logger


# Default prompt for LLM chunking
CHUNKING_PROMPT = """You are a document chunking assistant. Given a section of text, identify
the natural semantic boundaries where the document should be split into chunks.

Each chunk should:
- Represent a complete thought or idea
- Be self-contained and understandable on its own
- Not exceed {chunk_size} characters

Return your analysis as JSON with the following format:
{{
  "chunks": [
    {{"start": 0, "end": 150, "reason": "complete introduction"}},
    {{"start": 150, "end": 350, "reason": "main topic explained"}}
  ]
}}

IMPORTANT: The "start" and "end" are CHARACTER positions in the original text.
Only include chunks that are truly complete thoughts. If the text is short and
forms a single complete idea, return just one chunk.

Text to analyze:
{text}
"""


class LLMChunker(TextChunker):
    """Uses LLM to identify semantically complete chunks.

    This approach leverages the LLM's understanding of language to identify
    natural chunk boundaries based on semantic completeness rather than
    arbitrary size or structure.

    Note: This is computationally expensive as it requires LLM calls.
    """

    def __init__(
        self,
        chunk_size: int = 512,
        overlap: int = 50,
        llm_client: Optional[Any] = None,
        prompt: str = CHUNKING_PROMPT,
        max_llm_chunk_size: int = 2000,
        fallback_to_sentence: bool = True
    ):
        """Initialize LLMChunker.

        Args:
            chunk_size: Maximum characters per chunk
            overlap: Character overlap between adjacent chunks
            llm_client: LLM client with generate(prompt, context) method
            prompt: Custom prompt template for chunking
            max_llm_chunk_size: Maximum text size to send to LLM at once
            fallback_to_sentence: If True, fallback to sentence splitting if LLM fails
        """
        self.chunk_size = chunk_size
        self.overlap = overlap
        self.llm_client = llm_client
        self.prompt = prompt
        self.max_llm_chunk_size = max_llm_chunk_size
        self.fallback_to_sentence = fallback_to_sentence
        self.logger = get_logger(__name__)

    def name(self) -> str:
        return "llm"

    def _pre_chunk_text(self, text: str) -> List[str]:
        """Split text into chunks that fit within LLM context window."""
        if len(text) <= self.max_llm_chunk_size:
            return [text]

        # Split by paragraphs first (natural boundaries)
        paragraphs = text.split('\n\n')
        chunks = []
        current = ""

        for para in paragraphs:
            para = para.strip()
            if not para:
                continue

            trial = current + "\n\n" + para if current else para
            if len(trial) <= self.max_llm_chunk_size:
                current = trial
            else:
                if current:
                    chunks.append(current)
                # If single paragraph exceeds limit, split by sentences
                if len(para) > self.max_llm_chunk_size:
                    sentences = self._split_by_sentences(para)
                    for sent in sentences:
                        if len(sent) <= self.max_llm_chunk_size:
                            current = sent if not current else current + "\n\n" + sent
                            if len(current) > self.max_llm_chunk_size:
                                chunks.append(current[:-len(sent)])
                                current = sent
                        else:
                            if current:
                                chunks.append(current)
                                current = ""
                            # Split long sentence by words
                            chunks.extend(self._split_long_sentence(sent))
                else:
                    current = para

        if current:
            chunks.append(current)

        return chunks if chunks else [text]

    def _split_by_sentences(self, text: str) -> List[str]:
        """Split text by sentence boundaries."""
        sentence_pattern = r'[^.!?]+[.!?]+(?:\s|$)'
        parts = re.findall(sentence_pattern, text)
        if parts:
            return [p.strip() for p in parts if p.strip()]
        return [text]

    def _split_long_sentence(self, text: str) -> List[str]:
        """Split a very long sentence into smaller pieces."""
        words = text.split()
        chunks = []
        current = ""
        for word in words:
            trial = current + " " + word if current else word
            if len(trial) <= self.max_llm_chunk_size:
                current = trial
            else:
                if current:
                    chunks.append(current)
                current = word
        if current:
            chunks.append(current)
        return chunks

    def _call_llm(self, text: str) -> List[Dict[str, Any]]:
        """Call LLM to get chunk boundaries.

        Returns:
            List of chunk specs with start, end, reason
        """
        if not self.llm_client:
            return []

        prompt = self.prompt.format(chunk_size=self.chunk_size, text=text)

        try:
            response = self.llm_client.generate(prompt, [])
            return self._parse_llm_response(response)
        except Exception:
            return []

    def _parse_llm_response(self, response: str) -> List[Dict[str, Any]]:
        """Parse LLM JSON response into chunk specifications."""
        try:
            # Try to extract JSON from response
            json_match = re.search(r'\{[\s\S]*\}', response)
            if json_match:
                data = json.loads(json_match.group())
                if "chunks" in data:
                    return data["chunks"]
        except (json.JSONDecodeError, KeyError):
            pass
        return []

    def _apply_chunk_boundaries(self, text: str, boundaries: List[Dict[str, Any]], source: str) -> List[Dict[str, Any]]:
        """Apply LLM-suggested boundaries to create chunks."""
        if not boundaries:
            if self.fallback_to_sentence:
                return self._fallback_chunk_by_sentences(text, source)
            return [{
                "text": text[:self.chunk_size],
                "chunk_index": 0,
                "source": source,
                "metadata": {"llm_fallback": True}
            }]

        chunks = []
        for i, boundary in enumerate(boundaries):
            try:
                start = max(0, int(boundary.get("start", 0)))
                end = min(len(text), int(boundary.get("end", self.chunk_size)))
                reason = boundary.get("reason", "unknown")

                if end > start:
                    chunk_text = text[start:end]
                    chunks.append({
                        "text": chunk_text,
                        "chunk_index": len(chunks),
                        "source": source,
                        "metadata": {
                            "llm_generated": True,
                            "reason": reason,
                            "start": start,
                            "end": end
                        }
                    })
            except (ValueError, TypeError):
                continue

        return chunks if chunks else self._fallback_chunk_by_sentences(text, source)

    def _fallback_chunk_by_sentences(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Fallback: chunk by sentences when LLM is unavailable or fails."""
        sentences = self._split_by_sentences(text)

        chunks = []
        current_text = ""
        current_len = 0

        for sentence in sentences:
            sentence_len = len(sentence)
            trial = current_text + " " + sentence if current_text else sentence
            trial_len = len(trial)

            if not current_text:
                current_text = sentence
                current_len = sentence_len
            elif current_len + sentence_len + 1 <= self.chunk_size:
                current_text = trial
                current_len = trial_len
            else:
                chunks.append({
                    "text": current_text.strip(),
                    "chunk_index": len(chunks),
                    "source": source,
                    "metadata": {"fallback": "sentence", "chunk_size": self.chunk_size}
                })
                current_text = sentence
                current_len = sentence_len

        if current_text.strip():
            chunks.append({
                "text": current_text.strip(),
                "chunk_index": len(chunks),
                "source": source,
                "metadata": {"fallback": "sentence", "chunk_size": self.chunk_size}
            })

        # Apply overlap
        if self.overlap > 0 and len(chunks) > 1:
            chunks = self._apply_overlap(chunks)

        return chunks

    def _apply_overlap(self, chunks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Add overlap between adjacent chunks."""
        if self.overlap <= 0 or len(chunks) <= 1:
            return chunks

        result = [chunks[0]]

        for i in range(1, len(chunks)):
            curr = chunks[i]
            prev = result[-1]

            overlap_text = curr["text"][:self.overlap]
            extended_text = prev["text"] + overlap_text

            result[-1] = {
                **prev,
                "text": extended_text,
                "metadata": {**prev.get("metadata", {}), "overlap_applied": self.overlap}
            }
            result.append(curr)

        return result

    def _enforce_chunk_size_limit(self, chunks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Ensure no chunk exceeds chunk_size limit."""
        result = []

        for chunk in chunks:
            text = chunk["text"]
            if len(text) <= self.chunk_size:
                result.append(chunk)
            else:
                # Split large chunk
                sub_chunks = self._split_text_fixed(text)
                for i, sub_text in enumerate(sub_chunks):
                    result.append({
                        "text": sub_text,
                        "chunk_index": len(result),
                        "source": chunk["source"],
                        "metadata": {
                            **chunk.get("metadata", {}),
                            "split": True,
                            "sub_chunk": i
                        }
                    })

        return result

    def _split_text_fixed(self, text: str) -> List[str]:
        """Split text by fixed size."""
        if len(text) <= self.chunk_size:
            return [text]

        chunks = []
        start = 0
        step = self.chunk_size - self.overlap

        while start < len(text):
            end = min(start + self.chunk_size, len(text))
            chunks.append(text[start:end])
            start += step

        return chunks if chunks else [text]

    def chunk_with_metadata(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Split text using LLM to identify semantic boundaries.

        Process:
        1. Pre-chunk text to fit LLM context
        2. For each pre-chunk, call LLM to get boundaries
        3. Apply boundaries to create chunks
        4. Enforce chunk_size limits
        """
        self.logger.debug("Chunking text",
                        strategy="llm",
                        text_length=len(text),
                        source=source)

        if not text or not text.strip():
            self.logger.warning("Empty text provided for chunking")
            return []

        # Step 1: Pre-chunk for LLM
        pre_chunks = self._pre_chunk_text(text)

        all_chunks = []

        for pre_text in pre_chunks:
            # Step 2: Get boundaries from LLM
            boundaries = self._call_llm(pre_text)

            # Step 3: Apply boundaries
            offset = text.index(pre_text) if pre_text in text else 0
            adjusted_boundaries = []
            for b in boundaries:
                try:
                    adjusted_boundaries.append({
                        "start": b["start"] + offset,
                        "end": b["end"] + offset,
                        "reason": b.get("reason", "")
                    })
                except (KeyError, TypeError):
                    pass

            chunks = self._apply_chunk_boundaries(pre_text, adjusted_boundaries if adjusted_boundaries else [], source)
            all_chunks.extend(chunks)

        # Step 4: Enforce chunk_size limit
        result = self._enforce_chunk_size_limit(all_chunks)

        # Re-index
        for i, chunk in enumerate(result):
            chunk["chunk_index"] = i

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
                    "llm_client": bool(self.llm_client)
                }
            }

        sizes = [len(c["text"]) for c in chunks]
        return {
            "chunks": [
                {
                    "index": i,
                    "text": c["text"][:100] + "..." if len(c["text"]) > 100 else c["text"],
                    "size": len(c["text"])
                }
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
                "llm_client": bool(self.llm_client)
            }
        }


register_chunker("llm", LLMChunker)
