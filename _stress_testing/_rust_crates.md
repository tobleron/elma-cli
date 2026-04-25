# Rust Crates in Project Ecosystem

This document provides a comprehensive list of Rust crates used in the `opencrabs`, `goose`, and `codex-cli` projects, categorized by their primary functionality.

## AI & Machine Learning
- **bm25**: Implementation of the BM25 ranking function for search relevance.
- **llama-cpp-2**: Safe and ergonomic bindings for the llama.cpp library.
- **local-stt**: Library for local speech-to-text processing.
- **local-tts**: Library for local text-to-speech synthesis.
- **rwhisper**: Rust bindings for the whisper.cpp speech recognition engine.
- **tiktoken-rs**: Rust implementation of the tiktoken tokenizer used by OpenAI models.
- **qmd**: High-performance local search engine for Markdown documents.
- **rayon**: Data parallelism library for parallelizing computations across CPU cores.

## Async & Concurrency
- **async-channel**: Asynchronous multi-producer multi-consumer channels.
- **async-io**: Async I/O primitives and executors for building asynchronous applications.
- **async-stream**: Macros for creating asynchronous streams using generator-like syntax.
- **async-trait**: Support for defining asynchronous functions in traits.
- **crossbeam-channel**: High-performance multi-producer multi-consumer channels.
- **futures**: Zero-cost abstractions for asynchronous programming in Rust.
- **tokio**: Event-driven platform for writing non-blocking network applications.
- **tokio-stream**: Utilities for working with asynchronous streams in the Tokio ecosystem.
- **tokio-util**: Collection of specialized utilities for working with Tokio.
- **arc-swap**: Library for atomically swapping Arc pointers.
- **once_cell**: Single assignment cells and lazy statics for initialized-once data.
- **lazy_static**: Macro for declaring lazily evaluated static variables.

## Networking & Web
- **axum**: Ergonomic and modular web framework built on top of Tower and Hyper.
- **http**: Type system for HTTP requests, responses, and related components.
- **reqwest**: High-level and ergonomic HTTP client for Rust.
- **tower**: Modular components for building robust and reusable networking services.
- **tower-http**: HTTP-specific middleware and utilities for the Tower ecosystem.
- **tonic**: gRPC over HTTP/2 implementation for Rust.
- **tonic-prost**: Prost-based code generation for the Tonic gRPC framework.
- **eventsource-stream**: Parser for Server-Sent Events (SSE) as a stream of data.
- **socket2**: Low-level interface for working with network sockets.
- **dns-lookup**: Simple library for performing DNS lookups.
- **url**: Comprehensive library for parsing and manipulating URLs.
- **urlencoding**: Library for percent-encoding and decoding strings in URLs.
- **tungstenite**: Lightweight and fast WebSocket implementation.
- **tokio-tungstenite**: Tokio-compatible bindings for the Tungstenite WebSocket library.
- **tiny_http**: Low-level, fast, and simple HTTP server library.
- **mime_guess**: Utility for guessing MIME types based on file extensions.

## CLI & TUI
- **clap**: Robust and feature-rich command-line argument parser.
- **clap_complete**: Support for generating shell completion scripts from clap definitions.
- **crossterm**: Cross-platform library for terminal manipulation and input handling.
- **ratatui**: Library for building rich and interactive terminal user interfaces.
- **ratatui-macros**: Procedural macros for simplifying Ratatui UI development.
- **ansi-to-tui**: Utility for converting ANSI escape sequences into Ratatui text objects.
- **owo-colors**: Minimalist and efficient library for terminal text coloring.
- **supports-color**: Utility for detecting color support in the current terminal.
- **vt100**: Library for parsing and emulating VT100 terminal escape sequences.
- **textwrap**: Library for word-wrapping, indenting, and formatting text for the terminal.
- **syntect**: Library for high-quality syntax highlighting using Sublime Text grammars.
- **shell-words**: Utility for parsing and joining shell-style quoted strings.
- **shlex**: Shell-like lexer for splitting strings into tokens like a shell would.
- **arboard**: Cross-platform library for accessing the system clipboard.
- **browser**: Simple utility for opening URLs in the default web browser.
- **webbrowser**: Cross-platform library for opening web pages in a browser.
- **emojis**: Look up and use GitHub-style emojis in terminal output.

## Security & Crypto
- **age**: Simple, modern, and secure tool for file encryption.
- **base64**: Fast and efficient base64 encoding and decoding.
- **constant_time_eq**: Utility for constant-time equality checks to prevent timing attacks.
- **crypto_box**: Public-key authenticated encryption using modern cryptographic primitives.
- **ed25519-dalek**: Fast and secure Ed25519 digital signature implementation.
- **hmac**: Generic implementation of the Hash-based Message Authentication Code.
- **jsonwebtoken**: Implementation of JSON Web Tokens (JWT) for authentication.
- **p256**: Pure Rust implementation of the NIST P-256 elliptic curve.
- **sha1**: Implementation of the SHA-1 cryptographic hash function.
- **sha2**: Implementation of the SHA-2 family of cryptographic hash functions.
- **zeroize**: Securely zero out memory to protect sensitive information.
- **rustls**: Modern and safe implementation of the TLS protocol in Rust.
- **rustls-native-certs**: Use the system's native root certificates with the rustls library.
- **rustls-pki-types**: Common types used throughout the rustls PKI ecosystem.
- **openssl**: High-level bindings for the OpenSSL cryptographic library.
- **openssl-sys**: Low-level FFI bindings to the OpenSSL library.
- **keyring**: Cross-platform library for managing credentials in the system keyring.
- **landlock**: Sandboxing library for Linux using the Landlock security module.
- **seccompiler**: High-level compiler for creating seccomp-bpf filters on Linux.

## Serialization & Data Formats
- **serde**: Framework for efficient and generic serialization and deserialization.
- **serde_json**: JSON support for the Serde serialization framework.
- **serde_path_to_error**: Utility for finding the exact path to a Serde error.
- **serde_with**: Collection of custom serialization and deserialization helpers for Serde.
- **serde_yaml**: YAML support for the Serde serialization framework.
- **pulldown-cmark**: Fast and compliant CommonMark (Markdown) parsing library.
- **utoipa**: Compile-time auto-generated OpenAPI documentation for Rust.
- **toml**: Library for serializing and deserializing the TOML configuration format.
- **toml_edit**: Format-preserving editor for TOML files.
- **csv**: Fast and flexible CSV parsing and writing library.
- **quick-xml**: Fast XML pull-based parser and writer.
- **rmp-serde**: MessagePack support for the Serde serialization framework.
- **schemars**: Generate JSON Schema definitions from Rust data structures.
- **ts-rs**: Generate TypeScript type definitions from Rust types.

## Database & Storage
- **rusqlite**: Ergonomic and safe bindings for the SQLite database engine.
- **rusqlite_migration**: Simple migration management tool for rusqlite-based applications.
- **sqlx**: Modern, async, and type-safe SQL client with support for SQLite.
- **deadpool-sqlite**: Connection pool for SQLite databases using the Deadpool manager.

## Testing & Benchmarking
- **assert_cmd**: Tool for asserting the behavior of command-line applications in tests.
- **assert_matches**: Macro for asserting that an expression matches a given pattern.
- **criterion**: Statistics-driven benchmarking library for precise performance measurements.
- **insta**: Snapshot testing library for verifying complex data structures.
- **mockall**: Comprehensive and easy-to-use mocking library for Rust.
- **mockito**: HTTP mocking library for testing external API interactions.
- **wiremock**: HTTP mocking and stubbing library for integration testing.
- **proptest**: Property-based testing library inspired by Hypothesis and QuickCheck.
- **rstest**: Fixture-based testing framework for organizing complex test suites.
- **test-case**: Macro for creating parameterized tests with multiple inputs.
- **serial_test**: Utility for ensuring that specific tests run sequentially.
- **tracing-test**: Utilities for asserting and verifying tracing spans in tests.
- **tokio-test**: Collection of utilities for testing asynchronous code with Tokio.
- **pretty_assertions**: Enhanced assertion macros with colored diffs for easier debugging.
- **predicates**: Library for composing boolean predicates to use in assertions.

## Observability
- **log**: Lightweight and standard logging facade for Rust.
- **env_logger**: Simple logger that is configured via environment variables.
- **tracing**: Framework for instrumenting programs with structured, event-based data.
- **tracing-appender**: Utilities for writing tracing data to files and other outputs.
- **tracing-futures**: Integration for using tracing with asynchronous futures.
- **tracing-opentelemetry**: Integration for exporting tracing data to OpenTelemetry.
- **tracing-subscriber**: Utilities for collecting and processing tracing data.
- **opentelemetry**: API and SDK for instrumenting applications with OpenTelemetry.
- **opentelemetry_sdk**: Reference implementation of the OpenTelemetry SDK.
- **opentelemetry-appender-tracing**: Integration for bridging tracing data into OpenTelemetry.
- **opentelemetry-otlp**: Exporter for the OpenTelemetry Protocol (OTLP).
- **opentelemetry-semantic-conventions**: Standardized semantic conventions for OpenTelemetry data.
- **opentelemetry-stdout**: Simple OpenTelemetry exporter that prints to standard output.
- **sentry**: Official SDK for integrating with the Sentry error reporting platform.
- **pprof**: CPU and memory profiling tool for Rust applications.
- **profiling**: Common interface for multiple different profiling backends.
- **allocative**: Lightweight memory profiler for object traversal and size introspection.

## System, FFI & Low-level
- **libc**: Raw FFI bindings to the standard C library on various platforms.
- **winapi**: Comprehensive raw bindings to the Windows API.
- **winapi-util**: Safe wrappers and utilities for common Windows API tasks.
- **winresource**: Tool for embedding Windows resources like icons and version info.
- **uds_windows**: Implementation of Unix Domain Sockets for Windows systems.
- **v8**: High-performance Rust bindings for the V8 JavaScript engine.
- **runfiles**: Utility for accessing Bazel-managed runfiles in a portable way.
- **ctor**: Macro for defining module initialization and finalization functions.
- **tree-sitter**: Incremental parsing library for building fast syntax trees.
- **tree-sitter-bash**: Tree-sitter grammar for the Bash shell.
- **tree-sitter-go**: Tree-sitter grammar for the Go programming language.
- **tree-sitter-java**: Tree-sitter grammar for the Java programming language.
- **tree-sitter-javascript**: Tree-sitter grammar for JavaScript and related technologies.
- **tree-sitter-kotlin-ng**: Tree-sitter grammar for the Kotlin programming language.
- **tree-sitter-python**: Tree-sitter grammar for the Python programming language.
- **tree-sitter-ruby**: Tree-sitter grammar for the Ruby programming language.
- **tree-sitter-rust**: Tree-sitter grammar for the Rust programming language.
- **tree-sitter-swift**: Tree-sitter grammar for the Swift programming language.
- **tree-sitter-typescript**: Tree-sitter grammar for the TypeScript programming language.
- **portable-pty**: Cross-platform library for working with pseudo-terminals (PTYs).
- **starlark**: Rust implementation of the Starlark configuration language.

## Utilities & General Purpose
- **anyhow**: Flexible and ergonomic error handling library for applications.
- **bytes**: Utilities for working with bytes and byte buffers.
- **config**: Layered configuration system for Rust applications.
- **dotenvy**: Library for loading environment variables from .env files.
- **env-flags**: Utility for defining flags that can be overridden by environment variables.
- **env-lock**: Utility for locking environment variables to prevent accidental modification.
- **fs2**: Extensions to the standard library's filesystem types for file locking.
- **notify**: Cross-platform file system notification library.
- **thiserror**: Derive macro for defining custom error types with minimal boilerplate.
- **color-eyre**: Error reporting library with colored and structured output.
- **chrono**: Comprehensive library for working with dates, times, and time zones.
- **time**: Modern and type-safe date and time library.
- **iana-time-zone**: Utility for detecting the system's IANA time zone.
- **dirs**: Library for locating platform-specific standard directories.
- **etcetera**: Library for finding standard configuration and data directories.
- **path-absolutize**: Tool for resolving relative paths into absolute paths.
- **pathdiff**: Utility for calculating the relative path between two paths.
- **walkdir**: Library for recursively traversing directory structures.
- **glob**: Library for matching file paths against glob patterns.
- **globset**: Optimized library for matching multiple glob patterns simultaneously.
- **ignore**: Fast directory traversal that respects .gitignore and other ignore files.
- **derive_more**: Macros for deriving common trait implementations automatically.
- **strum**: Collection of macros for working with enums more easily.
- **strum_macros**: Procedural macros for the strum library.
- **indexmap**: Hash map that preserves the insertion order of its elements.
- **itertools**: Collection of additional iterator adapters and methods.
- **lru**: Fast implementation of a Least Recently Used (LRU) cache.
- **multimap**: Multi-map implementation that allows multiple values per key.
- **maplit**: Macros for creating collection literals like maps and sets.
- **regex**: Powerful and efficient regular expression engine.
- **regex-lite**: Compact and fast regular expression engine with a smaller footprint.
- **uuid**: Support for generating and parsing Universally Unique Identifiers (UUIDs).
- **rand**: Comprehensive library for random number generation and sampling.
- **tempfile**: Secure library for creating temporary files and directories.
- **which**: Utility for locating an executable file on the system's PATH.
- **whoami**: Retrieve information about the current user and host system.
- **gethostname**: Simple utility for getting the system's hostname.
- **os_info**: Library for detecting the operating system and its version.
- **sys-locale**: Utility for detecting the current system locale.
- **indoc**: Macro for creating unindented multi-line string literals.
- **unicode-segmentation**: Split strings into grapheme clusters, words, and sentences.
- **unicode-width**: Determine the displayed width of characters in terminal columns.
- **chardetng**: High-performance character encoding detector.
- **encoding_rs**: Robust and fast character encoding conversion library.
- **icu_decimal**: ICU4X library for formatting decimal numbers internationally.
- **icu_locale_core**: ICU4X library for managing and parsing locales.
- **icu_provider**: Core traits and types for providing ICU4X data.
- **flate2**: DEFLATE-based compression and decompression library.
- **tar**: Library for reading and writing Tar archive files.
- **zip**: Library for reading and writing Zip archive files.
- **zstd**: Safe bindings for the Zstandard compression algorithm.
- **dunce**: Drop-in replacement for canonicalize that handles Windows UNC paths.
- **include_dir**: Embed an entire directory of files into your Rust binary.
- **inventory**: Typed, distributed, compile-time plugin registry system.
- **cron**: Library for parsing cron expressions and generating schedules.
- **semver**: Implementation of the Semantic Versioning (SemVer) specification.
- **similar**: Versatile diffing library for text and arbitrary data.
- **diffy**: Library for generating and applying text diffs and patches.
- **wildmatch**: Pattern matching utility for strings using wildcards like * and ?.

## Media (Audio, Image, PDF)
- **hound**: Simple library for reading and writing WAV audio files.
- **image**: Versatile imaging library for reading, writing, and processing images.
- **qrcode**: Library for generating QR codes in various formats.
- **opusic-sys**: Low-level bindings to the Opus audio codec library.
- **symphonia**: Comprehensive multimedia demuxing and decoding framework.
- **symphonia-adapter-libopus**: Integration for using libopus with the Symphonia framework.
- **rodio**: High-level audio playback and management library.
- **rubato**: High-quality audio sample rate conversion library.
- **pdf-extract**: Library for extracting text and metadata from PDF files.

## External API Integrations
- **aws-config**: Standard configuration loader for the AWS SDK.
- **aws-credential-types**: Common types for AWS credentials and identity.
- **aws-sigv4**: Implementation of the AWS Signature Version 4 signing process.
- **aws-types**: Fundamental types and traits used throughout the AWS SDK.
- **serenity**: Feature-rich library for interacting with the Discord API.
- **slack-morphism**: Asynchronous and type-safe client for the Slack API.
- **teloxide**: Elegant and functional framework for building Telegram bots.
- **trello**: Client library for interacting with the Trello API.
- **whatsapp-rust**: SDK for integrating with the WhatsApp Business Platform.
- **whatsapp-rust-tokio-transport**: Tokio-based networking transport for whatsapp-rust.
- **whatsapp-rust-ureq-http-client**: ureq-based HTTP client for whatsapp-rust.
- **gix**: High-level and feature-complete implementation of the Git version control system.
- **discord**: (Internal or specific) Library for Discord integration.
- **slack**: (Internal or specific) Library for Slack integration.
- **telegram**: (Internal or specific) Library for Telegram integration.
- **whatsapp**: (Internal or specific) Library for WhatsApp integration.

## Internal & Project-Specific
- **codex-***: Suite of internal crates forming the Codex AI agent platform.
- **rmcp**: Implementation of the Remote Model Context Protocol.
- **sacp**: SDK for the Symposium Agent Client Protocol.
- **sacp-derive**: Procedural macros for the Symposium Agent Client Protocol.
- **wacore**: Core runtime and integration components for WebAssembly.
- **wacore-binary**: Binary utilities and runtime components for WebAssembly.
- **waproto**: Protocol and schema definitions for the Wacore ecosystem.
- **agent-client-protocol-schema**: Standardized schema for AI agent and client communication.
- **app_test_support**: Internal testing utilities and helpers for applications.
- **core_test_support**: Internal testing utilities and helpers for core components.
- **mcp_test_support**: Testing infrastructure for the Model Context Protocol (MCP).
- **crabrace**: Internal tool for detecting and debugging race conditions.
- **chromey**: Internal library for controlling Chrome via the DevTools Protocol.
