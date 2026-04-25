# Non-Rust Dependencies Reference

This document provides a comprehensive overview of the significant libraries, modules, and packages used in the non-Rust projects within the Elma CLI stress-testing environment.

## Python

### AI / LLM
- **langchain**: A framework for developing applications powered by large language models.
- **litellm**: A library to call 100+ LLM APIs using the OpenAI format.
- **openai**: The official Python client for the OpenAI API.
- **ollama**: Interface for running and interacting with large language models locally.
- **transformers**: State-of-the-art machine learning for Pytorch, TensorFlow, and JAX.
- **pytorch / torch**: An open-source machine learning framework that accelerates the path from research prototyping to production.
- **sentence-transformers**: Framework for state-of-the-art sentence, text, and image embeddings.
- **tiktoken**: A fast BPE tokenizer for use with OpenAI's models.
- **fal-client**: Python client for fal.ai, providing easy access to generative media models.
- **edge-tts**: Use Microsoft Edge's online text-to-speech service from Python.

### Web / Framework
- **fastapi**: A modern, fast (high-performance), web framework for building APIs with Python 3.8+ based on standard Python type hints.
- **flask**: A lightweight WSGI web application framework.
- **aiohttp**: Asynchronous HTTP Client/Server for asyncio and Python.

### Data / Infrastructure
- **pydantic**: Data validation and settings management using Python type hints.
- **chromadb**: An AI-native, open-source vector database.
- **beautifulsoup4**: Library for pulling data out of HTML and XML files.
- **firecrawl-py**: Python SDK for web crawling and data extraction optimized for LLMs.
- **celery**: A distributed task queue for processing vast amounts of messages.
- **tenacity**: An Apache 2.0 licensed general-purpose retrying library.
- **mcp**: Python implementation of the Model Context Protocol.

### Utilities
- **gitpython**: A python library used to interact with Git repositories.
- **tree-sitter**: Python bindings for the tree-sitter incremental parsing system.
- **fire**: A library for automatically generating command line interfaces (CLIs).
- **streamlit**: An open-source Python library that makes it easy to create and share custom web apps for machine learning and data science.
- **discord.py**: A modern, easy to use, feature-rich, and async ready API wrapper for Discord.

---

## Node.js / TypeScript

### AI / LLM
- **@ai-sdk/* (Vercel)**: A unified interface for interacting with various LLM providers (Anthropic, Google, OpenAI, etc.).
- **@anthropic-ai/sdk**: Official TypeScript library for the Anthropic API.
- **@google/genai**: Official Node.js SDK for Google's Generative AI models.
- **ai (Vercel AI SDK)**: Core library for building AI-powered streaming text and chat UIs.
- **@modelcontextprotocol/sdk**: Official implementation of the Model Context Protocol in TypeScript.
- **@roo-code/core**: Core logic and tools for the Roo-Code agentic framework.

### UI / Frontend
- **react**: The core library for building component-based user interfaces.
- **next**: The React framework for production, providing SSR, static generation, and routing.
- **tailwindcss**: A utility-first CSS framework for rapid UI development.
- **@radix-ui/* / shadcn/ui**: Low-level, accessible UI primitives and pre-styled components.
- **framer-motion**: A popular motion library for React animations.
- **lucide-react**: A library of beautiful and consistent icons.
- **ink**: React-based framework for building interactive command-line interfaces.
- **monaco-editor**: The browser-based code editor that powers VS Code.

### Backend / Data
- **express**: Fast, unopinionated, minimalist web framework for Node.js.
- **drizzle-orm**: Next-generation TypeScript ORM that feels like writing SQL.
- **zod**: TypeScript-first schema declaration and validation library.
- **axios**: Promise-based HTTP client for the browser and node.js.
- **react-router**: Standard routing library for React applications.

### Desktop / Native
- **electron**: Framework for building cross-platform desktop applications using web technologies.
- **tauri**: Build smaller, faster, and more secure desktop applications with a web frontend.

### Utilities
- **zustand**: A small, fast, and scalable bearbones state-management solution.
- **lodash**: A modern JavaScript utility library delivering modularity, performance, & extras.
- **chalk**: Terminal string styling done right.
- **commander**: The complete solution for Node.js command-line interfaces.
- **i18next**: An internationalization-framework written in and for JavaScript.
- **tree-sitter**: Incremental parsing system for multiple programming languages.

---

## Go

### AI / LLM
- **langchaingo**: The Go implementation of the LangChain framework for LLM applications.
- **github.com/sashabaranov/go-openai**: The most popular Go client for the OpenAI API.
- **github.com/modelcontextprotocol/go-sdk**: Go implementation of the Model Context Protocol.
- **github.com/anthropics/anthropic-sdk-go**: Official Go SDK for the Anthropic API.
- **google.golang.org/genai**: Official Go SDK for Google Generative AI.

### CLI / TUI
- **github.com/charmbracelet/bubbletea**: A powerful TUI framework based on the Elm Architecture.
- **github.com/spf13/cobra**: A library for creating powerful modern CLI applications.
- **github.com/spf13/viper**: Complete configuration solution for Go applications including JSON, TOML, YAML, HCL, env vars and config flags.
- **github.com/charmbracelet/lipgloss**: Terminal layout and style engine for nice TUI designs.
- **github.com/charmbracelet/glamour**: Markdown rendering for the terminal.
- **github.com/muesli/termenv**: Advanced ANSI styling and color support for terminal applications.

### Data / Storage
- **github.com/blevesearch/bleve/v2**: A modern text indexing library for Go.
- **github.com/philippgille/chromem-go**: An embeddable vector database for Go with zero external dependencies.
- **github.com/jackc/pgx/v5**: A pure Go driver and toolkit for PostgreSQL.
- **github.com/marcboeker/go-duckdb**: Go driver for the DuckDB analytical database.
- **go.etcd.io/bbolt**: An embedded key/value database for Go.

### PDF / Processing
- **github.com/gen2brain/go-fitz**: Go wrapper for MuPDF for PDF and ebook rendering.
- **github.com/jung-kurt/gofpdf**: A PDF document generator with high level support for text, images and colors.
- **github.com/dslipak/pdf**: A library for reading and extracting text from PDF files.

### Infrastructure
- **github.com/gofiber/fiber/v2**: An Express-inspired web framework built on top of Fasthttp.
- **github.com/go-git/go-git/v5**: A highly extensible git implementation library in pure Go.
- **github.com/google/uuid**: Go package for generating and inspecting UUIDs.
- **go.uber.org/zap**: Blazing fast, structured, leveled logging in Go.
