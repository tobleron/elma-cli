# pdfrag

> Local-first PDF and Markdown indexing with semantic search and citation-ready answers

`pdfrag` is a Go CLI that discovers PDFs/Markdown files, extracts text, chunks content, generates embeddings with Ollama, stores everything in DuckDB, and answers questions with source citations.

## Installing / Getting started

Quick start with the release binary (Linux amd64):

```shell
curl -L -o pdfrag https://github.com/iamgp/pdfrag/releases/latest/download/pdfrag-linux-amd64
chmod +x pdfrag
./pdfrag setup
./pdfrag index /path/to/documents
./pdfrag query "What does the paper say about X?"
```

What happens:

- Downloads the latest single `pdfrag` binary from GitHub Releases.
- Marks it executable.
- `setup` ensures Ollama is available and required models are pulled.
- `index` ingests PDFs/Markdown into DuckDB with embeddings.
- `query` performs semantic retrieval and returns a citation-ready answer.

Available release binaries:

- `pdfrag-linux-amd64`
- `pdfrag-darwin-arm64`

Windows users: run `pdfrag` via WSL using the Linux binary (`pdfrag-linux-amd64`).

If you prefer building locally:

```shell
go build -o pdfrag .
```

### Initial Configuration

Minimum runtime requirements:

- Ollama installed and reachable at `http://localhost:11434`

Default config path is `~/.pdfrag/config.yaml`.

Common settings to customize:

- `storage.path`
- `ollama.url`
- `ollama.embedding_model`
- `ollama.chat_model`
- `search.top_k`

If you run Ollama manually:

```shell
ollama serve
```

## Developing

```shell
git clone https://github.com/iamgp/pdfrag.git
cd pdfrag
go mod download
```

Then iterate with:

```shell
go test ./...
go run . query "test"
```

### Building

```shell
go build -o pdfrag .
```

CI build automation is in `.github/workflows/build.yml` and runs on every push to `main` and on all pull requests:

- `go test ./...`
- `go build -o pdfrag .`

### Deploying / Publishing

Release automation is in `.github/workflows/release.yml`.

Publishing flow:

```shell
git tag v0.1.0
git push origin v0.1.0
```

What happens:

- GitHub Actions runs tests.
- GitHub Actions builds Linux and macOS binaries on native runners.
- Uploads each binary and matching `.sha256` checksum to the GitHub Release.

## Features

- Index local PDFs and Markdown documents.
- Convert and chunk content for embedding/search workflows.
- Store documents, chunks, and embeddings in DuckDB.
- Generate embeddings via Ollama.
- Query with semantic retrieval and citation-ready output.
- Run interactive querying sessions.
- Reindex changed documents and manage indexed content.
- Export/import backups.

## Configuration

#### Config path (`--config`)

Type: `String`
Default: `~/.pdfrag/config.yaml`

Overrides the config file location.

Example:

```bash
./pdfrag --config ./config.yaml query "Summarize section 2"
```

#### Ollama URL (`ollama.url`)

Type: `String`
Default: `'http://localhost:11434'`

Sets the Ollama endpoint used for embeddings and chat.

Example:

```yaml
ollama:
  url: http://localhost:11434
```

#### Embedding model (`ollama.embedding_model`)

Type: `String`
Default: `'nomic-embed-text'`

Model used to generate embeddings during indexing and querying.

Example:

```yaml
ollama:
  embedding_model: nomic-embed-text
```

#### Chat model (`ollama.chat_model`)

Type: `String`
Default: `'llama3.1'`

Model used for answer generation in query flows.

Example:

```yaml
ollama:
  chat_model: llama3.1
```

#### Database path (`storage.path`)

Type: `String`
Default: `'./data/pdfrag.db'`

Controls where DuckDB data is stored.

Example:

```yaml
storage:
  path: ./data/pdfrag.db
```

#### Search top-k (`search.top_k`)

Type: `Number`
Default: `8`

Number of candidate chunks retrieved for answering.

Example:

```yaml
search:
  top_k: 8
```
