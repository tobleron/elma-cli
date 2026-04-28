#!/usr/bin/env python3
"""Runner script for Local RAG application."""

import sys
from pathlib import Path

project_root = Path(__file__).parent
sys.path.insert(0, str(project_root / "src"))

from app import main

if __name__ == "__main__":
    main()
