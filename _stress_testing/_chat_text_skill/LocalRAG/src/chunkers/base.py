"""Abstract base class for text chunkers."""

from abc import ABC, abstractmethod
from typing import List, Dict, Any


class TextChunker(ABC):
    """Abstract base class for text chunking strategies."""

    @abstractmethod
    def chunk_with_metadata(self, text: str, source: str) -> List[Dict[str, Any]]:
        """Split text into chunks with metadata.

        Args:
            text: Text content to chunk
            source: Source filename

        Returns:
            List of dicts containing:
                - text: chunk content (str)
                - source: source filename (str)
                - chunk_index: sequential index (int)
                - metadata: strategy-specific data (dict)
        """
        pass

    @abstractmethod
    def preview(self, text: str, **kwargs) -> Dict[str, Any]:
        """Preview chunking results without persisting.

        Args:
            text: Text to preview chunking on
            **kwargs: Strategy-specific preview parameters

        Returns:
            dict containing:
                - chunks: list of chunk previews
                - stats: dict with total_chunks, avg_size, min_size, max_size
                - params: parameters used for this preview
        """
        pass

    @abstractmethod
    def name(self) -> str:
        """Return strategy identifier string."""
        pass