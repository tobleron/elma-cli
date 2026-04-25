"""Document structure-based chunker - heading + content as logical unit."""

import re
from typing import List, Dict, Any, Optional, Tuple

from .base import TextChunker
from ._registry import register_chunker
from observability import get_logger


class Section:
    """Represents a document section: heading + content."""
    def __init__(self, heading: str, level: int, content: str, start: int, end: int, heading_text: str = ""):
        self.heading = heading  # Formatted with markdown prefix
        self.heading_text = heading_text  # Plain text without prefix
        self.level = level
        self.content = content
        self.start = start
        self.end = end

    def format(self) -> str:
        """Format section as heading + content."""
        if self.heading:
            return f"{self.heading}\n\n{self.content}".strip()
        return self.content.strip()


class StructureChunker(TextChunker):
    """Splits text based on document structure - heading + content as a logical unit.

    Each chunk contains a heading and its associated content until the next
    heading of the same or higher level. This preserves the semantic relationship
    between a section's title and its body.
    """

    def __init__(
        self,
        chunk_size: int = 512,
        overlap: int = 50,
        split_on: str = "auto",
        **kwargs
    ):
        """Initialize StructureChunker.

        Args:
            chunk_size: Maximum characters per chunk
            overlap: Character overlap between adjacent chunks
            split_on: What to split on: "auto", "headings", "paragraphs", "lists", "code_blocks"
        """
        self.chunk_size = chunk_size
        self.overlap = overlap
        self.split_on = split_on
        self.logger = get_logger(__name__)

        valid_options = ["auto", "headings", "paragraphs", "lists", "code_blocks"]
        if split_on not in valid_options:
            raise ValueError(f"Invalid split_on: {split_on}. Must be one of: {valid_options}")

    def name(self) -> str:
        return "structure"

    def _find_all_headings(self, text: str) -> List[Tuple[int, int, str, int]]:
        """Find all headings with their position and level.

        Returns:
            List of (start_pos, end_pos, heading_text, level)
        """
        headings = []

        # Markdown headings: # to ######
        md_pattern = r'^(#{1,6})\s+(.+?)$'
        for match in re.finditer(md_pattern, text, re.MULTILINE):
            level = len(match.group(1))
            headings.append((match.start(), match.end(), match.group(2).strip(), level))

        # HTML headings: <h1> to <h6>
        html_pattern = r'<h([1-6])[^>]*>(.+?)</h[1-6]>'
        for match in re.finditer(html_pattern, text, re.IGNORECASE):
            level = int(match.group(1))
            headings.append((match.start(), match.end(), match.group(2).strip(), level))

        # Sort by position
        headings.sort(key=lambda x: x[0])
        return headings

    def _parse_sections(self, text: str) -> List[Section]:
        """Parse document into sections - each heading with its content.

        A section ends when the next heading of same or higher level appears.
        """
        headings = self._find_all_headings(text)

        if not headings:
            # No headings - entire text is one section
            return [Section(heading="", level=0, content=text.strip(), start=0, end=len(text))]

        sections = []

        for i, (start, end, heading_text, level) in enumerate(headings):
            # Content starts after the heading
            content_start = end

            # Content ends at the next heading (or end of text)
            if i < len(headings) - 1:
                content_end = headings[i + 1][0]
            else:
                content_end = len(text)

            # Extract content between headings
            raw_content = text[content_start:content_end].strip()

            # Clean up leading/trailing whitespace and separators
            content = self._clean_content(raw_content)

            sections.append(Section(
                heading=self._format_heading(heading_text, level),
                level=level,
                heading_text=heading_text,  # Store plain text for metadata
                content=content,
                start=start,
                end=content_end
            ))

        return sections

    def _format_heading(self, text: str, level: int) -> str:
        """Format heading with markdown syntax."""
        return "#" * level + " " + text

    def _clean_content(self, content: str) -> str:
        """Clean content by removing leading separators and whitespace."""
        # Remove leading newlines and whitespace
        content = content.strip()
        # Normalize multiple newlines to double newline
        content = re.sub(r'\n{3,}', '\n\n', content)
        return content

    def _split_into_paragraphs(self, text: str) -> List[str]:
        """Split text into paragraphs, handling both structured and continuous text."""
        if not text:
            return []

        # If text fits in chunk_size, return as single paragraph
        if len(text) <= self.chunk_size:
            return [text]

        # Try splitting by double newlines first
        parts = re.split(r'\n\s*\n', text)
        result = []
        for part in parts:
            part = part.strip()
            if not part:
                continue

            # If still too long, try splitting by single newlines
            if len(part) > self.chunk_size:
                sub_parts = re.split(r'\n', part)
                for sp in sub_parts:
                    sp = sp.strip()
                    if sp:
                        result.append(sp)
            else:
                result.append(part)

        # If still nothing split (continuous text with no newlines),
        # do fixed-size split
        if len(result) == 0 or (len(result) == 1 and len(result[0]) > self.chunk_size):
            return self._split_text_fixed(text, 0)  # No overlap for internal splits

        return result

    def _create_chunks_from_sections(self, sections: List[Section]) -> List[Dict[str, Any]]:
        """Convert sections to chunks, splitting if necessary."""
        chunks = []

        for section in sections:
            section_text = section.format()
            section_len = len(section_text)

            # Determine structure type
            if section.level == 0:
                struct_type = "paragraph"
            else:
                struct_type = "section"

            if section_len <= self.chunk_size:
                # Section fits - add as single chunk
                chunks.append({
                    "text": section_text,
                    "metadata": {
                        "structure_type": struct_type,
                        "heading_level": section.level,
                        "heading_text": section.heading_text
                    }
                })
            else:
                # Section too large - split by paragraphs, keep heading with first part
                paragraphs = self._split_into_paragraphs(section.content)

                if not paragraphs:
                    continue

                # First chunk: heading + first paragraphs until fits
                current_text = section.heading
                current_len = len(current_text)

                for para in paragraphs:
                    para_len = len(para)
                    # Add newline if needed
                    trial = current_text + "\n\n" + para if current_text else para
                    trial_len = len(trial)

                    if trial_len > self.chunk_size and current_text != section.heading:
                        # Save current chunk and start new one
                        chunks.append({
                            "text": current_text.strip(),
                            "metadata": {
                                "structure_type": "section_split",
                                "heading_level": section.level,
                                "heading_text": section.heading_text,
                                "is_continuation": True
                            }
                        })
                        current_text = para
                    else:
                        if current_text:
                            current_text = trial
                        else:
                            current_text = para

                # Don't forget the last chunk
                if current_text.strip():
                    chunks.append({
                        "text": current_text.strip(),
                        "metadata": {
                            "structure_type": "section_split",
                            "heading_level": section.level,
                            "heading_text": section.heading_text,
                            "is_continuation": current_text.strip() != section_text
                        }
                    })

        return chunks

    def _apply_overlap(self, chunks: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Add overlap between adjacent chunks by carrying forward content."""
        if self.overlap <= 0 or len(chunks) <= 1:
            return chunks

        result = [chunks[0]]

        for i in range(1, len(chunks)):
            curr = chunks[i]
            prev = result[-1]

            # Get overlap text from current chunk's start
            overlap_text = curr["text"][:self.overlap]

            # Extend previous chunk with overlap
            extended_text = prev["text"] + overlap_text
            result[-1] = {
                **prev,
                "text": extended_text,
                "metadata": {**prev["metadata"], "overlap_applied": self.overlap}
            }
            result.append(curr)

        return result

    def chunk_with_metadata(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Split text based on document structure.

        Each chunk is a logical section: heading + its content.
        """
        self.logger.debug("Chunking text",
                        strategy="structure",
                        text_length=len(text),
                        source=source)

        if not text or not text.strip():
            self.logger.warning("Empty text provided for chunking")
            return []

        if self.split_on == "paragraphs":
            # Just split by paragraphs without structure awareness
            return self._chunk_by_paragraphs_only(text, source)
        elif self.split_on == "lists":
            return self._chunk_by_lists(text, source)
        elif self.split_on == "code_blocks":
            return self._chunk_by_code_blocks(text, source)
        elif self.split_on == "headings" or self.split_on == "auto":
            # Default behavior: heading-centric chunking
            return self._chunk_by_headings(text, source)
        else:
            # Fallback to paragraphs
            return self._chunk_by_paragraphs_only(text, source)

    def _chunk_by_headings(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Chunk by headings - heading + content as one logical unit."""
        sections = self._parse_sections(text)
        chunks = self._create_chunks_from_sections(sections)

        # Apply overlap
        if self.overlap > 0 and len(chunks) > 1:
            chunks = self._apply_overlap(chunks)

        # Final processing: add source and chunk_index
        result = []
        for i, chunk in enumerate(chunks):
            result.append({
                "text": chunk["text"],
                "chunk_index": i,
                "source": source,
                "metadata": chunk.get("metadata", {})
            })

        return result

    def _chunk_by_paragraphs_only(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Simple paragraph chunking without structure."""
        paragraphs = self._split_into_paragraphs(text)
        chunks = []
        current = ""

        for para in paragraphs:
            trial = current + "\n\n" + para if current else para
            if len(trial) > self.chunk_size and current:
                chunks.append(current.strip())
                current = para
            else:
                current = trial

        if current.strip():
            chunks.append(current.strip())

        result = []
        for i, text in enumerate(chunks):
            result.append({
                "text": text,
                "chunk_index": i,
                "source": source,
                "metadata": {"structure_type": "paragraph"}
            })

        return result

    def _chunk_by_lists(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Chunk by list items."""
        # Find list patterns
        pattern = r'(^|\n)(\s*[-*]\s+|\s*\d+\.\s+)'
        parts = re.split(pattern, text)

        chunks = []
        current_items = []
        current_heading = []

        for part in parts:
            stripped = part.strip()
            if not stripped:
                continue

            # Check if this is a list item
            is_list_item = bool(re.match(r'^[-*]\s+|\d+\.\s+$', stripped))

            if is_list_item:
                if current_items:
                    chunks.append({
                        "text": "\n".join(current_items),
                        "metadata": {"structure_type": "list"}
                    })
                current_items = [stripped]
            else:
                # Check if it's a heading
                md_match = re.match(r'^(#{1,6})\s+(.+)', stripped)
                if md_match:
                    if current_items:
                        chunks.append({
                            "text": "\n".join(current_items),
                            "metadata": {"structure_type": "list"}
                        })
                        current_items = []
                    current_heading = [stripped]
                elif current_heading:
                    current_heading.append(stripped)
                    current_items.append(stripped)
                    current_heading = []
                else:
                    current_items.append(stripped)

        if current_items:
            chunks.append({
                "text": "\n".join(current_items),
                "metadata": {"structure_type": "list"}
            })

        # Apply size limits and overlap
        final_chunks = []
        for chunk in chunks:
            if len(chunk["text"]) <= self.chunk_size:
                final_chunks.append(chunk)
            else:
                # Split large chunks
                sub_chunks = self._split_text_fixed(chunk["text"], self.overlap)
                for sc in sub_chunks:
                    final_chunks.append({
                        "text": sc,
                        "metadata": {**chunk["metadata"], "split": "fixed"}
                    })

        # Apply overlap
        if self.overlap > 0 and len(final_chunks) > 1:
            final_chunks = self._apply_overlap(final_chunks)

        # Add source and index
        result = []
        for i, chunk in enumerate(final_chunks):
            result.append({
                "text": chunk["text"],
                "chunk_index": i,
                "source": source,
                "metadata": chunk.get("metadata", {})
            })

        return result

    def _chunk_by_code_blocks(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Chunk by code blocks."""
        # Find code blocks with language
        pattern = r'(```[\w]*\n[\s\S]*?```|`<code>[\s\S]*?</code>`)'

        parts = re.split(pattern, text)
        chunks = []

        for part in parts:
            stripped = part.strip()
            if not stripped:
                continue

            is_code = stripped.startswith('```') or stripped.startswith('<code>')
            chunks.append({
                "text": stripped,
                "metadata": {"structure_type": "code_block" if is_code else "text"}
            })

        # Process chunks
        final_chunks = []
        for chunk in chunks:
            if len(chunk["text"]) <= self.chunk_size:
                final_chunks.append(chunk)
            else:
                sub_chunks = self._split_text_fixed(chunk["text"], self.overlap)
                for sc in sub_chunks:
                    final_chunks.append({
                        "text": sc,
                        "metadata": {**chunk["metadata"], "split": "fixed"}
                    })

        # Add source and index
        result = []
        for i, chunk in enumerate(final_chunks):
            result.append({
                "text": chunk["text"],
                "chunk_index": i,
                "source": source,
                "metadata": chunk.get("metadata", {})
            })

        return result

    def _split_text_fixed(self, text: str, overlap: int) -> List[str]:
        """Split text into fixed-size chunks."""
        if len(text) <= self.chunk_size:
            return [text]

        chunks = []
        step = self.chunk_size - overlap
        start = 0

        while start < len(text):
            end = min(start + self.chunk_size, len(text))
            chunks.append(text[start:end])
            start += step

        return chunks

    def preview(self, text: str, **kwargs) -> Dict[str, Any]:
        """Preview chunking results with statistics."""
        chunks = self.chunk_with_metadata(text, "preview")

        if not chunks:
            return {
                "chunks": [],
                "stats": {"total_chunks": 0, "avg_size": 0, "min_size": 0, "max_size": 0},
                "params": {"split_on": self.split_on, "chunk_size": self.chunk_size, "overlap": self.overlap}
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
            "params": {"split_on": self.split_on, "chunk_size": self.chunk_size, "overlap": self.overlap}
        }


register_chunker("structure", StructureChunker)
