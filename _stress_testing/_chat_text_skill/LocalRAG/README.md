# Local RAG

A lightweight, production-ready RAG (Retrieval Augmented Generation) system built from scratch for learning purposes.

[![GitHub stars](https://img.shields.io/github/stars/YannBuf/LocalRAG)](https://github.com/YannBuf/LocalRAG/stargazers)
[![Python](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/)
[![Streamlit](https://img.shields.io/badge/streamlit-1.40+-red.svg)](https://streamlit.io/)

## Overview

This project implements a complete RAG pipeline using OpenAI-compatible APIs, with a full-featured Streamlit UI:

```
Document → Load → Chunk → Embed → Vector Store → Retrieve → LLM → Answer
```

### Features

- **From scratch implementation** — No LangChain/LlamaIndex, understand every component
- **Lightweight & fast** — Chroma for vector storage, minimal dependencies
- **API-based** — Works with any OpenAI-compatible API (LM Studio, Ollama, vLLM, etc.)
- **5-Tab Web UI** — Configuration, Chunking, Documents, RAG, Observability
- **5 Chunking Strategies** — Fixed, Recursive, Structure, Semantic, LLM-based
- **Chat History** — Persistent conversation history with JSON storage
- **User Feedback** — Thumbs up/down on answers, stored persistently
- **Document Management** — View, filter, and delete indexed documents
- **Observability** — Structured logging, Prometheus metrics, log viewer
- **Hybrid Search** — BM25 + vector similarity with configurable weights
- **Reranking** — CrossEncoder support (API / local / HuggingFace / disabled)
- **MMR** — Maximal Marginal Relevance for diverse results
- **Incremental Upsert** — Only re-index changed chunks, reuse existing embeddings

## Tech Stack

| Component | Technology |
|-----------|------------|
| LLM | OpenAI-compatible API |
| Embedding | OpenAI-compatible API |
| Vector Store | Chroma |
| UI | Streamlit |
| Logging | structlog + RotatingFileHandler |
| Metrics | prometheus-client |
| Testing | pytest |

## Project Structure

```
SimpleRag/
├── config/
│   └── api_settings.yaml     # API configuration (LLM, Embedding, Rerank)
├── data/
│   ├── chroma_db/            # Chroma vector database
│   ├── chat_history.json     # Conversation history
│   ├── feedback.json         # User feedback
│   └── uploads/              # Uploaded documents
├── logs/
│   └── app.log               # Application logs (rotated)
├── src/
│   ├── __init__.py
│   ├── loader.py             # Document loader (txt, md, pdf)
│   ├── chunker.py            # Legacy chunker wrapper
│   ├── chunkers/             # Chunker strategies package
│   │   ├── __init__.py
│   │   ├── base.py           # Abstract base class
│   │   ├── _registry.py      # Chunker registry
│   │   ├── fixed_size_chunker.py
│   │   ├── recursive_chunker.py
│   │   ├── structure_chunker.py
│   │   ├── semantic_chunker.py
│   │   └── llm_chunker.py
│   ├── embedder_api.py       # Embedding API client (retry, cache, batch)
│   ├── vectorstore.py        # Chroma storage with upsert & HNSW config
│   ├── retriever.py          # Hybrid search, MMR, reranking, cache
│   ├── history_manager.py    # Chat history & feedback persistence
│   ├── llm_api.py            # LLM API client (retry, streaming)
│   ├── pipeline.py           # RAG orchestration
│   ├── observability.py      # Logging, metrics, tracing
│   └── app.py                # Streamlit app (5 tabs)
├── tests/
│   ├── test_loader.py
│   ├── test_chunker.py
│   ├── test_pipeline.py
│   ├── test_history_manager.py
│   └── test_chunkers/
├── CHANGELOG.md
├── CHAT_LOG_*.md
└── README.md
```

## Quick Start

### 1. Install Dependencies

```bash
pip install -r requirements.txt
```

### 2. Start an OpenAI-Compatible API Server

**LM Studio** (recommended for local):
1. Download [LM Studio](https://lmstudio.ai/)
2. Download a model (e.g., Llama 3.2)
3. Click "Start Server" — defaults to `http://localhost:1234/v1`

**Ollama**:
```bash
ollama serve
# Default: http://localhost:11434/v1
```

### 3. Run

```bash
streamlit run src/app.py --server.port 8501
```

Open `http://localhost:8501` in your browser.

### 4. Configure

In the **Configuration tab**, set your API endpoints and model names, then click **Apply Configuration**.

## UI Overview

### 5 Tabs

| Tab | Description |
|-----|-------------|
| **⚙️ Configuration** | API endpoints (LLM, Embedding, Reranking), retrieval settings |
| **🔪 Chunking** | Upload documents, choose chunking strategy, preview & index |
| **📁 Documents** | View indexed documents, chunk counts, delete per-doc or all |
| **💬 RAG** | Chat with your documents — sidebar shows chat history |
| **📊 Observability** | Live log viewer, Prometheus metrics endpoint, live stats |

### Chat History & Feedback

- Conversations are saved automatically to `data/chat_history.json`
- Click any past conversation in the sidebar to reload it
- Thumbs up/down on each answer are saved to `data/feedback.json`

### Reranking

Configure in the **Configuration tab**:

| Mode | Configuration |
|------|--------------|
| **API mode** | Set `Rerank API Base` + `Rerank API Key` |
| **Local mode** | Set `Rerank Model` as a local directory path |
| **HuggingFace mode** | Set `Rerank Model` as a HuggingFace model ID |
| **Disabled** | Leave all rerank fields empty — uses embedding fallback |

## Chunking Strategies

| Strategy | Description |
|----------|-------------|
| **Fixed Size** | Uniform chunks with character count + overlap |
| **Recursive** | Separator-based recursive splitting |
| **Structure** | Heading + content as one logical unit |
| **Semantic** | Sentence split → embed → merge by similarity |
| **LLM-based** | LLM-driven semantic boundary detection |

## Testing

```bash
# All tests
pytest tests/ -v

# Specific suites
pytest tests/test_history_manager.py -v
pytest tests/test_chunkers/ -v
```

## Requirements

- Python 3.10+
- OpenAI-compatible API server (LM Studio, Ollama, vLLM, etc.)
- Optional: CrossEncoder model for reranking

## License

MIT
