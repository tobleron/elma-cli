"""Tests for RAGPipeline module."""

import pytest
import tempfile
import os
from pathlib import Path
from src.pipeline import RAGPipeline


class TestRAGPipeline:
    """Test cases for RAGPipeline class."""

    def setup_method(self):
        """Set up test fixtures."""
        self.temp_db = tempfile.NamedTemporaryFile(delete=False, suffix=".db")
        self.temp_db.close()
        self.temp_dir = tempfile.mkdtemp()

    def teardown_method(self):
        """Clean up temp files."""
        try:
            os.unlink(self.temp_db.name)
        except:
            pass
        for f in os.listdir(self.temp_dir):
            try:
                os.unlink(os.path.join(self.temp_dir, f))
            except:
                pass
        try:
            os.rmdir(self.temp_dir)
        except:
            pass

    def _create_config(self) -> str:
        """Create a temporary config file."""
        config_path = os.path.join(self.temp_dir, "config.yaml")
        with open(config_path, "w") as f:
            f.write(f"""
llm:
  model_path: "model/llm"
  model_file: "model.gguf"

embedding:
  model_name: "all-MiniLM-L6-v2"

chunking:
  chunk_size: 512
  chunk_overlap: 50

retrieval:
  top_k: 5

vectorstore:
  persist_directory: "data/chroma_db"
""")
        return config_path

    def _create_doc(self, content: str, filename: str = "test.txt") -> str:
        """Create a temporary document."""
        path = os.path.join(self.temp_dir, filename)
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return path

    def test_pipeline_initialization(self):
        """Test that pipeline initializes without errors."""
        config_path = self._create_config()
        pipeline = RAGPipeline(config_path=config_path)
        assert pipeline is not None
        assert pipeline.loader is not None
        assert pipeline.chunker is not None

    def test_pipeline_with_missing_config_uses_defaults(self):
        """Test that pipeline uses defaults when config is missing."""
        pipeline = RAGPipeline(config_path="/nonexistent/config.yaml")
        assert pipeline.config["chunking"]["chunk_size"] == 512
        assert pipeline.config["retrieval"]["top_k"] == 5
