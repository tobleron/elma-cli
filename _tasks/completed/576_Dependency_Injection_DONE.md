# 576 — Add Deterministic Dependency Injection for Testable Components

- **Priority**: Medium
- **Category**: Testing
- **Depends on**: 005/554 (session-scoped state), 003/552 (split tool_calling.rs)
- **Blocks**: 563

## Problem Statement

Many components have hard-coded dependencies that prevent unit testing:

1. **HTTP client**: `reqwest::Client` is created inline, preventing mock injection
2. **File system**: `std::fs` calls are scattered throughout, with no abstraction
3. **Time**: `SystemTime::now()` and `Instant::now()` are called inline, making time-dependent tests flaky
4. **Environment**: `std::env::current_dir()` is called inline
5. **Global statics**: Components access global mutable state (Task 554)

This makes it impossible to write deterministic unit tests for:
- Tool executors (depend on filesystem)
- Tool loop (depends on HTTP client and time)
- Stop policy (depends on time for wall clock checks)
- Auto compaction (depends on token counting and HTTP)

## Why This Matters for Small Local LLMs

Testing with small models requires running many iterations of the same test with different model outputs. Without testable components, every test requires a full end-to-end setup with a real or mock LLM backend.

## Recommended Target Behavior

Define traits for injectable dependencies:

```rust
// Filesystem abstraction
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn write(&self, path: &Path, content: &str) -> io::Result<()>;
    fn metadata(&self, path: &Path) -> io::Result<Metadata>;
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;
}

// Time abstraction
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
    fn elapsed(&self, start: Instant) -> Duration;
}

// HTTP abstraction (already partially available via reqwest::Client)
pub trait LlmClient: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, req: ChatRequest) -> Result<Stream<...>>;
}

// Real implementations (used in production)
pub struct RealFileSystem;
pub struct RealClock;
pub struct RealLlmClient { inner: reqwest::Client }
```

Components accept these traits via generics or trait objects:

```rust
// Before
fn exec_read(av: &Value, workdir: &PathBuf, ...) -> ToolExecutionResult { ... }

// After 
fn exec_read<F: FileSystem>(fs: &F, av: &Value, workdir: &PathBuf, ...) -> ToolExecutionResult { ... }
```

## Source Files That Need Modification

- All tool executors — Accept `FileSystem` trait
- `tool_loop.rs` — Accept `Clock` and `LlmClient` traits
- `stop_policy.rs` — Accept `Clock` trait
- `auto_compact.rs` — Accept `LlmClient` and token counter traits

## New Files/Modules

- `src/abstractions.rs` — `FileSystem`, `Clock`, `LlmClient` traits + real implementations
- `src/abstractions_test.rs` — Mock implementations for testing

## Step-by-Step Implementation Plan

1. Create abstraction traits in `src/abstractions.rs`
2. Implement real versions (wrapping `std::fs`, `SystemTime`, `reqwest::Client`)
3. Implement mock versions for testing
4. Add traits as generic parameters to one component at a time (start with simplest)
5. Write unit tests using mock implementations
6. Ensure zero-cost abstraction in release builds (generic params compiled away)

## Recommended Crates

- `async-trait` — already a dependency; for async traits

## Acceptance Criteria

- At least 5 components are testable with mock dependencies
- Real implementations have zero overhead in release builds
- Mock implementations enable deterministic testing
- No global state access in abstracted components

## Risks and Migration Notes

- **Generic proliferation**: Adding generic params to many functions can make signatures complex. Consider using `dyn Trait` with `Arc` for less performance-sensitive paths.
- **Compile time**: Generic abstractions may increase compile time. Monitor with `cargo build --timings`.
- Start with `FileSystem` trait (biggest impact), then `Clock`, then `LlmClient`.
