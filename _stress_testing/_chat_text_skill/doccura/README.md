# Doccura

Doccura - Local RAG system with terminal interface. Run as TUI application or MCP server for integration with external tools.

## Features

- 🎨 TUI with Ink (React for terminal)
- 🔍 RAG system with Chroma vector database
- 🤖 Ollama integration with thinking control
- 📄 PDF and TXT document support
- 🐳 Docker MCP server support
- 💾 Local storage (everything in `data/` folder)
- 🎭 Customizable AI personality via `personality.txt`

## Prerequisites

- [Bun](https://bun.sh) installed
- [Ollama](https://ollama.ai) running locally (default: `http://localhost:11434`)
- A model installed in Ollama (default: `qwen3:1.7b`)
- [Chroma](https://www.trychroma.com/) server running (default: `http://localhost:8000`)

**Quick Chroma setup:**
```bash
# Using Docker
docker run -d -p 8000:8000 chromadb/chroma:latest

# Or using pip
pip install chromadb
chroma run --path ./data/chroma --port 8000
```

## Installation

```bash
# Install dependencies
bun install

# Copy environment template
cp env.template .env

# Edit .env if needed (optional)
```

## Usage

### TUI Mode

Run the terminal interface directly:

```bash
bun run dev
```

**Commands in TUI:**
- `/help` - Show available commands
- `/upload <filepath> [collection]` - Upload PDF/TXT document
- `/collections` - List all collections
- `/collection <name>` - Switch active collection
- `/status` - Show system status
- `/coldel <name>` - Delete collection
- `/col <name> del <num>` - Delete document from collection
- `/exit` or `/bye` - Exit the application

**RAG Queries:**
- `@rag <question>` - Search in documents
- `/rag <question>` - Search in documents
- `? <question>` - Search in documents
- Direct questions (without prefix) - Normal chat with Ollama

### MCP Server Mode

Run as MCP server for integration with external tools (e.g., Claude Desktop):

```bash
bun run mcp:dev
```

**Available MCP Tools:**
- `upload_document` - Upload and index a PDF or TXT document
- `query_rag` - Query the RAG system with a question
- `list_collections` - List all collections with statistics
- `get_status` - Get system status

### Docker Mode

Run MCP server in Docker with Chroma included:

```bash
cd docker
docker-compose up --build
```

**Note:** For Docker, make sure Ollama is accessible from container. Use `host.docker.internal:11434` on Mac/Windows or host network on Linux.

## Configuration

### Environment Variables

Edit `.env` file or set environment variables:

```env
# Ollama
OLLAMA_ENDPOINT=http://localhost:11434
OLLAMA_MODEL=qwen3:1.7b
ENABLE_THINKING=false

# RAG
RAG_CHUNK_SIZE=1000
RAG_CHUNK_OVERLAP=200
RAG_MAX_RESULTS=5
RAG_SIMILARITY_THRESHOLD=0.3

# Embeddings
EMBEDDING_MODEL=Xenova/paraphrase-multilingual-MiniLM-L12-v2

# Storage (relative to project root)
CHROMA_URL=http://localhost:8000
CHROMA_PATH=./data/chroma
DOCUMENTS_PATH=./data/documents
EMBEDDINGS_CACHE_PATH=./data/embeddings
MAX_FILE_SIZE_MB=50

# Personality (optional)
PERSONALITY_FILE=./personality.txt
RAG_PERSONALITY_FILE=./rag-personality.txt
```

### AI Personality

Customize the AI's personality by editing `personality.txt` in the project root. This file defines how the AI behaves in normal conversations.

Example `personality.txt`:
```
You are a friendly and helpful AI assistant. Answer questions and help the user.

You are knowledgeable, patient, and clear in your explanations. You adapt your communication style to the user's needs and provide helpful, accurate information.
```

For RAG queries (when searching documents), you can create a separate `rag-personality.txt` file. If it doesn't exist, the system will use a default RAG-specific personality.

The personality file is loaded at runtime, so you can modify it before starting the application without rebuilding.

## Project Structure

```
doccura/
├── src/
│   ├── core/              # RAG core logic
│   │   ├── embeddings.ts  # Embedding generation
│   │   ├── vector-db.ts   # Chroma client
│   │   ├── chunker.ts     # Text chunking
│   │   ├── pdf-processor.ts # PDF extraction
│   │   └── rag-service.ts  # Main RAG service
│   ├── ollama/            # Ollama integration
│   │   ├── client.ts      # Ollama API client
│   │   └── rag-chat.ts   # RAG + Ollama chat
│   ├── tui/               # Terminal UI
│   │   ├── app.tsx        # Main TUI component
│   │   └── components/    # UI components
│   ├── mcp/               # MCP server
│   │   ├── server.ts      # MCP server implementation
│   │   └── tools.ts       # RAG tools for MCP
│   └── config/            # Configuration
├── data/                  # Local storage (gitignored)
│   ├── chroma/           # Chroma vector DB storage
│   ├── documents/        # Uploaded documents
│   └── embeddings/       # Embeddings cache
├── docker/               # Docker configuration
│   ├── Dockerfile
│   └── docker-compose.yml
└── index.tsx             # Entry point
```

## How It Works

1. **Document Upload:** PDF/TXT files are processed, chunked, and embedded
2. **Vector Storage:** Embeddings stored in Chroma vector database
3. **Query Processing:** User questions are embedded and searched in vector DB
4. **Context Retrieval:** Relevant document chunks retrieved based on similarity
5. **LLM Response:** Ollama generates answer using retrieved context
6. **Streaming:** Responses streamed in real-time to TUI

## Development

```bash
# Run in development mode
bun run dev

# Build
bun run build

# MCP server
bun run mcp:dev

# Build MCP server
bun run mcp:build
```

## Testing

See `TESTING.md` for complete testing guide.

Quick test:
1. Start Chroma: `docker run -d -p 8000:8000 chromadb/chroma:latest`
2. Start TUI: `bun run dev`
3. Upload document: `/upload /path/to/document.pdf`
4. Query: `@rag What is this document about?`

## License

MIT
