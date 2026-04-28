"""Internal registry for chunker registration."""

CHUNKER_REGISTRY = {}


def register_chunker(name: str, chunker_class: type):
    """Register a chunker class by name."""
    CHUNKER_REGISTRY[name] = chunker_class