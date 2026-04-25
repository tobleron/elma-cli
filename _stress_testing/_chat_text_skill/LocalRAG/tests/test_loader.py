"""Tests for DocumentLoader module."""

import pytest
import tempfile
import os
from pathlib import Path
from src.loader import DocumentLoader


class TestDocumentLoader:
    """Test cases for DocumentLoader class."""

    def setup_method(self):
        """Set up test fixtures."""
        self.loader = DocumentLoader()
        self.temp_dir = tempfile.mkdtemp()

    def teardown_method(self):
        """Clean up temp files."""
        for f in os.listdir(self.temp_dir):
            os.remove(os.path.join(self.temp_dir, f))
        os.rmdir(self.temp_dir)

    def _create_temp_file(self, filename: str, content: str) -> str:
        """Create a temporary file with content."""
        path = os.path.join(self.temp_dir, filename)
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return path

    def test_load_txt_file(self):
        """Test loading a .txt file."""
        content = "Hello, World!"
        path = self._create_temp_file("test.txt", content)
        result = self.loader.load(path)
        assert result == content

    def test_load_md_file(self):
        """Test loading a .md file."""
        content = "# Header\nThis is markdown."
        path = self._create_temp_file("test.md", content)
        result = self.loader.load(path)
        assert result == content

    def test_load_nonexistent_file(self):
        """Test that FileNotFoundError is raised for missing file."""
        with pytest.raises(FileNotFoundError):
            self.loader.load("/nonexistent/file.txt")

    def test_load_unsupported_extension(self):
        """Test that ValueError is raised for unsupported file types."""
        path = self._create_temp_file("test.xyz", "Some content")
        with pytest.raises(ValueError) as exc_info:
            self.loader.load(path)
        assert "Unsupported file type" in str(exc_info.value)

    def test_load_invalid_pdf(self):
        """Test that ValueError is raised for invalid PDF files."""
        path = self._create_temp_file("test.pdf", "PDF content")
        with pytest.raises(ValueError) as exc_info:
            self.loader.load(path)
        assert "Failed to load PDF" in str(exc_info.value)

    def test_load_with_metadata(self):
        """Test loading file with metadata."""
        content = "Test content"
        filename = "test.txt"
        path = self._create_temp_file(filename, content)
        result = self.loader.load_with_metadata(path)
        assert result["content"] == content
        assert result["source"] == filename
        assert result["extension"] == ".txt"

    def test_load_with_metadata_markdown(self):
        """Test loading markdown file with metadata."""
        content = "# Markdown"
        path = self._create_temp_file("doc.md", content)
        result = self.loader.load_with_metadata(path)
        assert result["extension"] == ".md"
