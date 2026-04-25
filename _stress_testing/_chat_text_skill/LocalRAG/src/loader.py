"""Document loader module for Local RAG."""

import os
from pathlib import Path
from typing import Union
from pypdf import PdfReader

from observability import get_logger, traced, log_error_alert


class DocumentValidationError(ValueError):
    """Raised when a document fails validation."""
    pass


class ContentValidator:
    """Validates document and chunk content."""

    MIN_CHUNK_LENGTH = 10
    MAX_CHUNK_LENGTH = 10000
    MAX_DOC_LENGTH = 10_000_000  # ~10MB of text

    @classmethod
    def validate_document(cls, content: str, source: str) -> None:
        """Validate a loaded document.

        Raises:
            DocumentValidationError: If content is invalid.
        """
        if not isinstance(content, str):
            raise DocumentValidationError(
                f"Document content must be str, got {type(content).__name__}"
            )

        if len(content) == 0:
            raise DocumentValidationError(
                f"Document is empty: {source}"
            )

        if len(content) > cls.MAX_DOC_LENGTH:
            raise DocumentValidationError(
                f"Document too large ({len(content):,} chars, max {cls.MAX_DOC_LENGTH:,}): {source}"
            )

        stripped = content.strip()
        if len(stripped) == 0:
            raise DocumentValidationError(
                f"Document contains only whitespace: {source}"
            )

    @classmethod
    def validate_chunk(cls, text: str, chunk_index: int, source: str) -> None:
        """Validate a single chunk.

        Raises:
            DocumentValidationError: If chunk is invalid.
        """
        if not isinstance(text, str):
            raise DocumentValidationError(
                f"Chunk {chunk_index} content must be str, got {type(text).__name__} "
                f"(source: {source})"
            )

        if len(text.strip()) == 0:
            raise DocumentValidationError(
                f"Chunk {chunk_index} is empty or whitespace-only "
                f"(source: {source})"
            )

        if len(text) > cls.MAX_CHUNK_LENGTH:
            raise DocumentValidationError(
                f"Chunk {chunk_index} too large ({len(text):,} chars, "
                f"max {cls.MAX_CHUNK_LENGTH:,}) (source: {source})"
            )


class DocumentLoader:
    """Loads text documents from local filesystem.

    Supports .txt, .md, and .pdf file formats.
    """

    SUPPORTED_EXTENSIONS = {".txt", ".md", ".pdf"}

    def __init__(self):
        self.logger = get_logger(__name__)

    def load(self, file_path: Union[str, Path]) -> str:
        """Load document content from file.

        Args:
            file_path: Path to the document file.

        Returns:
            Raw text content of the document.

        Raises:
            FileNotFoundError: If the file does not exist.
            ValueError: If the file extension is not supported.
        """
        self.logger.info("Loading document", file_path=str(file_path))

        path = Path(file_path)

        if not path.exists():
            error = FileNotFoundError(f"File not found: {file_path}")
            log_error_alert(self.logger, error, "loader",
                          context={"file_path": str(file_path)})
            raise error

        if not path.is_file():
            error = FileNotFoundError(f"Path is not a file: {file_path}")
            log_error_alert(self.logger, error, "loader",
                          context={"file_path": str(file_path)})
            raise error

        extension = path.suffix.lower()
        if extension not in self.SUPPORTED_EXTENSIONS:
            error = ValueError(
                f"Unsupported file type: {extension}. "
                f"Supported types: {', '.join(self.SUPPORTED_EXTENSIONS)}"
            )
            log_error_alert(self.logger, error, "loader",
                          context={"file_path": str(file_path), "extension": extension})
            raise error

        try:
            if extension == ".pdf":
                content = self._load_pdf(path)
            else:
                with open(path, "r", encoding="utf-8") as f:
                    content = f.read()

            ContentValidator.validate_document(content, path.name)

            self.logger.info("Document loaded",
                           file_path=str(file_path),
                           content_length=len(content))
            return content

        except Exception as e:
            log_error_alert(self.logger, e, "loader",
                          context={"file_path": str(file_path)})
            raise

    def _load_pdf(self, path: Path) -> str:
        """Load text content from PDF file.

        Args:
            path: Path to the PDF file.

        Returns:
            Extracted text from all pages.

        Raises:
            ValueError: If file is not a valid PDF.
        """
        try:
            reader = PdfReader(path)
            text_parts = []
            for page in reader.pages:
                text = page.extract_text()
                if text:
                    text_parts.append(text)
            if not text_parts:
                return ""
            return "\n\n".join(text_parts)
        except Exception as e:
            raise ValueError(f"Failed to load PDF: {e}")

    def load_with_metadata(self, file_path: Union[str, Path]) -> dict:
        """Load document content along with metadata.

        Args:
            file_path: Path to the document file.

        Returns:
            Dictionary containing 'content', 'source', and 'extension' keys.
        """
        path = Path(file_path)
        content = self.load(path)

        return {
            "content": content,
            "source": path.name,
            "extension": path.suffix.lower(),
        }
