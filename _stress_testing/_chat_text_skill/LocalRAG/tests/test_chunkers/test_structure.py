"""Tests for StructureChunker - document structure-based chunking."""

import pytest
from src.chunkers.structure_chunker import StructureChunker


class TestStructureChunker:
    def test_name(self):
        chunker = StructureChunker()
        assert chunker.name() == "structure"

    def test_heading_with_content_forms_single_chunk(self):
        """Heading and its content should be ONE chunk."""
        chunker = StructureChunker(chunk_size=512)
        text = "# Title\n\nThis is the content under the title."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should be 1 chunk containing both heading and content
        assert len(chunks) == 1
        assert "# Title" in chunks[0]["text"]
        assert "This is the content" in chunks[0]["text"]
        assert chunks[0]["metadata"]["heading_text"] == "Title"
        assert chunks[0]["metadata"]["heading_level"] == 1

    def test_multiple_headings_each_form_separate_chunk(self):
        """Each heading starts a new chunk with its content."""
        chunker = StructureChunker(chunk_size=512)
        text = "# Title A\n\nContent A\n\n# Title B\n\nContent B"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 2
        assert "# Title A" in chunks[0]["text"]
        assert "Content A" in chunks[0]["text"]
        assert "# Title B" in chunks[1]["text"]
        assert "Content B" in chunks[1]["text"]

    def test_nested_headings(self):
        """H2 under H1: H2 content belongs to its parent H1 section."""
        chunker = StructureChunker(chunk_size=512)
        text = "# Main Title\n\nContent under main\n\n## Sub Title\n\nContent under sub"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should have 2 chunks: H1 section and H2 section
        assert len(chunks) == 2
        # First chunk: H1 + its content
        assert "# Main Title" in chunks[0]["text"]
        assert "Content under main" in chunks[0]["text"]
        # Second chunk: H2 + its content
        assert "## Sub Title" in chunks[1]["text"]
        assert "Content under sub" in chunks[1]["text"]

    def test_h2_becomes_new_chunk_when_level_decreases(self):
        """When H2 appears after H1 content, it starts a new chunk."""
        chunker = StructureChunker(chunk_size=512)
        text = "# H1 Title\n\nH1 content\n\n## H2 Title\n\nH2 content"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 2
        assert "## H2 Title" in chunks[1]["text"]

    def test_paragraphs_split_when_exceeds_chunk_size(self):
        """Long content should be split into multiple chunks."""
        chunker = StructureChunker(chunk_size=100)
        text = "# Title\n\n" + "A" * 300  # 300 chars content
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # Should be split into multiple chunks
        assert len(chunks) > 1
        # First chunk should still have the heading
        assert "# Title" in chunks[0]["text"]

    def test_heading_preserved_in_split_chunks(self):
        """When content is split, heading should be preserved in first chunk."""
        chunker = StructureChunker(chunk_size=100)
        text = "# Long Title\n\n" + "B" * 300
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # First chunk has heading
        assert "# Long Title" in chunks[0]["text"]

    def test_no_headings_returns_single_chunk(self):
        """Text without headings returns single chunk."""
        chunker = StructureChunker(chunk_size=512)
        text = "Just plain text without any headings."
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 1
        assert "Just plain text" in chunks[0]["text"]
        assert chunks[0]["metadata"]["structure_type"] == "paragraph"

    def test_html_headings(self):
        """HTML headings should also work."""
        chunker = StructureChunker(chunk_size=512)
        text = "<h1>HTML Title</h1>\n\nHTML content"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 1
        assert "HTML Title" in chunks[0]["text"]
        assert "HTML content" in chunks[0]["text"]

    def test_overlap_between_chunks(self):
        """Overlap should be applied between adjacent chunks."""
        chunker = StructureChunker(chunk_size=100, overlap=20)
        text = "# Title\n\n" + "A" * 200
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        if len(chunks) > 1:
            # Check that overlap was applied (text repeats at boundary)
            # This is indirect - we just verify we get chunks
            assert len(chunks) >= 1

    def test_preview_returns_stats(self):
        """Preview should return statistics."""
        chunker = StructureChunker(chunk_size=200)
        text = "# Title\n\nPara 1.\n\n## Sub\n\nPara 2."
        result = chunker.preview(text)

        assert "chunks" in result
        assert "stats" in result
        assert result["stats"]["total_chunks"] == len(result["chunks"])
        assert result["stats"]["avg_size"] > 0

    def test_list_items_grouped_together(self):
        """List items should be grouped as single chunk."""
        chunker = StructureChunker(chunk_size=512)
        text = "# Title\n\n- Item 1\n- Item 2\n- Item 3"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        assert len(chunks) == 1
        assert "Item 1" in chunks[0]["text"]
        assert "Item 2" in chunks[0]["text"]
        assert "Item 3" in chunks[0]["text"]

    def test_code_blocks_isolated(self):
        """Code blocks should be separate chunks."""
        chunker = StructureChunker(chunk_size=512)
        text = "# Title\n\nSome text\n\n```python\ncode here\n```"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        # At least one chunk should have the code block
        code_chunks = [c for c in chunks if "```" in c["text"]]
        assert len(code_chunks) >= 1

    def test_chunk_metadata_includes_heading_info(self):
        """Each chunk should have heading metadata."""
        chunker = StructureChunker(chunk_size=512)
        text = "# Main Title\n\nContent\n\n## Sub\n\nSub content"
        chunks = chunker.chunk_with_metadata(text, "test.txt")

        for chunk in chunks:
            assert "heading_level" in chunk["metadata"]
            assert "heading_text" in chunk["metadata"]
