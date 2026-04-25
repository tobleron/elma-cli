# Testing Guide - Ollama RAG TUI

## 1. Start Services

### Chroma Server (required)
```bash
# Check if already running
docker ps | grep chroma

# If not running, start it:
docker run -d -p 8000:8000 \
  -v $(pwd)/data/chroma:/chroma/chroma \
  -e IS_PERSISTENT=TRUE \
  -e PERSIST_DIRECTORY=/chroma/chroma \
  --name ollama-rag-chroma \
  chromadb/chroma:latest

# Verify it's running
curl http://localhost:8000/api/v1/heartbeat
```

### Ollama (required)
```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# If not running, start it:
ollama serve
```

## 2. Test TUI Mode

```bash
# Start TUI
bun run dev
```

**In TUI you can:**
- Type questions directly (after uploading documents)
- `/help` - See all commands
- `/collections` - List collections
- `/collection <name>` - Switch active collection
- `/status` - See system status

**Quick test:**
1. Start TUI: `bun run dev`
2. Type: `/help` (Enter)
3. Type: `/status` (Enter)
4. Type: `/collections` (Enter)

## 3. Test MCP Server

### Mode 1: Local (stdio)
```bash
# Start MCP server
bun run mcp:dev

# Server waits for input on stdin (used by MCP client)
```

### Mode 2: With an MCP client
```bash
# Example with Claude Desktop or other MCP client
# Configure in settings.json:
{
  "mcpServers": {
    "ollama-rag": {
      "command": "bun",
      "args": ["run", "src/mcp/server.ts"],
      "cwd": "/path/to/ollama-rag-tui"
    }
  }
}
```

**Available tools:**
- `upload_document` - Upload PDF/TXT
- `query_rag` - Query RAG system
- `list_collections` - List collections
- `get_status` - System status

## 4. Test Docker Mode

```bash
cd docker

# Start all services (Chroma + MCP server)
docker-compose up --build

# Or in background
docker-compose up -d

# View logs
docker-compose logs -f

# Stop
docker-compose down
```

## 5. Complete Testing - Workflow

### Step 1: Upload a document (via MCP or manual)

**Via MCP server:**
```bash
# Start MCP server
bun run mcp:dev

# In another terminal, use an MCP client or:
# Manual example (if you have a script):
echo '{"method":"tools/call","params":{"name":"upload_document","arguments":{"filePath":"/path/to/document.pdf","collection":"test"}}}' | bun run src/mcp/server.ts
```

**Or manually (for quick test):**
```bash
# Create a test file
echo "This is a test document. It contains testing information." > /tmp/test.txt

# Use MCP server or integrate directly in code
```

### Step 2: Query in TUI

```bash
# Start TUI
bun run dev

# Type a question about the uploaded document
# Example: "What does the document contain?"
```

### Step 3: Check collections

In TUI:
```
/collections
```

Or via MCP:
```bash
# Call list_collections tool
```

## 6. Debugging

### Check Chroma logs
```bash
docker logs ollama-rag-chroma
```

### Check if services respond
```bash
# Chroma
curl http://localhost:8000/api/v1/heartbeat

# Ollama
curl http://localhost:11434/api/tags
```

### Check environment variables
```bash
# Create .env from template
cp env.template .env

# Edit .env if needed
```

## 7. Common Issues

### Chroma not responding
```bash
# Check if container is running
docker ps | grep chroma

# Restart
docker restart ollama-rag-chroma

# Check logs
docker logs ollama-rag-chroma
```

### Ollama not responding
```bash
# Check if Ollama is running
ollama list

# Start Ollama
ollama serve
```

### Sharp module error
```bash
# Rebuild sharp
cd node_modules/@xenova/transformers/node_modules/sharp
npm rebuild sharp
cd ../../../../..
```

### Chroma connection error
```bash
# Check CHROMA_URL in .env
cat .env | grep CHROMA_URL

# Should be: CHROMA_URL=http://localhost:8000
```

## 8. Quick Test

```bash
# 1. Start Chroma
docker run -d -p 8000:8000 --name ollama-rag-chroma chromadb/chroma:latest

# 2. Check Ollama
curl http://localhost:11434/api/tags

# 3. Start TUI
bun run dev

# 4. In TUI, test:
#    - /help
#    - /status
#    - /collections
```

## 9. Next Steps

After everything works:
1. Upload real documents (PDF/TXT)
2. Test complex queries
3. Explore MCP tools
4. Test complete Docker setup
