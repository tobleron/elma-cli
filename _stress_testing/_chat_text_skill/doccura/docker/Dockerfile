FROM oven/bun:latest

WORKDIR /app

# Copy package files
COPY package.json bun.lockb* ./

# Install dependencies
RUN bun install --frozen-lockfile

# Copy source code
COPY . .

# Create data directory
RUN mkdir -p /app/data/chroma /app/data/documents /app/data/embeddings /app/data/vectors

# Build MCP server
RUN bun run mcp:build

# Set entrypoint
ENTRYPOINT ["bun", "run", "src/mcp/server.ts"]

