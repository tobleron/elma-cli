"""Chunkers package - multiple chunking strategy implementations."""

import importlib
from .base import TextChunker
from ._registry import CHUNKER_REGISTRY, register_chunker

# Import chunkers to trigger registration
from . import fixed_size_chunker  # noqa: F401
from . import recursive_chunker  # noqa: F401
from . import structure_chunker  # noqa: F401
from . import semantic_chunker  # noqa: F401
from . import llm_chunker  # noqa: F401


def _ensure_registered():
    """Ensure all chunkers are registered. Call this if registry appears empty."""
    if not CHUNKER_REGISTRY:
        # Re-import all chunker modules to trigger registration
        importlib.reload(fixed_size_chunker)
        importlib.reload(recursive_chunker)
        importlib.reload(structure_chunker)
        importlib.reload(semantic_chunker)
        importlib.reload(llm_chunker)


def create_chunker(name: str, **kwargs) -> TextChunker:
    """Factory function to create a chunker by name.

    Args:
        name: Chunker name (e.g., 'fixed', 'semantic', 'recursive', 'structure')
        **kwargs: Chunker-specific parameters

    Returns:
        Chunker instance

    Raises:
        ValueError: If chunker name is not registered
    """
    _ensure_registered()
    if name not in CHUNKER_REGISTRY:
        raise ValueError(f"Unknown chunker: {name}. Available: {list(CHUNKER_REGISTRY.keys())}")
    return CHUNKER_REGISTRY[name](**kwargs)


def list_chunkers():
    """Return list of available chunker names."""
    _ensure_registered()
    return list(CHUNKER_REGISTRY.keys())